//! `story:setup-command`: `conductor setup` prints doctor's rows, computed by doctor's own
//! checks, then four rows of the repository (its planning store's version, whether an instance
//! of the config file names it, whether that instance's records exist, whether the repository is
//! a trusted Claude Code workspace) and ends with one verdict over both. `--apply` writes the
//! instance config naming the repository and runs `conductor init` and `conductor trust` when they
//! are missing; it never logs a CLI in, installs a program or migrates a store, and prints each of
//! those as the command to run.
//!
//! Each case runs the built binary with the environment cleared: `HOME` names the case's `home/`,
//! every repository lies under it (so no `b10x.toml` of this checkout is found), and `PATH` holds
//! only the case's `bin/`: fakes printing a fixed version line, and a link to the real `git`,
//! which `conductor init` commits with. Git reads no system config and takes its identity from
//! the environment.

mod common;

use std::env;
use std::fs;
use std::os::unix::fs::{PermissionsExt as _, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// Each program conductor starts but `git`, `gh` and `glab`, with the version line its fake
/// prints; `aep`, `worktree` and `ess` newer than any pin a build carries.
const PROGRAMS: [(&str, &str); 6] = [
    ("claude", "2.1.295 (Claude Code)"),
    ("jq", "jq-1.8.2"),
    ("task", "3.53.1+ff3372fc"),
    ("aep", "aep 99.1.0"),
    ("worktree", "worktree-cli 99.1.0"),
    ("ess", "ess 99.1.0"),
];

/// The planning store version `aep` writes today, and one an older `aep` wrote.
const CURRENT_STORE: &str = "aep.project/5";
const OLD_STORE: &str = "aep.project/1";

/// The self-hosted GitLab host of the GitLab fixture.
const GITLAB_HOST: &str = "gitlab.example.com";

struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

/// Where `program` is on this test process's own `PATH`.
fn on_path(program: &str) -> PathBuf {
    let path = env::var_os("PATH").expect("PATH is set");
    env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{program} is on PATH"))
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("setup")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home/src", "bin"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let dir = dir.canonicalize().expect("the case's directory");
        let case = Self { dir };
        symlink(on_path("git"), case.bin().join("git")).expect("link git");
        for (name, line) in PROGRAMS {
            case.fake(name, line);
        }
        case.forge("gh", "gh version 2.100.0 (2026-09-04)", "github.com", true);
        case.forge("glab", "glab 1.46.0 (2024-09-01)", GITLAB_HOST, true);
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    /// The file every fake appends its arguments to, one call a line.
    fn calls(&self) -> PathBuf {
        self.dir.join("calls.log")
    }

    /// A program `name` that prints `line` for `--version`, and records every other call.
    fn fake(&self, name: &str, line: &str) {
        let file = self.bin().join(name);
        fs::write(
            &file,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then printf '%s\\n' '{line}'; exit 0; fi\n\
                 printf '%s %s\\n' '{name}' \"$*\" >> '{calls}'\n\
                 exit 0\n",
                calls = self.calls().display(),
            ),
        )
        .expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// A forge CLI `name`: `--version` prints `version`; `auth status --hostname <host>` exits 0
    /// naming the account `example-user` when `logged_in`, and 1 otherwise; every other call is
    /// recorded and exits 0.
    fn forge(&self, name: &str, version: &str, host: &str, logged_in: bool) {
        let status = if logged_in {
            format!("printf '%s\\n' '{host}' '  Logged in to {host} as example-user'; exit 0")
        } else {
            format!("printf '%s\\n' '{host} has not been authenticated' >&2; exit 1")
        };
        let file = self.bin().join(name);
        fs::write(
            &file,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then printf '%s\\n' '{version}'; exit 0; fi\n\
                 if [ \"$1\" = auth ] && [ \"$2\" = status ]; then\n\
                 if [ \"$4\" = '{host}' ]; then {status}; fi\n\
                 exit 1\n\
                 fi\n\
                 printf '%s %s\\n' '{name}' \"$*\" >> '{calls}'\n\
                 exit 0\n",
                calls = self.calls().display(),
            ),
        )
        .expect("write the forge CLI");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// Git, isolated as the module says.
    fn isolated<'c>(&self, command: &'c mut Command) -> &'c mut Command {
        command
            .env("HOME", self.home())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Example Operator")
            .env("GIT_AUTHOR_EMAIL", "operator@example.invalid")
            .env("GIT_COMMITTER_NAME", "Example Operator")
            .env("GIT_COMMITTER_EMAIL", "operator@example.invalid")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
    }

    /// A git repository `home/src/<name>`, its `origin` `origin` when there is one, and its
    /// planning store's version `store` when there is one.
    fn repository(&self, name: &str, origin: Option<&str>, store: Option<&str>) -> PathBuf {
        let repository = self.home().join("src").join(name);
        fs::create_dir_all(&repository).expect("create the repository");
        let git = |args: &[&str]| {
            let output = self
                .isolated(Command::new("git").arg("-C").arg(&repository).args(args))
                .output()
                .expect("git runs");
            assert!(output.status.success(), "git {args:?}: {}", shown(&output));
        };
        git(&["init", "--quiet"]);
        if let Some(origin) = origin {
            git(&["remote", "add", "origin", origin]);
        }
        if let Some(version) = store {
            fs::create_dir_all(repository.join(".engineering")).expect("create the store");
            fs::write(
                repository.join(".engineering/project.yaml"),
                format!("version: {version}\n\n# A comment.\n"),
            )
            .expect("write the project file");
        }
        repository
    }

    /// `~/.claude.json`, holding `{}`.
    fn claude_json(&self) -> PathBuf {
        let file = self.home().join(".claude.json");
        fs::write(&file, "{}\n").expect("write ~/.claude.json");
        file
    }

    /// The config file `conductor` reads without `--config` or `CONDUCTOR_CONFIG`.
    fn config_file(&self) -> PathBuf {
        self.home().join(".b10x/conductor/conductor.yaml")
    }

    /// `conductor setup <args>` in `cwd`.
    fn setup(&self, cwd: &Path, args: &[&str]) -> Output {
        self.isolated(
            common::conductor(self.home())
                .arg("setup")
                .args(args)
                .current_dir(cwd)
                .env_clear()
                .env("PATH", self.bin()),
        )
        .output()
        .expect("conductor runs")
    }
}

fn shown(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stdout(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn last(output: &Output) -> String {
    stdout(output).pop().unwrap_or_default()
}

/// Asserts `output` printed each of `expected` as a line of its own.
fn holds(output: &Output, expected: &[String]) {
    let lines = stdout(output);
    for line in expected {
        assert!(lines.contains(line), "no line {line:?}: {}", shown(output));
    }
}

/// The repository rows of a repository no instance names: the instance row names the source
/// `--apply` would write for its origin.
fn unconfigured(repository: &Path, source: &str) -> Vec<String> {
    let shown = repository.display();
    vec![
        format!(
            "instance: none names {shown}, which --apply names with the source {source}; run \
             conductor setup --apply"
        ),
        "records: missing (no instance names the repository)".to_owned(),
        format!("trust: not trusted ({shown})"),
    ]
}

/// Doctor's rows come first, computed by doctor's checks: the machine, a program row and the
/// pins line.
fn holds_doctor_rows(output: &Output) {
    let lines = stdout(output);
    let host = lines.iter().position(|line| line.starts_with("host: "));
    let program = lines
        .iter()
        .position(|line| line.starts_with("aep: present, "));
    let pins = lines.iter().position(|line| line == "pins: none");
    let store = lines.iter().position(|line| line.starts_with("store: "));
    assert!(
        matches!((host, program, pins, store), (Some(a), Some(b), Some(c), Some(d)) if a < b && b < c && c < d),
        "doctor's machine, program and pins rows come before the repository's: {}",
        shown(output)
    );
}

#[test]
fn setup_reads_a_github_repository_that_nothing_names_yet() {
    let case = Case::new("github");
    let repository = case.repository(
        "alpha",
        Some("https://github.com/example-org/alpha.git"),
        Some(CURRENT_STORE),
    );
    let output = case.setup(&repository, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    holds_doctor_rows(&output);
    let mut expected = vec![format!("store: current ({CURRENT_STORE})")];
    expected.extend(unconfigured(&repository, "{github: example-org}"));
    holds(&output, &expected);
    assert_eq!(
        last(&output),
        "not ready: instance (none), records (missing), trust (not trusted)",
        "{}",
        shown(&output)
    );
    assert!(
        !case.config_file().exists(),
        "without --apply setup writes nothing"
    );
}

#[test]
fn setup_reads_a_self_hosted_gitlab_repository_named_by_flag() {
    let case = Case::new("gitlab");
    let repository = case.repository(
        "beta",
        Some(&format!("git@{GITLAB_HOST}:example-group/beta.git")),
        Some(CURRENT_STORE),
    );
    let output = case.setup(
        &case.home(),
        &["--repository", repository.to_str().expect("UTF-8")],
    );
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    holds_doctor_rows(&output);
    holds(
        &output,
        &unconfigured(
            &repository,
            &format!("{{gitlab: example-group, host: {GITLAB_HOST}}}"),
        ),
    );
    assert_eq!(
        last(&output),
        "not ready: instance (none), records (missing), trust (not trusted)",
        "{}",
        shown(&output)
    );
}

#[test]
fn setup_reads_a_repository_without_an_origin_or_a_store() {
    let case = Case::new("no-origin");
    let repository = case.repository("gamma", None, None);
    let output = case.setup(&repository, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let parent = case.home().join("src");
    let mut expected = vec!["store: none".to_owned()];
    expected.extend(unconfigured(
        &repository,
        &format!("{{local: {}}}", parent.display()),
    ));
    holds(&output, &expected);
    // No store is no failure: conductor reads a store only where there is one.
    assert_eq!(
        last(&output),
        "not ready: instance (none), records (missing), trust (not trusted)",
        "{}",
        shown(&output)
    );
}

#[test]
fn setup_names_an_old_store_and_the_migrate_command() {
    let case = Case::new("old-store");
    let repository = case.repository(
        "delta",
        Some("https://github.com/example-org/delta.git"),
        Some(OLD_STORE),
    );
    let output = case.setup(&repository, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    holds(
        &output,
        &[format!(
            "store: old ({OLD_STORE}); run aep plan store migrate git --dry-run"
        )],
    );
    assert_eq!(
        last(&output),
        "not ready: store (old), instance (none), records (missing), trust (not trusted)",
        "{}",
        shown(&output)
    );
}

/// `--apply` writes the config naming the repository, runs `conductor init` and
/// `conductor trust`, and so turns those three rows ready; it does not log `glab` in, install the
/// missing `jq` or migrate the old store, and prints each as the command to run. A second run
/// changes nothing and prints the same rows.
#[test]
fn setup_apply_fixes_what_is_safe_and_prints_what_is_not() {
    let case = Case::new("apply");
    fs::remove_file(case.bin().join("jq")).expect("take jq off PATH");
    case.forge("glab", "glab 1.46.0 (2024-09-01)", GITLAB_HOST, false);
    let claude_json = case.claude_json();
    let repository = case.repository(
        "epsilon",
        Some(&format!("https://{GITLAB_HOST}/example-group/epsilon.git")),
        Some(OLD_STORE),
    );
    let records = case.home().join(".b10x/conductor/epsilon/records");
    let shown_repository = repository.display().to_string();

    for run in 0..2 {
        let output = case.setup(&repository, &["--apply"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "run {run}: {}",
            shown(&output)
        );
        holds(
            &output,
            &[
                format!("store: old ({OLD_STORE}); run aep plan store migrate git --dry-run"),
                format!("instance: epsilon names {shown_repository}"),
                format!("records: present ({})", records.display()),
                format!("trust: trusted ({shown_repository})"),
                "to install: jq".to_owned(),
                format!("to run: glab auth login --hostname {GITLAB_HOST}"),
                "to run: aep plan store migrate git --dry-run".to_owned(),
            ],
        );
        assert_eq!(
            last(&output),
            format!("not ready: jq (missing), glab (not logged in to {GITLAB_HOST}), store (old)"),
            "run {run}: {}",
            shown(&output)
        );
    }

    let config = fs::read_to_string(case.config_file()).expect("--apply wrote the config file");
    for text in [
        "name: epsilon",
        "gitlab: example-group",
        &format!("host: {GITLAB_HOST}"),
    ] {
        assert!(
            config.contains(text),
            "the config holds {text:?}:\n{config}"
        );
    }
    assert!(
        records.join("NORTHSTAR.md").is_file(),
        "conductor init wrote the records"
    );
    let claude: Value =
        serde_json::from_str(&fs::read_to_string(&claude_json).expect("read ~/.claude.json"))
            .expect("JSON");
    assert_eq!(
        claude["projects"][shown_repository.as_str()]["hasTrustDialogAccepted"],
        Value::Bool(true),
        "conductor trust marked the repository: {claude}"
    );
    let project = fs::read_to_string(repository.join(".engineering/project.yaml"))
        .expect("the project file is there");
    assert!(
        project.starts_with(&format!("version: {OLD_STORE}")),
        "the store is not migrated"
    );
    let calls = fs::read_to_string(case.calls()).unwrap_or_default();
    for never in ["glab auth login", "aep plan store migrate"] {
        assert!(!calls.contains(never), "--apply ran `{never}`:\n{calls}");
    }
}
