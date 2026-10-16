//! `conductor doctor` (`story:install-prerequisites`): each program conductor starts, present
//! with the first line its `--version` prints, or missing; and, when the nearest `b10x.toml`
//! pins one of them, whether it is older than that pin; for `aep`, `ess` or `worktree` that no
//! `b10x.toml` pins, whether it is older than the pin the binary was built with ([`build_pins`],
//! `story:doctor-minimum-versions`). One `conductor.config.Prerequisite` row per program. A
//! forge's CLI is needed only when an instance of the config file has a source on that forge
//! (`story:gh-optional`): `gh` for a `github` source, `glab` for a `gitlab` source; the other is
//! `not needed`. A config file that does not load makes the run not ready. The last line is the
//! verdict, `ready` or `not ready: ` and why (`story:doctor-verdict`).
//!
//! Before the program rows it prints the machine it runs on, as `uname` gives it, and its own
//! version ([`machine`]); after them, for each host of the config file's sources whose CLI is
//! present, the account that CLI is logged in as there, or `not logged in`, which makes the run
//! not ready (`story:doctor-identity`). Only the login name is printed: never a token or an
//! e-mail address the CLI prints beside it ([`account_of`]).
//!
//! Under the row of each program missing or older than its pin it prints, indented two spaces,
//! `install: ` and the command that installs it on this machine (`story:install-hints`): the
//! package of the package manager found on the `PATH` it searches ([`manager`]) for the system
//! `uname -s` names, else the program's own install page ([`install_command`]).
//!
//! After the logins, for each instance's checkouts root, whether it is inside an activated
//! `worktree` workspace, as `worktree workspace: <root> activated` or `… not activated`, the
//! latter with `  activate: ` and the command that activates it ([`activate_command`]), which
//! doctor never runs; a root not activated makes the run not ready
//! (`story:worktree-activation`, [`activation_of`]).
//!
//! After the workspaces, for each `usage.watcher` of the config file's instances, the systemd user
//! unit that rotates the harness accounts, whether it runs, as `systemctl --user is-active` on the
//! `PATH` answers: `usage watcher: <unit> running`, or `… not running (<state>)`
//! (`story:usage-probe`, [`watcher_line`]). A watcher not running is reported, not a failure:
//! the rotation is the operator's own tool.
//!
//! [`check`] computes every row, [`Doctor::failures`] what is not ready and [`verdict_of`] the
//! last line, so `conductor setup` prints the same rows and adds its own failures to the same
//! verdict (`story:setup-command`).
//!
//! [`find`] is the one check of whether a program is there: `conductor snapshot start-snapshot`
//! runs it on the programs its collectors start ([`crate::collect::programs`]) and refuses at
//! start, naming each one missing, rather than failing inside a collector.

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use conductor_model::config::{
    Config, DoctorReport, ForgeLogin, ForgeLoginState, Instance, Machine, Prerequisite,
    PrerequisiteState, ProviderAvailability, Source,
};

use crate::collect;
use crate::config;

/// Every program conductor starts: its collectors, its gate and its tasks. A forge's CLI of
/// [`FORGES`] is among them only for an instance with a source on that forge ([`forges`]).
pub const PROGRAMS: [&str; 9] = [
    "git", "gh", "glab", "claude", "jq", "task", "aep", "worktree", "ess",
];

/// The forges' CLIs: `gh` for GitHub, `glab` for GitLab.
pub const FORGES: [&str; 2] = ["gh", "glab"];

/// The pin file the ecosystem installer's `pin` command writes, found from the working
/// directory upward.
pub const PIN_FILE: &str = "b10x.toml";

/// The pins this binary was built with (`story:doctor-minimum-versions`): `name=spec` pairs
/// separated by `,`, the `aep`, `ess` and `worktree` pins of the product repository's own
/// `b10x.toml` at build time (`build.rs`); empty when the build found no such file.
const BUILD_PINS: &str = env!("CONDUCTOR_BUILD_PINS");

/// [`BUILD_PINS`] as `(program, spec)` pairs, in their order: what `conductor doctor` compares
/// with for each of their programs that the `b10x.toml` it finds does not pin ([`merge_pins`]).
#[must_use]
pub fn build_pins() -> Vec<(String, String)> {
    BUILD_PINS
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, spec)| (name.to_owned(), spec.to_owned()))
        .collect()
}

/// The pins `conductor doctor` checks: each of `project` (a `b10x.toml`'s), in its order, then
/// each of `build` ([`build_pins`]) whose program `project` does not pin. The project's pin wins
/// for a program both pin; the build's stays a minimum for one the project leaves out.
#[must_use]
pub fn merge_pins(
    project: &[(String, String)],
    build: &[(String, String)],
) -> Vec<(String, String)> {
    project
        .iter()
        .cloned()
        .chain(
            build
                .iter()
                .filter(|(program, _)| !project.iter().any(|(name, _)| name == program))
                .cloned(),
        )
        .collect()
}

/// How long `<program> --version` may take.
const VERSION_BOUND: Duration = Duration::from_secs(10);

/// How long `<forge CLI> auth status --hostname <host>` may take: it may ask the host.
const AUTH_BOUND: Duration = Duration::from_secs(20);

/// The host of a `github` source.
pub const GITHUB_HOST: &str = "github.com";

/// What a machine row reads for a value `uname` does not give.
const UNKNOWN: &str = "unknown";

/// Where `uname` is looked for besides `PATH`, which may hold none (a test's `PATH` holds only
/// its fakes).
const UNAME: [&str; 2] = ["/usr/bin/uname", "/bin/uname"];

/// What `uname <flag>` prints, trimmed: `uname` from `PATH`, else from [`UNAME`]; [`UNKNOWN`]
/// when none runs or it prints nothing.
fn uname(flag: &str) -> String {
    let candidates = find("uname").into_iter().chain(
        UNAME
            .iter()
            .map(PathBuf::from)
            .filter(|path| path.is_file()),
    );
    for path in candidates {
        let Ok(output) = collect::run(Command::new(&path).arg(flag), VERSION_BOUND) else {
            continue;
        };
        let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if output.status.success() && !text.is_empty() {
            return text;
        }
    }
    UNKNOWN.to_owned()
}

/// The machine `conductor doctor` runs on (`story:doctor-identity`): `uname -n`, `-s`, `-r` and
/// `-m`, and this binary's version.
#[must_use]
pub fn machine() -> Machine {
    Machine {
        host: uname("-n"),
        system: uname("-s"),
        release: uname("-r"),
        architecture: uname("-m"),
        conductor: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// The lines `conductor doctor` prints for `machine`, before its program rows.
#[must_use]
pub fn machine_lines(machine: &Machine) -> Vec<String> {
    vec![
        format!("host: {}", machine.host),
        format!("system: {} {}", machine.system, machine.release),
        format!("architecture: {}", machine.architecture),
        format!("conductor {}", machine.conductor),
    ]
}

/// The hosts of `config`'s sources, across every instance, each with the forge CLI that logs in
/// to it, each pair once, in the order the file names them: [`GITHUB_HOST`] with `gh` for a
/// `github` source, the source's `host` with `glab` for a `gitlab` source.
#[must_use]
pub fn forge_hosts(config: &Config) -> Vec<(&'static str, String)> {
    let mut hosts: Vec<(&'static str, String)> = Vec::new();
    for source in config
        .instances
        .iter()
        .flat_map(|instance| &instance.sources)
    {
        let host = match source {
            Source::GitHub(_) => ("gh", GITHUB_HOST.to_owned()),
            Source::GitLab(gitlab) => ("glab", gitlab.host.clone()),
            Source::Local(_) => continue,
        };
        if !hosts.contains(&host) {
            hosts.push(host);
        }
    }
    hosts
}

/// The login name `text`, what `<forge CLI> auth status --hostname <host>` printed, gives for
/// `host`: the word after `Logged in to <host> account` (`gh`) or `Logged in to <host> as`
/// (`glab`, and an older `gh`), when it is a plain name of letters, digits, `-`, `_` and `.`.
/// Nothing else of `text` is answered, so no token or e-mail address the CLI prints beside it is.
#[must_use]
pub fn account_of(text: &str, host: &str) -> Option<String> {
    let marker = format!("Logged in to {host} ");
    text.lines().find_map(|line| {
        let (_, rest) = line.split_once(&marker)?;
        let rest = rest
            .strip_prefix("account ")
            .or_else(|| rest.strip_prefix("as "))?;
        let name = rest
            .split_whitespace()
            .next()?
            .trim_end_matches([',', ';', ':']);
        (!name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        .then(|| name.to_owned())
    })
}

/// Whether `program`, found at `path`, is logged in to `host` (`story:doctor-identity`): its
/// `auth status --hostname <host>` exits 0; and the account it names there ([`account_of`]).
fn login(program: &str, path: &Path, host: &str) -> ForgeLogin {
    let answered = collect::run(
        Command::new(path).args(["auth", "status", "--hostname", host]),
        AUTH_BOUND,
    );
    let (state, account) = match answered {
        Ok(output) if output.status.success() => {
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push('\n');
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            (ForgeLoginState::LoggedIn, account_of(&text, host))
        }
        _ => (ForgeLoginState::NotLoggedIn, None),
    };
    ForgeLogin {
        program: program.to_owned(),
        host: host.to_owned(),
        state,
        account,
    }
}

/// The line `conductor doctor` prints for `login`.
#[must_use]
pub fn login_line(login: &ForgeLogin) -> String {
    let ForgeLogin {
        program,
        host,
        state,
        account,
    } = login;
    match (state, account) {
        (ForgeLoginState::LoggedIn, Some(account)) => {
            format!("{program}: logged in to {host} as {account}")
        }
        (ForgeLoginState::LoggedIn, None) => format!("{program}: logged in to {host}"),
        (ForgeLoginState::NotLoggedIn, _) => format!("{program}: not logged in to {host}"),
    }
}

/// The forge CLIs of [`FORGES`] that `instance` starts (`story:gh-optional`): `gh` when it has a
/// `github` source, `glab` when it has a `gitlab` source.
#[must_use]
pub fn forges(instance: &Instance) -> Vec<&'static str> {
    FORGES
        .into_iter()
        .filter(|forge| {
            instance.sources.iter().any(|source| match source {
                Source::GitHub(_) => *forge == "gh",
                Source::GitLab(_) => *forge == "glab",
                Source::Local(_) => false,
            })
        })
        .collect()
}

/// Where `program` is on `PATH`: the first directory holding a regular file of that name that
/// may be run. `None` when there is none, or `PATH` is not set.
#[must_use]
pub fn find(program: &str) -> Option<PathBuf> {
    find_in(program, env::var_os("PATH").as_deref()?)
}

fn find_in(program: &str, path: &OsStr) -> Option<PathBuf> {
    env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(program))
        .find(|candidate| {
            fs::metadata(candidate)
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

/// The ecosystem's setup page, which installs `aep`, `ess` and `worktree` at a pin
/// (`story:install-hints`).
pub const SETUP_URL: &str =
    "https://github.com/beyond10x/agentplugins/releases/latest/download/SETUP.md";

/// The programs [`SETUP_URL`] installs.
const ECOSYSTEM: [&str; 3] = ["aep", "ess", "worktree"];

/// A package manager doctor names an install command of (`story:install-hints`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    /// Homebrew, `brew`.
    Brew,
    /// Debian's and Ubuntu's `apt-get`.
    AptGet,
    /// Fedora's `dnf`.
    Dnf,
    /// Arch's `pacman`.
    Pacman,
}

impl Manager {
    /// The program the manager is on `PATH` as.
    #[must_use]
    pub fn program(self) -> &'static str {
        match self {
            Self::Brew => "brew",
            Self::AptGet => "apt-get",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
        }
    }

    /// The command that installs a package, the package name after it.
    #[must_use]
    pub fn install(self) -> &'static str {
        match self {
            Self::Brew => "brew install",
            Self::AptGet => "sudo apt-get install",
            Self::Dnf => "sudo dnf install",
            Self::Pacman => "sudo pacman -S",
        }
    }
}

/// The package manager of a machine whose `uname -s` is `system`, looked for on `search`, a
/// `PATH` (none finds nothing): `brew` on macOS (`Darwin`); elsewhere the first of `apt-get`,
/// `dnf`, `pacman` and `brew` there is.
#[must_use]
pub fn manager(system: &str, search: Option<&OsStr>) -> Option<Manager> {
    let candidates: &[Manager] = if system == "Darwin" {
        &[Manager::Brew]
    } else {
        &[
            Manager::AptGet,
            Manager::Dnf,
            Manager::Pacman,
            Manager::Brew,
        ]
    };
    let search = search?;
    candidates
        .iter()
        .copied()
        .find(|candidate| find_in(candidate.program(), search).is_some())
}

/// The page that installs `program` without a package manager.
fn upstream(program: &str) -> Option<&'static str> {
    Some(match program {
        "task" => "https://taskfile.dev/installation/",
        "jq" => "https://github.com/jqlang/jq/releases/latest",
        "gh" => "https://github.com/cli/cli#installation",
        "glab" => "https://gitlab.com/gitlab-org/cli#installation",
        "git" => "https://git-scm.com/downloads",
        "claude" => "https://docs.anthropic.com/en/docs/claude-code/setup",
        _ => return None,
    })
}

/// The command that installs `program`, pinned at `pin` when one is known, with `manager`
/// (`story:install-hints`): `aep`, `ess` and `worktree` follow [`SETUP_URL`], naming the pin; a
/// program the manager packages is `<manager's install> <package>` (`task` is `go-task`, and only
/// Homebrew packages `task`, `gh` and `glab`); any other is `see <its install page>`. `None` for a
/// program doctor knows no install of.
#[must_use]
pub fn install_command(
    program: &str,
    pin: Option<&str>,
    manager: Option<Manager>,
) -> Option<String> {
    if ECOSYSTEM.contains(&program) {
        return Some(match pin {
            Some(pin) => format!("follow {SETUP_URL} (needs {program} {pin})"),
            None => format!("follow {SETUP_URL}"),
        });
    }
    let package = match (program, manager) {
        ("task", Some(Manager::Brew)) => Some("go-task"),
        ("gh" | "glab", Some(Manager::Brew)) | ("jq" | "git", Some(_)) => Some(program),
        _ => None,
    };
    match (package, manager) {
        (Some(package), Some(manager)) => Some(format!("{} {package}", manager.install())),
        _ => upstream(program).map(|page| format!("see {page}")),
    }
}

/// The programs of `programs` that [`find`] does not find, in their order.
#[must_use]
pub fn missing<'p>(programs: &[&'p str]) -> Vec<&'p str> {
    programs
        .iter()
        .copied()
        .filter(|program| find(program).is_none())
        .collect()
}

/// Refuses, naming every program of `programs` that is not on `PATH`, for `what`: the subject
/// that starts them, read with the plural verb "start" (such as "a snapshot's collectors").
///
/// # Errors
///
/// A program is missing.
pub fn require(programs: &[&str], what: &str) -> Result<()> {
    let missing = missing(programs);
    if missing.is_empty() {
        return Ok(());
    }
    let named: Vec<String> = missing
        .iter()
        .map(|program| format!("`{program}`"))
        .collect();
    bail!(
        "{} not on PATH, and {what} start {}; `conductor doctor` lists every program conductor \
         starts",
        if named.len() == 1 {
            format!("{} is", named[0])
        } else {
            format!("{} are", named.join(", "))
        },
        if named.len() == 1 { "it" } else { "them" }
    )
}

/// The first line `program --version` prints, on standard output or else standard error, empty
/// lines aside; why there is none when the run fails or prints nothing.
fn version_line(program: &Path) -> Result<String, String> {
    let output = collect::run(Command::new(program).arg("--version"), VERSION_BOUND)
        .map_err(|error| format!("{error:#}"))?;
    let first = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(str::to_owned)
    };
    first(&output.stdout)
        .or_else(|| first(&output.stderr))
        .ok_or_else(|| match output.status.code() {
            Some(code) => format!("`--version` exited {code} and printed nothing"),
            None => "`--version` was stopped by a signal".to_owned(),
        })
}

/// The version a `--version` line names: its first word that, after a leading `v` or a
/// `<name>-` prefix, is `x.y` or `x.y.z` in digits, read up to the first character that is not
/// part of it (`3.53.1+ff3372fc` is `3.53.1`, `jq-1.8.2` is `1.8.2`).
#[must_use]
pub fn version_of(line: &str) -> Option<(u64, u64, u64)> {
    line.split_whitespace().find_map(|word| {
        let word = word
            .rsplit('-')
            .find(|part| starts_with_digit(part))
            .or(Some(word))?;
        let word = word.strip_prefix('v').unwrap_or(word);
        let numeric: String = word
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let parts: Vec<&str> = numeric.trim_end_matches('.').split('.').collect();
        let number = |index: usize| parts.get(index).and_then(|part| part.parse::<u64>().ok());
        match parts.len() {
            2 => Some((number(0)?, number(1)?, 0)),
            3 => Some((number(0)?, number(1)?, number(2)?)),
            _ => None,
        }
    })
}

fn starts_with_digit(text: &str) -> bool {
    text.trim_start_matches('v')
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
}

/// A pin of `b10x.toml`: `x.y.z`, exactly that release, or `x.y`, the newest `x.y.*` (a leading
/// `v` is allowed), as the installer's `pin` command writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pin {
    /// Exactly `x.y.z`.
    Exact(u64, u64, u64),
    /// The newest `x.y.*`.
    Line(u64, u64),
}

impl Pin {
    /// Reads `0.32.0` or `0.59`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let bare = text.trim().trim_start_matches('v');
        let parts: Vec<&str> = bare.split('.').collect();
        let number = |index: usize| -> Option<u64> {
            let part = parts.get(index)?;
            (!part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
                .then(|| part.parse().ok())
                .flatten()
        };
        match parts.len() {
            2 => Some(Self::Line(number(0)?, number(1)?)),
            3 => Some(Self::Exact(number(0)?, number(1)?, number(2)?)),
            _ => None,
        }
    }

    /// Whether `version` is older than anything this pin allows.
    #[must_use]
    pub fn newer_than(self, version: (u64, u64, u64)) -> bool {
        match self {
            Self::Exact(major, minor, patch) => version < (major, minor, patch),
            Self::Line(major, minor) => (version.0, version.1) < (major, minor),
        }
    }
}

/// The nearest `b10x.toml` from `start` upward, as the installer finds it: the home directory itself
/// and everything above it are not searched, so a stray file in the home directory pins nothing.
#[must_use]
pub fn pin_file(start: &Path, home: Option<&Path>) -> Option<PathBuf> {
    for dir in start.ancestors() {
        if Some(dir) == home {
            return None;
        }
        let candidate = dir.join(PIN_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// The `[pins]` table of a `b10x.toml`: each `name = "spec"` line, comments and blank lines
/// aside, in the file's order. Any other line in that table is refused, naming its number.
///
/// # Errors
///
/// A line of `[pins]` is not `name = "spec"`.
pub fn pins(text: &str, file: &Path) -> Result<Vec<(String, String)>> {
    let mut pins = Vec::new();
    let mut in_pins = false;
    for (index, raw) in text.lines().enumerate() {
        let line = uncommented(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_pins = line == "[pins]";
            continue;
        }
        if !in_pins {
            continue;
        }
        let parsed = line.split_once('=').and_then(|(name, value)| {
            let name = name.trim().trim_matches('"');
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))?;
            (!name.is_empty()).then(|| (name.to_owned(), value.to_owned()))
        });
        match parsed {
            Some(pin) => pins.push(pin),
            None => bail!(
                "{}:{}: `{line}` is not a pin `name = \"x.y.z\"`",
                file.display(),
                index + 1
            ),
        }
    }
    Ok(pins)
}

/// `line` up to a `#` that is not inside a double-quoted string.
fn uncommented(line: &str) -> &str {
    let mut quoted = false;
    for (index, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

/// One row per program of [`PROGRAMS`], with its pin from `pins` when there is one; a forge
/// CLI of [`FORGES`] that `needed` does not name is `NotNeeded`, whether it is there or not.
/// Each program is looked for on `search`, a `PATH`; none finds nothing.
fn examine(
    pins: &[(String, String)],
    needed: &[&str],
    search: Option<&OsStr>,
) -> Vec<Prerequisite> {
    PROGRAMS
        .iter()
        .map(|program| {
            let pin = pins
                .iter()
                .find(|(name, _)| name == program)
                .map(|(_, spec)| spec.clone());
            if FORGES.contains(program) && !needed.contains(program) {
                return Prerequisite {
                    program: (*program).to_owned(),
                    state: PrerequisiteState::NotNeeded,
                    version_line: None,
                    version: None,
                    pin,
                };
            }
            let Some(path) = search.and_then(|search| find_in(program, search)) else {
                return Prerequisite {
                    program: (*program).to_owned(),
                    state: PrerequisiteState::Missing,
                    version_line: None,
                    version: None,
                    pin,
                };
            };
            let line = version_line(&path);
            let version = line.as_deref().ok().and_then(version_of);
            let state = match (pin.as_deref().map(Pin::parse), version) {
                (None, _) => PrerequisiteState::Present,
                (Some(Some(pin)), Some(version)) if pin.newer_than(version) => {
                    PrerequisiteState::OlderThanPin
                }
                (Some(Some(_)), Some(_)) => PrerequisiteState::Present,
                (Some(_), _) => PrerequisiteState::VersionUnread,
            };
            Prerequisite {
                program: (*program).to_owned(),
                state,
                version_line: Some(line.unwrap_or_else(|why| format!("version unknown: {why}"))),
                version: version.map(|(major, minor, patch)| format!("{major}.{minor}.{patch}")),
                pin,
            }
        })
        .collect()
}

/// The lines `conductor doctor` prints for `row`, whose pin is called `its` (`its pin` for a pin
/// of a `b10x.toml`, `the build's pin` for one of [`build_pins`]).
fn lines(row: &Prerequisite, its: &str) -> Vec<String> {
    let program = &row.program;
    match row.state {
        PrerequisiteState::Missing => return vec![format!("{program}: missing")],
        PrerequisiteState::NotNeeded => return vec![format!("{program}: not needed")],
        _ => {}
    }
    let mut lines = vec![format!(
        "{program}: present, {}",
        row.version_line.as_deref().unwrap_or("")
    )];
    if let Some(pin) = &row.pin {
        let version = row.version.as_deref().unwrap_or("");
        lines.push(match row.state {
            PrerequisiteState::OlderThanPin => {
                format!("{program}: {version} is older than {its} {pin}")
            }
            PrerequisiteState::VersionUnread if Pin::parse(pin).is_none() => {
                format!("{program}: {its} {pin:?} is not a version: write x.y.z or x.y")
            }
            PrerequisiteState::VersionUnread => {
                format!("{program}: no version read, so {its} {pin} cannot be checked")
            }
            _ => format!("{program}: {version} meets {its} {pin}"),
        });
    }
    lines
}

/// Whether a checkouts root is inside an activated `worktree` workspace
/// (`story:worktree-activation`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activation {
    /// `worktree` resolves a workspace profile for it.
    Activated,
    /// `worktree` refuses it as `workspace-not-activated`.
    NotActivated,
    /// Not asked or not answered, and why: no checkout under the root, or `worktree` answered
    /// something else.
    Unchecked(String),
}

/// One instance's checkouts root and whether it is in an activated `worktree` workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    /// The checkouts root, as the config file gives it.
    pub root: PathBuf,
    /// What `worktree` answered for it.
    pub activation: Activation,
}

/// The refusal `worktree` 0.18 answers, on standard error, for a repository no activated
/// workspace profile contains.
pub const NOT_ACTIVATED: &str = "workspace-not-activated";

/// The id doctor asks `worktree inspect` for: one with a dot, which no tree can have, so
/// `worktree` refuses it (`invalid-worktree-id`) after it has resolved the workspace and before
/// it inspects any tree.
const PROBE_ID: &str = "conductor.activation-probe";

/// How long `worktree inspect` may take to refuse.
const ACTIVATION_BOUND: Duration = Duration::from_secs(10);

/// The command that activates `root` as a `worktree` workspace: doctor prints it, `conductor
/// setup --apply` prints it as a `to run:` line, and neither runs it, since it writes the user's
/// own `worktree` config. `<profile>` is the user's profile template.
#[must_use]
pub fn activate_command(root: &Path) -> String {
    format!(
        "worktree activate --profile <profile> --workspace {}",
        root.display()
    )
}

/// What `worktree inspect --json` printed, on standard error then standard output, says of
/// activation: `workspace-not-activated` is [`Activation::NotActivated`]; `ok: true`, or a
/// refusal of the id (`invalid-worktree-id`, `unknown-worktree-id`), which comes only after the
/// workspace resolved, is [`Activation::Activated`]; anything else is unchecked, naming the code.
#[must_use]
pub fn activation_of(stderr: &[u8], stdout: &[u8]) -> Activation {
    let answer = [stderr, stdout]
        .into_iter()
        .find_map(|bytes| serde_json::from_slice::<serde_json::Value>(bytes).ok());
    let Some(answer) = answer else {
        return Activation::Unchecked("worktree printed no JSON answer".to_owned());
    };
    match answer.get("code").and_then(serde_json::Value::as_str) {
        Some(NOT_ACTIVATED) => Activation::NotActivated,
        Some("invalid-worktree-id" | "unknown-worktree-id") => Activation::Activated,
        Some(code) => Activation::Unchecked(format!("worktree answered {code}")),
        None if answer.get("ok") == Some(&serde_json::Value::Bool(true)) => Activation::Activated,
        None => Activation::Unchecked("worktree printed no answer code".to_owned()),
    }
}

/// Whether `root` is in an activated `worktree` workspace, asked of the `worktree` at `program`
/// (`story:worktree-activation`). `worktree doctor` and `worktree status` take no path, and every
/// command that does takes a git repository, so doctor asks `worktree inspect --json --repo
/// <checkout> --id` with [`PROBE_ID`] for the first git checkout under `root`
/// ([`crate::dashboard::waste::profile_checkout`]), or `root` itself when it is one. Read-only.
fn activation(program: &Path, root: &Path) -> Activation {
    let checkout = crate::dashboard::waste::profile_checkout(root)
        .or_else(|| root.join(".git").exists().then(|| root.to_path_buf()));
    let Some(checkout) = checkout else {
        return Activation::Unchecked("no git checkout in it".to_owned());
    };
    let answered = collect::run(
        Command::new(program)
            .args(["inspect", "--json", "--repo"])
            .arg(&checkout)
            .args(["--id", PROBE_ID]),
        ACTIVATION_BOUND,
    );
    match answered {
        Ok(output) => activation_of(&output.stderr, &output.stdout),
        Err(error) => Activation::Unchecked(format!("{error:#}")),
    }
}

/// How long `systemctl --user is-active` may take to answer (`story:usage-probe`).
const WATCHER_BOUND: Duration = Duration::from_secs(10);

/// A `usage.watcher`, the systemd user unit that rotates the harness accounts, and whether it
/// runs (`story:usage-probe`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watcher {
    /// The unit's name.
    pub unit: String,
    /// `None` when it runs; otherwise why not: the state `systemctl` names, or why it was not
    /// asked or did not answer.
    pub not_running: Option<String>,
}

/// Whether `unit` runs, asked of the `systemctl` at `program` as `systemctl --user is-active
/// <unit>`: running when it exits 0, otherwise not, with the first line it printed (such as
/// `inactive` or `failed`). Without a `systemctl` it is not running, `systemctl not found`.
fn watcher(program: Option<&Path>, unit: String) -> Watcher {
    let Some(program) = program else {
        return Watcher {
            unit,
            not_running: Some("systemctl not found".to_owned()),
        };
    };
    let answered = collect::run(
        Command::new(program).args(["--user", "is-active", &unit]),
        WATCHER_BOUND,
    );
    let not_running = match answered {
        Ok(output) if output.status.success() => None,
        Ok(output) => Some(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map_or_else(
                    || {
                        output.status.code().map_or_else(
                            || "ended by a signal".to_owned(),
                            |code| format!("exit {code}"),
                        )
                    },
                    str::to_owned,
                ),
        ),
        Err(error) => Some(one_line(&format!("{error:#}"))),
    };
    Watcher { unit, not_running }
}

/// The first line of `text`.
fn one_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().to_owned()
}

/// The line `conductor doctor` prints for `watcher`: `usage watcher: <unit> running`, or `…
/// not running (<why>)` (`story:usage-probe`).
#[must_use]
pub fn watcher_line(watcher: &Watcher) -> String {
    match &watcher.not_running {
        None => format!("usage watcher: {} running", watcher.unit),
        Some(why) => format!("usage watcher: {} not running ({why})", watcher.unit),
    }
}

/// The line `conductor doctor` prints for `workspace`.
#[must_use]
pub fn workspace_line(workspace: &Workspace) -> String {
    let root = workspace.root.display();
    match &workspace.activation {
        Activation::Activated => format!("worktree workspace: {root} activated"),
        Activation::NotActivated => format!("worktree workspace: {root} not activated"),
        Activation::Unchecked(why) => format!("worktree workspace: {root} not checked: {why}"),
    }
}

/// The forge CLIs a config needs, the hosts of its file's sources with the CLI of each, the
/// checkouts root of each instance of the file, each once, and the `usage.watcher` of each
/// instance that names one, each once ([`read_config`]).
type Forges = (
    Vec<&'static str>,
    Vec<(&'static str, String)>,
    Vec<PathBuf>,
    Vec<String>,
);

/// What doctor reads of the config `flag` (the global `--config`) and the environment name: the
/// forge CLIs of [`FORGES`] that any of its instances starts ([`forges`]), in [`FORGES`]' order,
/// every instance of the file, not only the one a command would select, since the default
/// instance's conductor serves them all, none when there is no config file; and the hosts of the
/// file's sources ([`forge_hosts`]), none without a file. Why the config
/// does not load when it does not, each line of it. The environment is `environment`, or this
/// process's when none is given.
fn read_config(
    flag: Option<&Path>,
    environment: Option<&config::Environment>,
) -> Result<Forges, Vec<String>> {
    let loaded = match environment {
        Some(environment) => config::load(flag, environment),
        None => config::Environment::from_process()
            .and_then(|environment| config::load(flag, &environment)),
    };
    match loaded {
        // Without a config file no source is named, so no forge CLI is needed: the built-in
        // instance's placeholder GitHub source is no source of the user's.
        Ok(loaded) if !loaded.read => Ok((Vec::new(), Vec::new(), Vec::new(), Vec::new())),
        Ok(loaded) => Ok((
            FORGES
                .into_iter()
                .filter(|forge| {
                    loaded
                        .config
                        .instances
                        .iter()
                        .any(|instance| forges(instance).contains(forge))
                })
                .collect(),
            forge_hosts(&loaded.config),
            loaded
                .config
                .instances
                .iter()
                .map(|instance| PathBuf::from(&instance.checkouts.root))
                .fold(Vec::new(), |mut roots, root| {
                    if !roots.contains(&root) {
                        roots.push(root);
                    }
                    roots
                }),
            loaded
                .config
                .instances
                .iter()
                .filter_map(|instance| instance.usage.watcher.clone())
                .fold(Vec::new(), |mut units, unit| {
                    if !units.contains(&unit) {
                        units.push(unit);
                    }
                    units
                }),
        )),
        Err(error) => Err(format!("{error:#}")
            .split(['\n', '\r'])
            .filter(|line| !line.trim().is_empty())
            .map(str::to_owned)
            .collect()),
    }
}

/// `conductor doctor` for the config the environment names: [`doctor_with`] without a
/// `--config`.
///
/// # Errors
///
/// What [`doctor_with`] answers.
pub fn doctor() -> Result<ExitCode> {
    doctor_with(None, &crate::confine::Host::default())
}

/// What `conductor doctor` found, before it is printed ([`check`]).
#[derive(Debug, Clone)]
pub struct Doctor {
    /// The machine it runs on.
    pub machine: Machine,
    /// Why the config does not load, each line of it; empty when it loads.
    pub config_problems: Vec<String>,
    /// One row per program of [`PROGRAMS`].
    pub rows: Vec<Prerequisite>,
    /// One login per host of the config file's sources whose CLI is present.
    pub logins: Vec<ForgeLogin>,
    /// One per checkouts root of the config file's instances, when `worktree` is there
    /// (`story:worktree-activation`).
    pub workspaces: Vec<Workspace>,
    /// One per `usage.watcher` of the config file's instances (`story:usage-probe`).
    pub watchers: Vec<Watcher>,
    /// The `b10x.toml` read, if any.
    pub pin_file: Option<PathBuf>,
    /// One row per confinement provider (`story:session-envelope-seam`), which does not change
    /// the verdict.
    pub providers: Vec<ProviderAvailability>,
    /// The pins of that file, which win over the build's.
    project: Vec<(String, String)>,
    /// The package manager found for the machine's system ([`manager`]).
    manager: Option<Manager>,
}

impl Doctor {
    /// Whether the config does not load.
    #[must_use]
    pub fn config_unreadable(&self) -> bool {
        !self.config_problems.is_empty()
    }

    /// What `conductor doctor` prints but its verdict: a `config: ` line for each line of why the
    /// config does not load, when it does not; the machine; one line per program, and under one
    /// missing or too old `  install: ` and its command ([`Doctor::install`]); one per login;
    /// the `b10x.toml` read, or `pins: none`; and one line per pinned program, meeting its pin or
    /// older than it, the build's own pin ([`build_pins`]) standing for each program the file
    /// does not pin ([`merge_pins`]).
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.config_unreadable() {
            out.extend(
                self.config_problems
                    .iter()
                    .map(|line| format!("config: {line}")),
            );
            out.push("config: does not load, so gh and glab are each counted as needed".to_owned());
        }
        out.extend(machine_lines(&self.machine));
        let its = |program: &str| {
            if self.project.iter().any(|(name, _)| name == program) {
                "its pin"
            } else {
                "the build's pin"
            }
        };
        for row in &self.rows {
            if let Some(first) = lines(row, its(row.program.as_str())).first() {
                out.push(first.clone());
            }
            if let Some(command) = self.install(row) {
                out.push(format!("  install: {command}"));
            }
        }
        out.extend(self.logins.iter().map(login_line));
        for workspace in &self.workspaces {
            out.push(workspace_line(workspace));
            if workspace.activation == Activation::NotActivated {
                out.push(format!("  activate: {}", activate_command(&workspace.root)));
            }
        }
        out.extend(self.watchers.iter().map(watcher_line));
        match &self.pin_file {
            Some(file) => out.push(format!("pins: {}", file.display())),
            None => out.push("pins: none".to_owned()),
        }
        for row in self
            .rows
            .iter()
            .filter(|row| row.pin.is_some() && row.state != PrerequisiteState::Missing)
        {
            out.extend(lines(row, its(row.program.as_str())).into_iter().skip(1));
        }
        out.extend(self.providers.iter().map(provider_line));
        out
    }

    /// The command that installs `row`'s program on this machine ([`install_command`]), when it
    /// is missing or older than its pin; `None` otherwise (`story:install-hints`).
    #[must_use]
    pub fn install(&self, row: &Prerequisite) -> Option<String> {
        matches!(
            row.state,
            PrerequisiteState::Missing | PrerequisiteState::OlderThanPin
        )
        .then(|| install_command(&row.program, row.pin.as_deref(), self.manager))
        .flatten()
    }

    /// What is not ready, in the verdict's order ([`failures`]), then each checkouts root not in
    /// an activated `worktree` workspace, as `worktree workspace (not activated: <root>)`
    /// (`story:worktree-activation`).
    #[must_use]
    pub fn failures(&self) -> Vec<String> {
        let mut failures = failures(&self.rows, &self.logins, self.config_unreadable());
        failures.extend(
            self.workspaces
                .iter()
                .filter(|workspace| workspace.activation == Activation::NotActivated)
                .map(|workspace| {
                    format!(
                        "worktree workspace (not activated: {})",
                        workspace.root.display()
                    )
                }),
        );
        failures
    }

    /// The command that activates each checkouts root not in an activated `worktree` workspace
    /// ([`activate_command`]), in the config file's order.
    #[must_use]
    pub fn activations(&self) -> Vec<String> {
        self.workspaces
            .iter()
            .filter(|workspace| workspace.activation == Activation::NotActivated)
            .map(|workspace| activate_command(&workspace.root))
            .collect()
    }

    /// The report the specification declares, `conductor.config.DoctorReport`.
    #[must_use]
    pub fn report(&self) -> DoctorReport {
        DoctorReport {
            machine: self.machine.clone(),
            prerequisites: self.rows.clone(),
            logins: self.logins.clone(),
            providers: self.providers.clone(),
            verdict: verdict_of(&self.failures()),
        }
    }
}

/// Every check of `conductor doctor` for the config `flag` and the environment name: the
/// machine ([`machine`]); why that config does not load, when it does not; one row per program
/// of [`PROGRAMS`], `present` with its version line, `missing`, or `not needed` for a forge CLI
/// that no instance of that config starts; one login per host of the config file's sources whose
/// CLI is present ([`forge_hosts`]); and the `b10x.toml` found from the working directory upward,
/// whose pins win over the build's own ([`merge_pins`]). Writes nothing.
///
/// # Errors
///
/// The working directory cannot be read, or the `b10x.toml` found cannot be read or holds a line
/// in `[pins]` that is no pin.
pub fn check(flag: Option<&Path>) -> Result<Doctor> {
    let cwd = env::current_dir().context("doctor")?;
    check_at(flag, &cwd)
}

/// [`check`] with the `b10x.toml` found from `start` upward rather than from the working
/// directory: `conductor setup` reads the pins of the repository it checks
/// (`story:setup-command`).
///
/// # Errors
///
/// The `b10x.toml` found cannot be read or holds a line in `[pins]` that is no pin.
pub fn check_at(flag: Option<&Path>, start: &Path) -> Result<Doctor> {
    check_in(None, flag, start, env::var_os("PATH").as_deref())
}

/// [`check_at`] in the environment `environment` (this process's when none is given: its
/// `HOME`, its working directory and its config variables), looking for each program on
/// `search`, a `PATH` (none finds nothing): the dashboard's health view takes doctor's rows this
/// way, over the sources it is given (`story:dashboard-health`).
///
/// # Errors
///
/// What [`check_at`] answers.
pub fn check_in(
    environment: Option<&config::Environment>,
    flag: Option<&Path>,
    start: &Path,
    search: Option<&OsStr>,
) -> Result<Doctor> {
    const WHAT: &str = "doctor";
    let home = match environment {
        Some(environment) => environment.home.clone(),
        None => env::var_os("HOME").map(PathBuf::from),
    };
    let pin_file = pin_file(start, home.as_deref());
    let project = match &pin_file {
        Some(file) => {
            let text = fs::read_to_string(file)
                .with_context(|| format!("{WHAT}: read {}", file.display()))?;
            pins(&text, file).context(WHAT)?
        }
        None => Vec::new(),
    };
    // The build's own pins stay the minimum for each program the file does not pin
    // (story:doctor-minimum-versions).
    let pins = merge_pins(&project, &build_pins());
    let (needed, hosts, roots, units, config_problems) = match read_config(flag, environment) {
        Ok((needed, hosts, roots, units)) => (needed, hosts, roots, units, Vec::new()),
        Err(why) => (FORGES.to_vec(), Vec::new(), Vec::new(), Vec::new(), why),
    };
    let rows = examine(&pins, &needed, search);
    // A login is read only for a CLI that is there (story:doctor-identity).
    let logins = hosts
        .iter()
        .filter_map(|(program, host)| {
            let row = rows.iter().find(|row| row.program == *program)?;
            if matches!(
                row.state,
                PrerequisiteState::Missing | PrerequisiteState::NotNeeded
            ) {
                return None;
            }
            search
                .and_then(|search| find_in(program, search))
                .map(|path| login(program, &path, host))
        })
        .collect();
    // Activation is asked only of a `worktree` that is there (story:worktree-activation).
    let workspaces = match search.and_then(|search| find_in("worktree", search)) {
        Some(program) => roots
            .into_iter()
            .map(|root| Workspace {
                activation: activation(&program, &root),
                root,
            })
            .collect(),
        None => Vec::new(),
    };
    // The watcher is asked of the `systemctl` on the search path (story:usage-probe).
    let systemctl = search.and_then(|search| find_in("systemctl", search));
    let watchers = units
        .into_iter()
        .map(|unit| watcher(systemctl.as_deref(), unit))
        .collect();
    let machine = machine();
    let manager = manager(&machine.system, search);
    Ok(Doctor {
        machine,
        config_problems,
        rows,
        logins,
        workspaces,
        watchers,
        pin_file,
        providers: crate::confine::availability(&crate::confine::Host::default()),
        project,
        manager,
    })
}

/// One confinement provider's row (`story:session-envelope-seam`): `provider <name>: available`,
/// or `provider <name>: not available: <reason>`.
#[must_use]
pub fn provider_line(row: &ProviderAvailability) -> String {
    if row.available {
        format!("provider {}: available", row.provider)
    } else {
        format!(
            "provider {}: not available: {}",
            row.provider,
            row.reason.as_deref().unwrap_or("no reason given")
        )
    }
}

/// `conductor doctor`: every line of [`Doctor::lines`] for what [`check`] finds, its provider rows
/// those of `providers` ([`provider_line`]), and last the verdict, `ready` or `not ready: ` with
/// what failed ([`verdict_of`]). Exits 1 when the config does not load, a program is missing,
/// older than its pin, or pinned with a pin or version that does not read, or a forge CLI is not
/// logged in to a host of the config file's sources; 0 otherwise. A provider that is not
/// available changes no verdict. Writes nothing.
///
/// # Errors
///
/// What [`check`] answers.
pub fn doctor_with(
    flag: Option<&Path>,
    providers: &dyn crate::confine::Providers,
) -> Result<ExitCode> {
    let mut doctor = check(flag)?;
    doctor.providers = crate::confine::availability(providers);
    for line in doctor.lines() {
        println!("{line}");
    }
    let verdict = verdict_of(&doctor.failures());
    println!("{verdict}");
    Ok(if verdict == READY {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// The verdict of a run every program of which passed (`story:doctor-verdict`).
pub const READY: &str = "ready";

/// What is not ready (`story:doctor-verdict`), in this order: `config (unreadable)` when
/// `config_unreadable`; then each program of `rows` that is not `Present` or `NotNeeded`, in
/// [`PROGRAMS`]' order, as `<program> (missing)`, `<program> (too old)`,
/// `<program> (pin unread)` (its version was read, its pin is no version) or
/// `<program> (version unread)`; then each of `logins` that is not logged in, as
/// `<program> (not logged in to <host>)` (`story:doctor-identity`).
#[must_use]
pub fn failures(
    rows: &[Prerequisite],
    logins: &[ForgeLogin],
    config_unreadable: bool,
) -> Vec<String> {
    let config = config_unreadable.then(|| "config (unreadable)".to_owned());
    config
        .into_iter()
        .chain(rows.iter().filter_map(|row| {
            let why = match row.state {
                PrerequisiteState::Present | PrerequisiteState::NotNeeded => return None,
                PrerequisiteState::Missing => "missing",
                PrerequisiteState::OlderThanPin => "too old",
                PrerequisiteState::VersionUnread
                    if row.version.is_some()
                        && row
                            .pin
                            .as_deref()
                            .is_some_and(|pin| Pin::parse(pin).is_none()) =>
                {
                    "pin unread"
                }
                PrerequisiteState::VersionUnread => "version unread",
            };
            Some(format!("{} ({why})", row.program))
        }))
        .chain(
            logins
                .iter()
                .filter(|login| login.state == ForgeLoginState::NotLoggedIn)
                .map(|login| format!("{} (not logged in to {})", login.program, login.host)),
        )
        .collect()
}

/// The last line `conductor doctor` prints (`story:doctor-verdict`), and `conductor setup` with
/// its own failures added (`story:setup-command`): [`READY`] when `failures` is empty, otherwise
/// `not ready: ` and each failure, comma-separated, in their order.
#[must_use]
pub fn verdict_of(failures: &[String]) -> String {
    if failures.is_empty() {
        READY.to_owned()
    } else {
        format!("not ready: {}", failures.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_read_from_each_programs_line() {
        for (line, version) in [
            ("git version 2.51.0", (2, 51, 0)),
            ("gh version 2.100.0 (2026-09-04)", (2, 100, 0)),
            ("2.1.295 (Claude Code)", (2, 1, 295)),
            ("jq-1.8.2", (1, 8, 2)),
            ("3.53.1+ff3372fc", (3, 53, 1)),
            ("aep 0.69.1", (0, 69, 1)),
            ("worktree-cli 0.13.0", (0, 13, 0)),
            ("ess 0.56.0", (0, 56, 0)),
            ("Task version: v3.40.0 (h1:abc)", (3, 40, 0)),
            ("tool 1.2", (1, 2, 0)),
        ] {
            assert_eq!(version_of(line), Some(version), "{line}");
        }
        for line in ["no version here", "", "build 7", "1.2.3.4"] {
            assert_eq!(version_of(line), None, "{line}");
        }
    }

    #[test]
    fn each_build_pin_is_a_pin_of_aep_ess_or_worktree() {
        for (program, spec) in build_pins() {
            assert!(
                ["aep", "ess", "worktree"].contains(&program.as_str()),
                "{program}"
            );
            assert!(Pin::parse(&spec).is_some(), "{program} = {spec:?}");
        }
    }

    #[test]
    fn each_program_has_an_install_command_with_and_without_a_manager() {
        let command = |program, manager| install_command(program, None, manager);
        assert_eq!(
            command("task", Some(Manager::Brew)).as_deref(),
            Some("brew install go-task")
        );
        assert_eq!(
            command("task", Some(Manager::AptGet)).as_deref(),
            Some("see https://taskfile.dev/installation/")
        );
        assert_eq!(
            command("gh", Some(Manager::Brew)).as_deref(),
            Some("brew install gh")
        );
        assert_eq!(
            command("gh", Some(Manager::Dnf)).as_deref(),
            Some("see https://github.com/cli/cli#installation")
        );
        assert_eq!(
            command("glab", Some(Manager::Pacman)).as_deref(),
            Some("see https://gitlab.com/gitlab-org/cli#installation")
        );
        assert_eq!(
            command("git", Some(Manager::Pacman)).as_deref(),
            Some("sudo pacman -S git")
        );
        assert_eq!(command("aep", None), Some(format!("follow {SETUP_URL}")));
        for program in PROGRAMS {
            assert!(command(program, None).is_some(), "{program}");
        }
        assert_eq!(command("unknown-program", None), None);
    }

    #[test]
    fn a_pin_is_exact_or_a_line() {
        assert_eq!(Pin::parse("0.32.0"), Some(Pin::Exact(0, 32, 0)));
        assert_eq!(Pin::parse("v0.59"), Some(Pin::Line(0, 59)));
        for refused in ["", "1", "1.2.3.4", "x.y", "1..2", "latest"] {
            assert_eq!(Pin::parse(refused), None, "{refused}");
        }
        assert!(Pin::Exact(0, 32, 1).newer_than((0, 32, 0)));
        assert!(!Pin::Exact(0, 32, 0).newer_than((0, 32, 0)));
        assert!(!Pin::Exact(0, 32, 0).newer_than((0, 33, 0)));
        assert!(Pin::Line(0, 70).newer_than((0, 69, 9)));
        assert!(!Pin::Line(0, 70).newer_than((0, 70, 0)));
        assert!(!Pin::Line(0, 70).newer_than((0, 71, 0)));
    }

    #[test]
    fn a_project_pin_wins_and_the_build_s_stays_for_what_it_leaves_out() {
        let pair = |name: &str, spec: &str| (name.to_owned(), spec.to_owned());
        let project = [pair("worktree", "0.1"), pair("jq", "1.8")];
        let build = [pair("aep", "0.70"), pair("worktree", "0.13.0")];
        assert_eq!(
            merge_pins(&project, &build),
            vec![
                pair("worktree", "0.1"),
                pair("jq", "1.8"),
                pair("aep", "0.70")
            ]
        );
        assert_eq!(merge_pins(&[], &build), build.to_vec());
    }

    #[test]
    fn the_verdict_names_an_unreadable_config_and_a_pin_that_is_no_version() {
        let row = |program: &str, version: Option<&str>, pin: &str| Prerequisite {
            program: program.to_owned(),
            state: PrerequisiteState::VersionUnread,
            version_line: Some(String::new()),
            version: version.map(str::to_owned),
            pin: Some(pin.to_owned()),
        };
        let rows = [
            row("aep", Some("0.70.1"), "latest"),
            row("ess", None, "0.55"),
            row("worktree", None, "latest"),
        ];
        assert_eq!(
            verdict_of(&failures(&rows, &[], true)),
            "not ready: config (unreadable), aep (pin unread), ess (version unread), \
             worktree (version unread)"
        );
        assert_eq!(
            verdict_of(&failures(&[], &[], true)),
            "not ready: config (unreadable)"
        );
        assert_eq!(verdict_of(&failures(&[], &[], false)), READY);
        let login = |state| ForgeLogin {
            program: "glab".to_owned(),
            host: "gitlab.example.com".to_owned(),
            state,
            account: None,
        };
        assert_eq!(
            verdict_of(&failures(
                &rows[..1],
                &[
                    login(ForgeLoginState::LoggedIn),
                    login(ForgeLoginState::NotLoggedIn)
                ],
                false
            )),
            "not ready: aep (pin unread), glab (not logged in to gitlab.example.com)"
        );
    }

    #[test]
    fn activation_is_read_from_worktree_s_json_answer() {
        let refusal = |code: &str| {
            format!("{{\"code\":\"{code}\",\"message\":\"{code}: x\",\"ok\":false,\"version\":4}}")
        };
        assert_eq!(
            activation_of(refusal(NOT_ACTIVATED).as_bytes(), b""),
            Activation::NotActivated
        );
        for code in ["invalid-worktree-id", "unknown-worktree-id"] {
            assert_eq!(
                activation_of(refusal(code).as_bytes(), b""),
                Activation::Activated,
                "{code}"
            );
        }
        assert_eq!(
            activation_of(b"", b"{\"version\":4,\"ok\":true}"),
            Activation::Activated
        );
        assert_eq!(
            activation_of(refusal("git-command-failed").as_bytes(), b""),
            Activation::Unchecked("worktree answered git-command-failed".to_owned())
        );
        assert!(matches!(
            activation_of(b"worktree-cli 0.18.1", b""),
            Activation::Unchecked(_)
        ));
        assert_eq!(
            activate_command(Path::new("/srv/example-org")),
            "worktree activate --profile <profile> --workspace /srv/example-org"
        );
    }

    #[test]
    fn the_account_is_the_login_name_and_nothing_else() {
        let gh = "github.com\n  \u{2713} Logged in to github.com account example-user (keyring)\n\
                  \x20 - Token: example-token\n";
        assert_eq!(
            account_of(gh, "github.com").as_deref(),
            Some("example-user")
        );
        let glab = "gitlab.example.com\n  \u{2713} Logged in to gitlab.example.com as \
                    example.user (~/.config/glab-cli/config.yml)\n";
        assert_eq!(
            account_of(glab, "gitlab.example.com").as_deref(),
            Some("example.user")
        );
        assert_eq!(account_of(glab, "github.com"), None);
        let mail = "  Logged in to github.com account user@example.com (keyring)\n";
        assert_eq!(account_of(mail, "github.com"), None);
        assert_eq!(
            account_of("You are not logged into any GitHub hosts.", "github.com"),
            None
        );
    }

    #[test]
    fn pins_are_read_from_the_pins_table_only() {
        let text = "[other]\naep = \"9.9\"\n\n[pins]\n# a comment\naep = \"0.70\" # newest\n\
                    \"ess\" = \"0.55.1\"\n[tail]\nx = 1\n";
        assert_eq!(
            pins(text, Path::new("b10x.toml")).expect("pins"),
            vec![
                ("aep".to_owned(), "0.70".to_owned()),
                ("ess".to_owned(), "0.55.1".to_owned())
            ]
        );
        let refused =
            pins("[pins]\naep = 0.70\n", Path::new("b10x.toml")).expect_err("an unquoted value");
        assert!(refused.to_string().contains("b10x.toml:2"), "{refused}");
    }
}
