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

mod active;
mod common;

use std::env;
use std::fs;
use std::os::unix::fs::{PermissionsExt as _, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// Each program conductor starts but `git`, `gh` and `glab`, with the version line its fake
/// prints; `aep`, `worktree`, `ess` and `substrate-daemon` newer than any pin a build carries.
const PROGRAMS: [(&str, &str); 7] = [
    ("claude", "2.1.295 (Claude Code)"),
    ("jq", "jq-1.8.2"),
    ("aep", "aep 99.1.0"),
    ("worktree", "worktree-cli 99.1.0"),
    ("ess", "ess 99.1.0"),
    ("substrate-daemon", "substrate-daemon 99.1.0"),
    ("b10x-gates", "b10x-gates 0.1.27"),
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

    /// A fake `uname` found before the system's: `-s` prints `system`, every other flag a fixed
    /// word (`story:install-hints`).
    fn uname(&self, system: &str) {
        let file = self.bin().join("uname");
        fs::write(
            &file,
            format!(
                "#!/bin/sh\n\
                 case \"$1\" in\n\
                 \x20 -s) printf '%s\\n' '{system}' ;;\n\
                 \x20 *) printf '%s\\n' 'example' ;;\n\
                 esac\n"
            ),
        )
        .expect("write uname");
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
/// missing `jq` or migrate the old store, and prints each as the command to run: for `jq` the one
/// doctor prints under its row, here the package of the fake Linux's `apt-get`
/// (`story:install-hints`). A second run changes nothing and prints the same rows.
#[test]
fn setup_apply_fixes_what_is_safe_and_prints_what_is_not() {
    let case = Case::new("apply");
    fs::remove_file(case.bin().join("jq")).expect("take jq off PATH");
    case.uname("Linux");
    case.fake("apt-get", "apt 2.9.0");
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
                "jq: missing".to_owned(),
                "  install: sudo apt-get install jq".to_owned(),
                "to install: jq: sudo apt-get install jq".to_owned(),
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
    for never in ["glab auth login", "aep plan store migrate", "apt-get"] {
        assert!(!calls.contains(never), "--apply ran `{never}`:\n{calls}");
    }
}

/// The variable that names the config file a [`daemon_child`] run opens the daemon on.
const DAEMON_CHILD: &str = "CONDUCTOR_TEST_SETUP_DAEMON";

/// A service manager that starts nothing: [`daemon_child`] only opens the daemon.
struct NoUnits;

impl conductor_cli::daemon::units::Units for NoUnits {
    fn start(&mut self, service: &conductor_cli::systemd::Service) -> anyhow::Result<()> {
        anyhow::bail!("no unit starts here: {}", service.unit)
    }

    fn stop(&mut self, _: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn active(&mut self, _: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    fn enable(&mut self, _: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn disable(&mut self, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

/// The child run of this test binary that opens conductord on the config file [`DAEMON_CHILD`]
/// names, with `HOME` the case's (so the store URL's `~` is the case's home); a plain run of the
/// suite passes at once. It prints `opened` and exits 0, or names the refusal and fails.
#[test]
fn daemon_child() {
    let Some(file) = env::var_os(DAEMON_CHILD) else {
        return;
    };
    active::isolate();
    let home = PathBuf::from(env::var_os("HOME").expect("the case's HOME"));
    let dir = PathBuf::from(env::var_os("CONDUCTOR_TEST_SETUP_DAEMON_DIR").expect("a directory"));
    let binary = conductor_cli::daemon::reconcile::Binary {
        path: PathBuf::from("/opt/conductor/bin/conductor"),
        version: "1.0.0".to_owned(),
    };
    match conductor_cli::daemon::reconcile::Daemon::open(
        PathBuf::from(file),
        home,
        dir,
        binary,
        NoUnits,
    ) {
        Ok(daemon) => println!("opened {}", daemon.config().instances.len()),
        Err(error) => panic!("the daemon refuses the file: {error:#}"),
    }
}

/// `--apply` with no config file writes a `conductor.config/2` file the daemon reconciles: it
/// names `daemon.store`, and its instance its `placement` and `session_prefix`. `conductor config
/// validate` accepts it, and conductord opens on it (it refuses a `/1` file and one without
/// `daemon.store`). With a config file there, `--apply` prints the instance to add, the lines of
/// one instance only, its placement among them.
#[test]
fn setup_apply_writes_a_second_version_file_the_daemon_reconciles() {
    let case = Case::new("second");
    case.claude_json();
    let repository = case.repository(
        "zeta",
        Some("https://github.com/example-org/zeta.git"),
        Some(CURRENT_STORE),
    );
    let output = case.setup(&repository, &["--apply"]);
    let config = fs::read_to_string(case.config_file()).unwrap_or_else(|error| {
        panic!("--apply wrote no config file ({error}): {}", shown(&output))
    });
    assert!(
        config.starts_with("version: conductor.config/2\n"),
        "a /2 file:\n{config}"
    );
    let parsed: serde_yaml::Value = serde_yaml::from_str(&config).expect("the file is YAML");
    let store = parsed["daemon"]["store"].as_str().unwrap_or_default();
    assert!(store.starts_with("sqlite:"), "daemon.store:\n{config}");
    let instance = &parsed["instances"][0];
    assert_eq!(instance["name"].as_str(), Some("zeta"), "{config}");
    assert_eq!(instance["placement"].as_str(), Some("systemd"), "{config}");
    assert_eq!(
        instance["session_prefix"].as_str(),
        Some("zeta"),
        "{config}"
    );

    let validated = case
        .isolated(
            common::conductor(case.home())
                .args(["config", "validate"])
                .current_dir(&repository)
                .env_clear()
                .env("PATH", case.bin()),
        )
        .output()
        .expect("conductor runs");
    assert_eq!(
        validated.status.code(),
        Some(0),
        "config validate: {}",
        shown(&validated)
    );

    let opened = Command::new(env::current_exe().expect("this test binary"))
        .args(["--exact", "daemon_child", "--nocapture", "-q"])
        .env_clear()
        .env("HOME", case.home())
        .env("PATH", case.bin())
        .env(DAEMON_CHILD, case.config_file())
        .env("CONDUCTOR_TEST_SETUP_DAEMON_DIR", case.dir.join("daemon"))
        .stdin(Stdio::null())
        .output()
        .expect("the child runs");
    assert!(
        opened.status.success() && String::from_utf8_lossy(&opened.stdout).contains("opened 1"),
        "the daemon opens on the file: {}",
        shown(&opened)
    );
    assert!(
        case.home()
            .join(".b10x/conductor/conductor.sqlite")
            .is_file(),
        "the store is under the case's home"
    );

    // A repository outside the instance's checkouts root, which no instance names.
    let other = case.home().join("elsewhere/eta");
    fs::create_dir_all(&other).expect("create the repository");
    let init = case
        .isolated(
            Command::new("git")
                .arg("-C")
                .arg(&other)
                .args(["init", "--quiet"]),
        )
        .output()
        .expect("git runs");
    assert!(init.status.success(), "git init: {}", shown(&init));
    let output = case.setup(&other, &["--apply"]);
    let added: Vec<String> = stdout(&output)
        .into_iter()
        .filter_map(|line| line.strip_prefix("to write: ").map(str::to_owned))
        .collect();
    assert!(
        added.iter().any(|line| line.trim() == "- name: eta"),
        "the instance to add: {}",
        shown(&output)
    );
    assert!(
        added.iter().any(|line| line.trim() == "placement: systemd"),
        "its placement: {}",
        shown(&output)
    );
    for key in ["version:", "daemon:", "store:", "instances:"] {
        assert!(
            !added.iter().any(|line| line.trim_start().starts_with(key)),
            "no {key} line among the instance's: {}",
            shown(&output)
        );
    }
    assert_eq!(
        fs::read_to_string(case.config_file()).expect("the config file"),
        config,
        "the file is unchanged"
    );
}

// ---------------------------------------------------------------------------------------------
// story:worktree-activation: `--apply` activates the checkouts root as a `worktree` workspace with
// a profile it writes, since a person new to conductor has none (every v1.0.0 onboarding run
// asked for one).
// ---------------------------------------------------------------------------------------------

impl Case {
    /// A fake `worktree` 0.18: `inspect` refuses as `workspace-not-activated` until `activate`
    /// has run, and as `invalid-worktree-id`, the refusal after the workspace resolved, from then
    /// on; `activate` exits `activate_exit`, and on 0 prints its line and marks the workspace
    /// activated. Every call but `--version` is recorded.
    fn worktree(&self, activate_exit: i32) {
        let marker = self.dir.join("worktree-activated");
        let file = self.bin().join("worktree");
        fs::write(
            &file,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then printf '%s\\n' 'worktree-cli 99.1.0'; exit 0; fi\n\
                 printf '%s %s\\n' worktree \"$*\" >> '{calls}'\n\
                 if [ \"$1\" = inspect ]; then\n\
                 if [ -e '{marker}' ]; then code=invalid-worktree-id; else code=workspace-not-activated; fi\n\
                 printf '{{\"code\":\"%s\",\"message\":\"%s: refused\",\"ok\":false,\"version\":4}}\\n' \"$code\" \"$code\" >&2\n\
                 exit 1\n\
                 fi\n\
                 if [ \"$1\" = activate ]; then\n\
                 if [ {activate_exit} -ne 0 ]; then printf '%s\\n' 'workspace-root-invalid: refused' >&2; exit {activate_exit}; fi\n\
                 : > '{marker}'\n\
                 printf '%s\\n' 'activated'\n\
                 exit 0\n\
                 fi\n\
                 exit 1\n",
                calls = self.calls().display(),
                marker = marker.display(),
            ),
        )
        .expect("write worktree");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// The `worktree` profile template `--apply` writes for the instance `name`, beside the
    /// config file.
    fn profile(&self, name: &str) -> PathBuf {
        self.home()
            .join(".b10x/conductor/worktree")
            .join(format!("{name}.toml"))
    }

    /// The `worktree activate` calls the fake recorded.
    fn activations(&self) -> Vec<String> {
        fs::read_to_string(self.calls())
            .unwrap_or_default()
            .lines()
            .filter(|call| call.starts_with("worktree activate"))
            .map(str::to_owned)
            .collect()
    }
}

/// With no config file and no profile, `--apply` writes the profile template for the instance it
/// writes, its trees the instance's `checkouts.trees`, runs `worktree activate` with it on the
/// checkouts root, reads the root again as activated, and ends `ready`.
#[test]
fn setup_apply_activates_the_worktree_workspace_with_a_profile_it_writes() {
    let case = Case::new("worktree-activation");
    case.worktree(0);
    case.claude_json();
    let repository = case.repository(
        "zeta",
        Some("https://github.com/example-org/zeta.git"),
        Some(CURRENT_STORE),
    );
    let root = case.home().join("src");
    let profile = case.profile("zeta");
    let output = case.setup(&repository, &["--apply"]);
    assert_eq!(
        fs::read_to_string(&profile).unwrap_or_default(),
        format!(
            "version = 1\n\
             name = \"conductor-zeta\"\n\
             worktree_root = \"{}\"\n\
             expire_after_seconds = 604800\n\
             protect_workspace_root = true\n",
            case.home()
                .join(".local/state/worktree/trees/zeta")
                .display()
        ),
        "{}",
        shown(&output)
    );
    assert_eq!(
        case.activations(),
        vec![format!(
            "worktree activate --profile {} --workspace {}",
            profile.display(),
            root.display()
        )],
        "{}",
        shown(&output)
    );
    holds(
        &output,
        &[format!("worktree workspace: {} activated", root.display())],
    );
    assert_eq!(last(&output), "ready", "{}", shown(&output));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
}

/// A profile template that is there already is the person's: `--apply` activates with it and
/// leaves it as it is.
#[test]
fn setup_apply_keeps_a_profile_that_is_there() {
    let case = Case::new("worktree-profile-kept");
    case.worktree(0);
    case.claude_json();
    let repository = case.repository(
        "zeta",
        Some("https://github.com/example-org/zeta.git"),
        Some(CURRENT_STORE),
    );
    let profile = case.profile("zeta");
    fs::create_dir_all(profile.parent().expect("a directory")).expect("create its directory");
    let own = "version = 1\nname = \"own\"\nexpire_after_seconds = 60\n";
    fs::write(&profile, own).expect("write the profile");
    let output = case.setup(&repository, &["--apply"]);
    assert_eq!(
        fs::read_to_string(&profile).expect("the profile"),
        own,
        "{}",
        shown(&output)
    );
    assert_eq!(case.activations().len(), 1, "{}", shown(&output));
    assert_eq!(last(&output), "ready", "{}", shown(&output));
}

/// Without `--apply` setup writes nothing: no profile, no activation, and the root's row says
/// `not activated` with the command that activates it.
#[test]
fn setup_without_apply_activates_nothing() {
    let case = Case::new("worktree-check-only");
    case.worktree(0);
    case.claude_json();
    let repository = case.repository(
        "zeta",
        Some("https://github.com/example-org/zeta.git"),
        Some(CURRENT_STORE),
    );
    let first = case.setup(&repository, &["--apply"]);
    assert_eq!(last(&first), "ready", "{}", shown(&first));
    fs::remove_file(case.dir.join("worktree-activated")).expect("deactivate the fake's workspace");
    fs::remove_file(case.profile("zeta")).expect("remove the profile");
    fs::remove_file(case.calls()).expect("clear the calls");
    let root = case.home().join("src");
    let output = case.setup(&repository, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert!(case.activations().is_empty(), "{}", shown(&output));
    assert!(!case.profile("zeta").exists(), "{}", shown(&output));
    holds(
        &output,
        &[
            format!("worktree workspace: {} not activated", root.display()),
            format!(
                "  activate: conductor setup --apply --repository {}",
                repository.display()
            ),
        ],
    );
}

/// A `worktree` that refuses the activation: `--apply` names the refusal on standard error,
/// prints the exact command as a `to run:` line, with the profile it wrote, and is not ready.
#[test]
fn setup_apply_prints_the_activation_worktree_refused() {
    let case = Case::new("worktree-activation-refused");
    case.worktree(1);
    case.claude_json();
    let repository = case.repository(
        "zeta",
        Some("https://github.com/example-org/zeta.git"),
        Some(CURRENT_STORE),
    );
    let root = case.home().join("src");
    let profile = case.profile("zeta");
    let output = case.setup(&repository, &["--apply"]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert!(profile.is_file(), "{}", shown(&output));
    holds(
        &output,
        &[format!(
            "to run: worktree activate --profile {} --workspace {}",
            profile.display(),
            root.display()
        )],
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("workspace-root-invalid"),
        "{}",
        shown(&output)
    );
    assert_eq!(
        last(&output),
        format!(
            "not ready: worktree workspace (not activated: {})",
            root.display()
        ),
        "{}",
        shown(&output)
    );
}
