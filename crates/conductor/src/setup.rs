//! `conductor setup` (`story:setup-command`): checks one repository's prerequisites, and with
//! `--apply` applies the fixes that are safe to repeat.
//!
//! It prints doctor's rows, computed by doctor's own checks ([`doctor::check_at`], with the pins
//! of the repository's `b10x.toml`), then four rows of the repository ([`checks`]):
//! - `store`: the planning store's version, `version` in `.engineering/project.yaml`: current
//!   ([`CURRENT_STORE`]), old with the command that migrates it ([`MIGRATE`]), or none, which is
//!   no failure: conductor reads a store only where there is one;
//! - `instance`: whether an instance of the config file names the repository, its checkouts root
//!   or a `local` source being the directory that holds it, or the one above that; when none
//!   does, the source `--apply` would write for the repository's `origin` ([`Origin`]);
//! - `records`: whether that instance's records directory holds what `conductor init` writes;
//! - `trust`: whether `~/.claude.json` marks the repository a trusted Claude Code workspace.
//!
//! The last line is one verdict, doctor's ([`doctor::verdict_of`]) over doctor's failures and then
//! each failing row, `store (old)`, `instance (none)`, `records (missing)` and
//! `trust (not trusted)`; it exits 1 unless it is `ready`.
//!
//! `--apply` writes a config file with one instance naming the repository when there is no config
//! file (with one, it prints the instance to add and changes nothing), then runs `conductor init`
//! and `conductor trust` for the instance naming the repository when its records or its trust are
//! missing. It never logs a CLI in, installs a program or migrates a store: it prints each as a
//! `to install: ` or `to run: ` line. Everything it checks is read again after it applied.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use conductor_model::config::{
    ForgeLoginState, Instance, PrerequisiteState, RepositoryCheck, SetupReport, Source,
};
use serde_json::Value;

use crate::cli::{InitArgs, SetupArgs, TrustArgs};
use crate::collect;
use crate::config::{self, Environment, Loaded};
use crate::doctor::{self, Doctor};
use crate::trust::CLAUDE_JSON;

/// The planning store version a current `aep` writes.
pub const CURRENT_STORE: &str = "aep.project/5";

/// The planning store's project file, under the repository.
const PROJECT_FILE: &str = ".engineering/project.yaml";

/// The command that shows what migrating an old planning store would change; only the person
/// approves the migration itself.
pub const MIGRATE: &str = "aep plan store migrate git --dry-run";

/// A file `conductor init` writes, whose presence says the records exist.
const RECORDS_MARK: &str = "NORTHSTAR.md";

/// How long `git remote get-url origin` may take.
const GIT_BOUND: Duration = Duration::from_secs(10);

/// The forge a repository's `origin` is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// `github.com`, under an owner.
    GitHub {
        /// The owner, the first part of the path.
        owner: String,
    },
    /// Any other host, read as a GitLab host, under a group.
    GitLab {
        /// The host, with an `https` URL's port.
        host: String,
        /// The group: the path but its last part.
        group: String,
    },
    /// No `origin`, or one that names no host and path.
    None,
}

/// The forge of the remote URL `url`: `https://host[:port]/path`, `ssh://[user@]host[:port]/path`
/// or `[user@]host:path`. `None` for a URL that names no host or a path of fewer than two parts.
#[must_use]
pub fn origin_of(url: &str) -> Origin {
    let url = url.trim();
    let (host, path) = match url.split_once("://") {
        Some((scheme, rest)) => {
            let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
            let host = authority.rsplit('@').next().unwrap_or(authority);
            // A port is the forge's for a web URL, and only the transport's for ssh.
            let host = if scheme.starts_with("http") {
                host
            } else {
                host.split(':').next().unwrap_or(host)
            };
            (host, path)
        }
        None => match url.split_once(':') {
            Some((authority, path)) if !authority.contains('/') => {
                (authority.rsplit('@').next().unwrap_or(authority), path)
            }
            _ => return Origin::None,
        },
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    if host.is_empty() || parts.len() < 2 {
        return Origin::None;
    }
    if host == doctor::GITHUB_HOST {
        Origin::GitHub {
            owner: parts[0].to_owned(),
        }
    } else {
        Origin::GitLab {
            host: host.to_owned(),
            group: parts[..parts.len() - 1].join("/"),
        }
    }
}

/// The forge of `repository`'s `origin` remote; [`Origin::None`] when it has none or `git` does
/// not answer.
fn origin(repository: &Path) -> Origin {
    let answered = collect::run(
        Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(["remote", "get-url", "origin"]),
        GIT_BOUND,
    );
    match answered {
        Ok(output) if output.status.success() => {
            origin_of(&String::from_utf8_lossy(&output.stdout))
        }
        _ => Origin::None,
    }
}

/// The source an instance naming a repository of `origin` takes, as a flow mapping: `parent` is
/// the directory that holds the repository, a `local` source's path.
#[must_use]
pub fn source_text(origin: &Origin, parent: &Path) -> String {
    match origin {
        Origin::GitHub { owner } => format!("{{github: {owner}}}"),
        Origin::GitLab { host, group } => format!("{{gitlab: {group}, host: {host}}}"),
        Origin::None => format!("{{local: {}}}", parent.display()),
    }
}

/// The instance name for the repository directory `name`: its letters, digits, `.`, `_` and `-`,
/// any other character as `-`, starting with a letter or digit.
#[must_use]
pub fn instance_name(name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    match name.chars().next() {
        Some(c) if c.is_ascii_alphanumeric() => name,
        Some(_) => format!("repository{name}"),
        None => "repository".to_owned(),
    }
}

/// A config file of one instance `name` naming the repository held by `parent`, with the source
/// of `origin`.
#[must_use]
pub fn config_text(name: &str, origin: &Origin, parent: &Path) -> String {
    let quoted = |path: &Path| {
        serde_json::to_string(&path.display().to_string()).unwrap_or_else(|_| "\"\"".to_owned())
    };
    let source = match origin {
        Origin::GitHub { owner } => format!("      - github: {owner}\n"),
        Origin::GitLab { host, group } => {
            format!("      - gitlab: {group}\n        host: {host}\n")
        }
        Origin::None => format!("      - local: {}\n", quoted(parent)),
    };
    format!(
        "version: {version}\n\
         instances:\n\
         \x20 - name: {name}\n\
         \x20   sources:\n\
         {source}\
         \x20   checkouts:\n\
         \x20     root: {root}\n\
         \x20     trees: ~/.local/state/worktree/trees/{name}\n",
        version = config::VERSION,
        root = quoted(parent),
    )
}

/// `version` in a planning store's project file, YAML or JSON; none when it names none.
fn store_version(text: &str) -> Option<String> {
    if let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(text) {
        return fields
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_owned);
    }
    text.lines()
        .find_map(|line| line.strip_prefix("version:"))
        .map(|value| {
            value
                .split(" #")
                .next()
                .unwrap_or_default()
                .trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .to_owned()
        })
        .filter(|version| !version.is_empty())
}

/// The `store` row of `repository`.
fn store_check(repository: &Path) -> RepositoryCheck {
    let file = repository.join(PROJECT_FILE);
    let (ready, detail, fix) = match fs::read_to_string(&file) {
        Err(error) if error.kind() == ErrorKind::NotFound => (true, "none".to_owned(), None),
        Err(error) => (
            false,
            format!("old (unread: {error})"),
            Some(MIGRATE.to_owned()),
        ),
        Ok(text) => match store_version(&text) {
            Some(version) if version == CURRENT_STORE => {
                (true, format!("current ({version})"), None)
            }
            Some(version) => (false, format!("old ({version})"), Some(MIGRATE.to_owned())),
            None => (
                false,
                "old (no version)".to_owned(),
                Some(MIGRATE.to_owned()),
            ),
        },
    };
    RepositoryCheck {
        check: "store".to_owned(),
        ready,
        detail,
        fix,
    }
}

/// Whether `instance` names `repository`, a physical path: its checkouts root, or the path of one
/// of its `local` sources, is the directory that holds the repository or the one above that.
fn names(instance: &Instance, repository: &Path) -> bool {
    let roots = std::iter::once(instance.checkouts.root.as_str()).chain(
        instance.sources.iter().filter_map(|source| match source {
            Source::Local(local) => Some(local.path.as_str()),
            _ => None,
        }),
    );
    let above: Vec<&Path> = repository.ancestors().skip(1).take(2).collect();
    roots
        .filter_map(|root| fs::canonicalize(root).ok())
        .any(|root| above.contains(&root.as_path()))
}

/// The instance of the config file `loaded` that names `repository`, the first in the file's
/// order; none without a config file.
fn naming<'l>(loaded: &'l Loaded, repository: &Path) -> Option<&'l Instance> {
    if !loaded.read {
        return None;
    }
    loaded
        .config
        .instances
        .iter()
        .find(|instance| names(instance, repository))
}

/// Whether `~/.claude.json` under `home` marks `repository` trusted.
fn trusted(home: Option<&Path>, repository: &Path) -> bool {
    let Some(home) = home else {
        return false;
    };
    let Ok(text) = fs::read_to_string(home.join(CLAUDE_JSON)) else {
        return false;
    };
    let Ok(document) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    document["projects"][repository.display().to_string()]["hasTrustDialogAccepted"]
        == Value::Bool(true)
}

/// The four rows of `repository` ([`setup`]), for the config `flag` and `environment`.
#[must_use]
pub fn checks(
    flag: Option<&Path>,
    environment: &Environment,
    repository: &Path,
) -> Vec<RepositoryCheck> {
    let shown = repository.display();
    let loaded = config::load(flag, environment);
    let instance = loaded
        .as_ref()
        .ok()
        .and_then(|loaded| naming(loaded, repository));
    let parent = repository.parent().unwrap_or(repository);
    let instance_check = match (&loaded, instance) {
        (Err(_), _) => RepositoryCheck {
            check: "instance".to_owned(),
            ready: false,
            detail: "unread (the config does not load)".to_owned(),
            fix: Some("conductor config validate".to_owned()),
        },
        (Ok(_), Some(instance)) => RepositoryCheck {
            check: "instance".to_owned(),
            ready: true,
            detail: format!("{} names {shown}", instance.name.0),
            fix: None,
        },
        (Ok(_), None) => RepositoryCheck {
            check: "instance".to_owned(),
            ready: false,
            detail: format!(
                "none names {shown}, which --apply names with the source {}",
                source_text(&origin(repository), parent)
            ),
            fix: Some("conductor setup --apply".to_owned()),
        },
    };
    let records_check = match instance {
        Some(instance) => {
            let records = Path::new(&instance.records);
            let present = records.join(RECORDS_MARK).is_file();
            RepositoryCheck {
                check: "records".to_owned(),
                ready: present,
                detail: format!(
                    "{} ({})",
                    if present { "present" } else { "missing" },
                    records.display()
                ),
                fix: (!present).then(|| format!("conductor init --instance {}", instance.name.0)),
            }
        }
        None => RepositoryCheck {
            check: "records".to_owned(),
            ready: false,
            detail: "missing (no instance names the repository)".to_owned(),
            fix: None,
        },
    };
    let is_trusted = trusted(environment.home.as_deref(), repository);
    let trust_check = RepositoryCheck {
        check: "trust".to_owned(),
        ready: is_trusted,
        detail: format!(
            "{} ({shown})",
            if is_trusted { "trusted" } else { "not trusted" }
        ),
        fix: match instance {
            Some(instance) if !is_trusted => {
                Some(format!("conductor trust --instance {}", instance.name.0))
            }
            _ => None,
        },
    };
    vec![
        store_check(repository),
        instance_check,
        records_check,
        trust_check,
    ]
}

/// The line `conductor setup` prints for `check`: `<check>: <detail>`, and `; run <fix>` when a
/// command fixes it.
#[must_use]
pub fn check_line(check: &RepositoryCheck) -> String {
    match &check.fix {
        Some(fix) => format!("{}: {}; run {fix}", check.check, check.detail),
        None => format!("{}: {}", check.check, check.detail),
    }
}

/// The failure a row that is not ready adds to the verdict: `store (old)`, `instance (none)`,
/// `records (missing)` or `trust (not trusted)`.
fn failure(check: &RepositoryCheck) -> Option<String> {
    if check.ready {
        return None;
    }
    let what = match check.check.as_str() {
        "store" => "old",
        "instance" => "none",
        "records" => "missing",
        "trust" => "not trusted",
        _ => "not ready",
    };
    Some(format!("{} ({what})", check.check))
}

/// What `--apply` leaves to a person, as the command to run: `to install: <program>` for each
/// program missing or older than its pin, `to run: <cli> auth login --hostname <host>` for each
/// forge CLI not logged in, and the store's migration.
fn left_to_a_person(doctor: &Doctor, checks: &[RepositoryCheck]) -> Vec<String> {
    let installs = doctor
        .rows
        .iter()
        .filter(|row| {
            matches!(
                row.state,
                PrerequisiteState::Missing | PrerequisiteState::OlderThanPin
            )
        })
        .map(|row| format!("to install: {}", row.program));
    let logins = doctor
        .logins
        .iter()
        .filter(|login| login.state == ForgeLoginState::NotLoggedIn)
        .map(|login| {
            format!(
                "to run: {} auth login --hostname {}",
                login.program, login.host
            )
        });
    let migrate = checks
        .iter()
        .filter(|check| check.check == "store" && !check.ready)
        .map(|_| format!("to run: {MIGRATE}"));
    installs.chain(logins).chain(migrate).collect()
}

/// `--apply`: writes the config file naming `repository` when there is none, then runs
/// `conductor init` and `conductor trust` for the instance naming it when its records or its
/// trust are missing. A step that fails is named on standard error, and the rows read afterwards
/// say what is still missing.
///
/// # Errors
///
/// The config file cannot be written.
fn apply(flag: Option<&Path>, environment: &Environment, repository: &Path) -> Result<()> {
    const WHAT: &str = "setup";
    let named = config::load(flag, environment)
        .ok()
        .and_then(|loaded| naming(&loaded, repository).map(|instance| instance.name.0.clone()));
    let name = match named {
        Some(name) => name,
        None => {
            let (file, _) = config::locate(flag, environment).context(WHAT)?;
            let parent = repository.parent().unwrap_or(repository);
            let name = instance_name(
                &repository
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
            let text = config_text(&name, &origin(repository), parent);
            if file.exists() {
                println!(
                    "to write: {} names no instance holding {}; add this instance under its \
                     instances:",
                    file.display(),
                    repository.display()
                );
                for line in text.lines().skip(2) {
                    println!("to write: {line}");
                }
                return Ok(());
            }
            if let Some(dir) = file.parent() {
                fs::create_dir_all(dir)
                    .with_context(|| format!("{WHAT}: create {}", dir.display()))?;
            }
            fs::write(&file, text).with_context(|| format!("{WHAT}: write {}", file.display()))?;
            println!("wrote: {}", file.display());
            name
        }
    };
    let rows = checks(flag, environment, repository);
    let missing = |check: &str| rows.iter().any(|row| row.check == check && !row.ready);
    if missing("records") {
        let args = InitArgs {
            instance: Some(name.clone()),
        };
        if let Err(error) = crate::init::init(flag, &args) {
            eprintln!("conductor: setup: {error:#}");
        }
    }
    if missing("trust") {
        let args = TrustArgs {
            instance: Some(name),
        };
        if let Err(error) = crate::trust::trust(flag, &args) {
            eprintln!("conductor: setup: {error:#}");
        }
    }
    Ok(())
}

/// `conductor setup`: with `--apply`, [`apply`] first; then doctor's lines, the four rows of the
/// repository, with `--apply` what it leaves to a person, and last the verdict over both. Exits 0
/// when the verdict is `ready`, 1 otherwise. Writes nothing without `--apply`.
///
/// # Errors
///
/// The repository is not a directory, what [`doctor::check_at`] answers, or `--apply` cannot
/// write the config file.
pub fn setup(flag: Option<&Path>, args: &SetupArgs) -> Result<ExitCode> {
    const WHAT: &str = "setup";
    let environment = Environment::from_process().context(WHAT)?;
    let asked = args
        .repository
        .clone()
        .unwrap_or_else(|| environment.cwd.clone());
    let repository: PathBuf = match fs::canonicalize(&asked) {
        Ok(path) if path.is_dir() => path,
        _ => bail!(
            "{WHAT}: the repository {} is not a directory",
            asked.display()
        ),
    };
    let applying = args.apply.unwrap_or(false);
    if applying {
        apply(flag, &environment, &repository)?;
    }
    let doctor = doctor::check_at(flag, &repository)?;
    let checks = checks(flag, &environment, &repository);
    for line in doctor.lines() {
        println!("{line}");
    }
    for check in &checks {
        println!("{}", check_line(check));
    }
    if applying {
        for line in left_to_a_person(&doctor, &checks) {
            println!("{line}");
        }
    }
    let report = report(&doctor, &repository, checks);
    println!("{}", report.verdict);
    Ok(if report.verdict == doctor::READY {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// The report the specification declares, `conductor.config.SetupReport`: its verdict is
/// doctor's over doctor's failures and then each row's.
#[must_use]
pub fn report(doctor: &Doctor, repository: &Path, checks: Vec<RepositoryCheck>) -> SetupReport {
    let failures: Vec<String> = doctor
        .failures()
        .into_iter()
        .chain(checks.iter().filter_map(failure))
        .collect();
    SetupReport {
        doctor: doctor.report(),
        repository: repository.display().to_string(),
        verdict: doctor::verdict_of(&failures),
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_origin_names_its_forge() {
        let github = Origin::GitHub {
            owner: "example-org".to_owned(),
        };
        for url in [
            "https://github.com/example-org/alpha.git",
            "git@github.com:example-org/alpha.git",
            "ssh://git@github.com/example-org/alpha",
        ] {
            assert_eq!(origin_of(url), github, "{url}");
        }
        let gitlab = |host: &str, group: &str| Origin::GitLab {
            host: host.to_owned(),
            group: group.to_owned(),
        };
        assert_eq!(
            origin_of("git@gitlab.example.com:example-group/sub/beta.git"),
            gitlab("gitlab.example.com", "example-group/sub")
        );
        assert_eq!(
            origin_of("https://gitlab.example.com:8443/example-group/beta"),
            gitlab("gitlab.example.com:8443", "example-group")
        );
        assert_eq!(
            origin_of("ssh://git@gitlab.example.com:2222/example-group/beta.git"),
            gitlab("gitlab.example.com", "example-group")
        );
        for url in ["", "/srv/repositories/alpha", "https://github.com/alpha"] {
            assert_eq!(origin_of(url), Origin::None, "{url:?}");
        }
    }

    #[test]
    fn an_instance_name_is_a_config_name() {
        assert_eq!(instance_name("alpha"), "alpha");
        assert_eq!(instance_name("my repo"), "my-repo");
        assert_eq!(instance_name(".hidden"), "repository.hidden");
        assert_eq!(instance_name(""), "repository");
    }

    #[test]
    fn a_written_config_parses_with_the_source_of_the_origin() {
        let home = Path::new("/srv/home");
        let parent = Path::new("/srv/home/src");
        for origin in [
            Origin::GitHub {
                owner: "example-org".to_owned(),
            },
            Origin::GitLab {
                host: "gitlab.example.com".to_owned(),
                group: "example-group".to_owned(),
            },
            Origin::None,
        ] {
            let text = config_text("alpha", &origin, parent);
            let parsed = config::parse(&text, home)
                .unwrap_or_else(|problems| panic!("{problems:?}:\n{text}"));
            let instance = &parsed.instances[0];
            assert_eq!(instance.name.0, "alpha");
            assert_eq!(instance.checkouts.root, "/srv/home/src");
            let matches = match (&origin, &instance.sources[0]) {
                (Origin::GitHub { owner }, Source::GitHub(source)) => source.owner == *owner,
                (Origin::GitLab { host, group }, Source::GitLab(source)) => {
                    source.host == *host && source.group == *group
                }
                (Origin::None, Source::Local(source)) => source.path == "/srv/home/src",
                _ => false,
            };
            assert!(matches, "{origin:?}:\n{text}");
        }
    }

    #[test]
    fn the_store_version_is_read_from_yaml_or_json() {
        assert_eq!(
            store_version("version: aep.project/5\n# A comment.\n").as_deref(),
            Some(CURRENT_STORE)
        );
        assert_eq!(
            store_version("{\"version\": \"aep.project/1\"}").as_deref(),
            Some("aep.project/1")
        );
        assert_eq!(store_version("protocol: adp/1\n"), None);
    }
}
