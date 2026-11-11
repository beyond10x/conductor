//! Adversary cases for `story:doctor-checks-guard-forge-writes` (wave 51, unit U3, pass 1).
//!
//! Each case runs the built binary with the environment cleared: `HOME` names the case's `home/`,
//! `PATH` holds only the case's `bin/` (fakes printing a fixed version line, and for the setup
//! case a link to the real `git`). No case reads the real home, a real store or a real guard
//! state.

mod common;

use std::env;
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::{PermissionsExt as _, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Each program doctor checks, with the version line its fake prints.
const PROGRAMS: [(&str, &str); 10] = [
    ("git", "git version 2.51.0"),
    ("gh", "gh version 2.100.0 (2026-09-04)"),
    ("glab", "glab 1.46.0 (2024-09-01)"),
    ("claude", "2.1.295 (Claude Code)"),
    ("jq", "jq-1.8.2"),
    ("aep", "aep 99.1.0"),
    ("worktree", "worktree-cli 99.1.0"),
    ("ess", "ess 99.1.0"),
    ("substrate-daemon", "substrate-daemon 99.1.0"),
    ("b10x-gates", "b10x-gates 0.1.27"),
];

/// The variable that names the Gates policy.
const POLICY_VARIABLE: &str = "B10X_GATES_POLICY";

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

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("adv_w51_doctor_guard")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home/work", "bin", "checkouts"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let dir = dir.canonicalize().expect("the case's directory");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn work(&self) -> PathBuf {
        self.dir.join("home/work")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn checkouts(&self) -> PathBuf {
        self.dir.join("checkouts")
    }

    /// Every program of [`PROGRAMS`], each printing its line and exiting 0.
    fn programs(&self) {
        for (name, line) in PROGRAMS {
            let file = self.bin().join(name);
            fs::write(
                &file,
                format!("#!/bin/sh\nprintf '%s\\n' '{line}'\nexit 0\n"),
            )
            .expect("write the program");
            fs::set_permissions(&file, fs::Permissions::from_mode(0o755))
                .expect("make it runnable");
        }
    }

    /// A git checkout at `dir` whose `origin` is `url`, as its `.git/config` names it.
    fn origin(&self, dir: &Path, url: &str) {
        fs::create_dir_all(dir.join(".git")).expect("create the checkout");
        fs::write(
            dir.join(".git/config"),
            format!("[core]\n\tbare = false\n[remote \"origin\"]\n\turl = {url}\n"),
        )
        .expect("write the checkout's git config");
    }

    /// A config file of one instance `alpha` whose `sources` are `sources`, ending with `rest`.
    fn config(&self, sources: &str, rest: &str) -> PathBuf {
        let file = self.dir.join("conductor.yaml");
        fs::write(
            &file,
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: alpha\n\
                 \x20   sources: {sources}\n\
                 \x20   checkouts:\n\
                 \x20     root: {root}\n\
                 \x20     trees: {trees}\n\
                 \x20   records: {records}\n\
                 {rest}",
                root = self.checkouts().display(),
                trees = self.dir.join("trees").display(),
                records = self.dir.join("records").display(),
            ),
        )
        .expect("write the config file");
        file
    }

    /// `conductor <args>` in `work/` under `config`, with `extra` set in its environment.
    fn run(&self, config: &Path, args: &[&str], extra: &[(&str, &str)]) -> Output {
        let mut command = common::conductor(self.home());
        command
            .args(args)
            .current_dir(self.work())
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .env("CONDUCTOR_CONFIG", config)
            .stdin(Stdio::null());
        for (name, value) in extra {
            command.env(name, value);
        }
        command.output().expect("conductor runs")
    }

    /// The guard's PreToolUse hook under `config`, asked about the Bash `command` of a session
    /// whose working directory is `cwd`.
    fn hook(&self, config: &Path, cwd: &Path, command: &str) -> Output {
        let payload = serde_json::json!({
            "session_id": "example-session",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "cwd": cwd,
            "tool_input": {"command": command},
        });
        let mut child = common::conductor(self.home())
            .args(["guard", "record-guard-decision", "--from-pre-tool-use"])
            .current_dir(self.work())
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .env("CONDUCTOR_CONFIG", config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the hook starts");
        child
            .stdin
            .take()
            .expect("the hook's standard input")
            .write_all(payload.to_string().as_bytes())
            .expect("write the payload");
        child.wait_with_output().expect("the hook ends")
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

/// A `local` source over a directory of checkouts (the README's and getting-started's config) on
/// the route `gates` with a policy: one checkout on GitHub, one on a GitLab host. The hook denies
/// the controller in the GitLab checkout every merge request create, since `glab` only reads
/// under `gates`; doctor asks about the first checkout alone, prints `guard controller: allowed`
/// and ends `ready`. The acceptance: a role whose forge write the guard denies makes the verdict
/// `not ready`.
#[test]
fn adv_doctor_is_ready_while_the_hook_denies_the_lead_of_a_second_checkout() {
    let case = Case::new("second-checkout");
    case.programs();
    let policy = case.dir.join("gates-policy.json");
    fs::write(&policy, "{}\n").expect("write the policy");
    case.origin(
        &case.checkouts().join("alpha"),
        "https://github.com/example-org/alpha.git",
    );
    let beta = case.checkouts().join("beta");
    case.origin(&beta, "https://gitlab.example.com/example-group/beta.git");
    let config = case.config(
        &format!("[{{local: {}}}]", case.checkouts().display()),
        &format!("\x20   gates_policy: {}\n", policy.display()),
    );

    let hook = case.hook(
        &config,
        &beta,
        "glab mr create -R example-group/beta --fill",
    );
    assert_eq!(
        hook.status.code(),
        Some(2),
        "the hook denies the lead of beta its merge request create: {}",
        shown(&hook)
    );

    let output = case.run(&config, &["doctor"], &[]);
    assert_ne!(
        last(&output),
        "ready",
        "doctor ends ready while the hook denies the lead of beta every forge write: {}",
        shown(&output)
    );
}

/// The fix line names the path `B10X_GATES_POLICY` names as the `gates_policy` to write. With a
/// relative path in the variable (the path the broker's working directory resolves), the line
/// names the relative path, and the config file refuses it as no path: the fix cannot be made
/// as printed.
#[test]
fn adv_the_policy_fix_names_a_path_the_config_file_accepts() {
    let case = Case::new("relative-policy");
    case.programs();
    fs::write(case.work().join("gates-policy.json"), "{}\n").expect("write the policy");
    let config = case.config("[{github: example-org}]", "");
    let output = case.run(
        &config,
        &["doctor"],
        &[(POLICY_VARIABLE, "gates-policy.json")],
    );
    let lines = stdout(&output);
    let line = lines
        .iter()
        .find(|line| line.starts_with("forge route alpha: "))
        .unwrap_or_else(|| panic!("a forge route line: {}", shown(&output)));
    let value = line
        .split_once("set instances[0].gates_policy: ")
        .and_then(|(_, rest)| rest.split_once(", the path "))
        .map(|(value, _)| value.to_owned())
        .unwrap_or_else(|| panic!("the line names a gates_policy to write: {}", shown(&output)));

    let text = fs::read_to_string(&config).expect("the config file");
    fs::write(&config, format!("{text}\x20   gates_policy: {value}\n")).expect("write the fix");
    let validated = case.run(&config, &["config", "validate"], &[]);
    assert_eq!(
        validated.status.code(),
        Some(0),
        "the gates_policy doctor's fix names ({value:?}) is refused by the config file: {}",
        shown(&validated)
    );
}

/// A `gates_policy` that names no file is no policy for the bot route to write under, and
/// `conductor setup` already takes only a readable file as one. Doctor prints
/// `forge route alpha: gates, policy <path>` for a path that does not exist, and for a
/// directory, and ends `ready`.
#[test]
fn adv_doctor_is_not_ready_on_a_gates_policy_that_names_no_file() {
    let case = Case::new("missing-policy");
    case.programs();
    case.origin(
        &case.checkouts().join("alpha"),
        "https://github.com/example-org/alpha.git",
    );
    let missing = case.dir.join("no-such-policy.json");
    let directory = case.dir.join("policies");
    fs::create_dir_all(&directory).expect("create the directory");
    let mut ready = Vec::new();
    for policy in [&missing, &directory] {
        let config = case.config(
            "[{github: example-org}]",
            &format!("\x20   gates_policy: {}\n", policy.display()),
        );
        let output = case.run(&config, &["doctor"], &[]);
        if last(&output) == "ready" {
            ready.push(format!("{}: {}", policy.display(), shown(&output)));
        }
    }
    assert!(
        ready.is_empty(),
        "doctor is ready on a gates_policy that names no file:\n{}",
        ready.join("\n")
    );
}

/// Doctor with a role the guard denies, against an instance whose state directory holds a store
/// file and a guard lease copy and whose records hold a decisions log: every file under the
/// case is byte-identical after the run.
#[test]
fn adv_doctor_with_a_denied_role_changes_no_byte_of_state_or_records() {
    let case = Case::new("records-nothing");
    case.programs();
    let policy = case.dir.join("gates-policy.json");
    fs::write(&policy, "{}\n").expect("write the policy");
    case.origin(
        &case.checkouts().join("alpha"),
        "https://gitlab.example.com/example-group/alpha.git",
    );
    let records = case.dir.join("records");
    case.origin(
        &records,
        "https://gitlab.example.com/example-group/records.git",
    );
    fs::create_dir_all(records.join("decisions")).expect("create the decisions");
    fs::write(records.join("decisions/log.jsonl"), "{\"kept\":true}\n").expect("write the log");
    let state = case.dir.join("state");
    fs::create_dir_all(&state).expect("create the state");
    fs::write(state.join("conductor.sqlite"), "not a database").expect("write the store");
    fs::write(state.join("lease.json"), "{}\n").expect("write the lease copy");
    let config = case.config(
        "[{gitlab: example-group, host: gitlab.example.com}]",
        &format!(
            "\x20   state: {}\n\x20   gates_policy: {}\n",
            state.display(),
            policy.display()
        ),
    );
    let before = contents(&case.dir);
    let output = case.run(&config, &["doctor"], &[]);
    assert!(
        stdout(&output)
            .iter()
            .any(|line| line.starts_with("guard controller: denied by ")),
        "the guard denies the controller: {}",
        shown(&output)
    );
    assert_eq!(
        contents(&case.dir),
        before,
        "doctor changes no file: {}",
        shown(&output)
    );
}

/// Every file under `dir` with its bytes, sorted by path.
fn contents(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut found = Vec::new();
    let mut left = vec![dir.to_path_buf()];
    while let Some(dir) = left.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() && !path.is_symlink() {
                left.push(path);
            } else {
                let bytes = fs::read(&path).unwrap_or_default();
                found.push((path, bytes));
            }
        }
    }
    found.sort();
    found
}

// ---------------------------------------------------------------------------------------------
// The decided point: `conductor setup` ends `ready` where doctor on the same file says
// `not ready` because of the route.
// ---------------------------------------------------------------------------------------------

/// Where `program` is on this test process's own `PATH`.
fn on_path(program: &str) -> PathBuf {
    let path = env::var_os("PATH").expect("PATH is set");
    env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{program} is on PATH"))
}

struct SetupCase {
    dir: PathBuf,
}

impl Drop for SetupCase {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl SetupCase {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("adv_w51_doctor_guard_setup")
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
            if !matches!(name, "git" | "gh" | "glab" | "worktree") {
                case.script(
                    name,
                    &format!(
                        "if [ \"$1\" = --version ]; then printf '%s\\n' '{line}'; exit 0; fi\n\
                         exit 0\n"
                    ),
                );
            }
        }
        case.script(
            "gh",
            "if [ \"$1\" = --version ]; then printf '%s\\n' 'gh version 2.100.0 (2026-09-04)'; \
             exit 0; fi\n\
             if [ \"$1\" = auth ] && [ \"$2\" = status ]; then\n\
             if [ \"$4\" = github.com ]; then printf '%s\\n' 'github.com' '  Logged in to \
             github.com as example-user'; exit 0; fi\n\
             exit 1\n\
             fi\n\
             exit 0\n",
        );
        let marker = case.dir.join("worktree-activated");
        case.script(
            "worktree",
            &format!(
                "if [ \"$1\" = --version ]; then printf '%s\\n' 'worktree-cli 99.1.0'; exit 0; fi\n\
                 if [ \"$1\" = inspect ]; then\n\
                 if [ -e '{marker}' ]; then code=invalid-worktree-id; else \
                 code=workspace-not-activated; fi\n\
                 printf '{{\"code\":\"%s\",\"message\":\"%s: refused\",\"ok\":false,\"version\":4}}\\n' \
                 \"$code\" \"$code\" >&2\n\
                 exit 1\n\
                 fi\n\
                 if [ \"$1\" = activate ]; then : > '{marker}'; printf '%s\\n' activated; exit 0; fi\n\
                 exit 1\n",
                marker = marker.display(),
            ),
        );
        fs::write(case.home().join(".claude.json"), "{}\n").expect("write ~/.claude.json");
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn script(&self, name: &str, body: &str) {
        let file = self.bin().join(name);
        fs::write(&file, format!("#!/bin/sh\n{body}")).expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

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

    /// A git repository `home/src/<name>` on GitHub, with a current planning store.
    fn repository(&self, name: &str) -> PathBuf {
        let repository = self.home().join("src").join(name);
        fs::create_dir_all(&repository).expect("create the repository");
        for args in [
            vec!["init", "--quiet"],
            vec![
                "remote",
                "add",
                "origin",
                "https://github.com/example-org/zeta.git",
            ],
        ] {
            let output = self
                .isolated(Command::new("git").arg("-C").arg(&repository).args(&args))
                .output()
                .expect("git runs");
            assert!(output.status.success(), "git {args:?}: {}", shown(&output));
        }
        fs::create_dir_all(repository.join(".engineering")).expect("create the store");
        fs::write(
            repository.join(".engineering/project.yaml"),
            "version: aep.project/5\n",
        )
        .expect("write the project file");
        repository
    }

    fn conductor(&self, cwd: &Path, args: &[&str]) -> Output {
        self.isolated(
            common::conductor(self.home())
                .args(args)
                .current_dir(cwd)
                .env_clear()
                .env("PATH", self.bin()),
        )
        .output()
        .expect("conductor runs")
    }
}

/// `conductor setup --apply` writes an instance on the default route, `gates`, without a
/// `gates_policy`, prints the route line with its fix, and ends `ready`; doctor on the same file
/// then ends `not ready: … forge route zeta (no gates_policy)`. Setup's closing verdict must not
/// say `ready` then.
#[test]
fn adv_setup_is_not_ready_where_doctor_is_not_ready_on_the_route() {
    let case = SetupCase::new("ready-on-route");
    let repository = case.repository("zeta");
    let setup = case.conductor(&repository, &["setup", "--apply"]);
    let doctor = case.conductor(&repository, &["doctor"]);
    assert!(
        last(&doctor).contains("forge route zeta (no gates_policy)"),
        "doctor on the file setup wrote names the route: {}",
        shown(&doctor)
    );
    assert!(
        last(&setup).contains("forge route zeta (no gates_policy)"),
        "setup ends {:?} where doctor ends {:?}: {}",
        last(&setup),
        last(&doctor),
        shown(&setup)
    );
}
