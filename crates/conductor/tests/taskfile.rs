//! `story:instance-records-start`: the tasks of `Taskfile.yml` that run conductor run it in the
//! active instance's records directory, `task trust` trusts that directory, and `task
//! agents:link REPO=<path>` links this checkout's Claude Code adapters into that repository's
//! `.claude/agents/` without overwriting anything (`story:agents-link-local`). `task trust` runs `conductor trust` (`story:portable-trust`), whose own
//! cases are `tests/trust.rs`.
//!
//! The first tests read `Taskfile.yml` as YAML. The others run the tasks with `task` against a
//! case directory under this test target's temporary directory: `HOME` names the case's home,
//! `CONDUCTOR_CONFIG` a config file the case writes (or is removed), `CONDUCTOR_INSTANCE` is
//! removed, and `PATH` starts with the case's `bin/`, which holds the built `conductor` and a
//! `claude` that logs its working directory and arguments, so no case starts a session or reads
//! the operator's config. When `task` is not on `PATH` those tests print why and return.

mod common;

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// The workspace root, which holds `Taskfile.yml`.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

fn taskfile() -> serde_yaml::Value {
    let text = fs::read_to_string(root().join("Taskfile.yml")).expect("read Taskfile.yml");
    serde_yaml::from_str(&text).expect("Taskfile.yml is YAML")
}

/// The `confinement` every instance of a case writes: the `none` provider, so a task that starts
/// a session through the built `conductor` launches the case's `claude` itself, and never reaches
/// the host's `systemd-run` whatever cgroup controllers the host delegates
/// (`story:session-limits-systemd`).
const UNBOXED: &str = "\x20   confinement: {provider: none}\n";

/// The tasks that run a conductor session.
const CONDUCTOR_TASKS: [&str; 3] = ["conductor:start", "conductor:attach", "conductor:restart"];

/// The records directory of the instance `conductor config show` reports: the one it selects.
const RECORDS: &str = "conductor config show --format json | jq -r '.instances[0].records'";

/// The prefix every shell command of a conductor task starts with.
const CD_RECORDS: &str = "cd {{shellQuote .RECORDS}} && ";

/// The agent the role conductor starts with: its `agent`, else `conductor`.
const AGENT: &str = "conductor config show --format json | jq -r '.instances[0].roles[] | \
                     select(.role == \"conductor\") | .agent // \"conductor\"'";

/// The adapters `task agents:link` links.
const AGENTS: [&str; 3] = ["conductor", "repo-controller", "conductor-dev"];

/// The shell commands of `task`: its `cmds:` entries that are text (not `task:` calls).
fn shell_commands(task: &serde_yaml::Value) -> Vec<String> {
    task["cmds"]
        .as_sequence()
        .into_iter()
        .flatten()
        .filter_map(|cmd| cmd.as_str().map(str::to_owned))
        .collect()
}

#[test]
fn each_conductor_task_changes_to_the_records_directory_config_show_names() {
    let taskfile = taskfile();
    for name in CONDUCTOR_TASKS {
        let task = &taskfile["tasks"][name];
        assert!(!task.is_null(), "Taskfile.yml has no task {name}");
        assert_eq!(
            task["vars"]["RECORDS"]["sh"].as_str(),
            Some(RECORDS),
            "{name} reads the records directory from config show"
        );
        let commands = shell_commands(task);
        assert!(!commands.is_empty(), "{name} runs no shell command");
        // `conductor session start` and `session stop` run conductor's session in the records
        // directory themselves (`story:session-envelope-seam`).
        for command in &commands {
            assert!(
                command.starts_with(CD_RECORDS) || command.starts_with("conductor session "),
                "{name}: a command that does not start in the records directory: {command:?}"
            );
        }
        let preconditions =
            serde_yaml::to_string(&task["preconditions"]).expect("preconditions as text");
        assert!(
            preconditions.contains("test -d {{shellQuote .RECORDS}}"),
            "{name} refuses a records directory that is not there: {preconditions}"
        );
    }
}

#[test]
fn conductor_starts_as_the_conductor_agent_with_the_role_settings_or_this_checkouts() {
    let taskfile = taskfile();
    let task = &taskfile["tasks"]["conductor:start"];
    assert_eq!(
        task["vars"]["ROLE_SETTINGS"]["sh"].as_str(),
        Some(
            "conductor config show --format json | jq -r '.instances[0].roles[] | \
             select(.role == \"conductor\") | .settings // empty'"
        ),
        "{task:?}"
    );
    assert_eq!(
        task["vars"]["SETTINGS"].as_str(),
        Some(
            "{{.ROLE_SETTINGS | default (printf \"%s/.claude/conductor-settings.json\" \
             .ROOT_DIR)}}"
        ),
        "{task:?}"
    );
    // The role's `agent` when it names one, else `conductor` (coordinator decision 4, pass 1).
    assert_eq!(
        task["vars"]["AGENT"]["sh"].as_str(),
        Some(AGENT),
        "{task:?}"
    );
    // The role's `profile` when it names one (W02 U1).
    assert_eq!(
        task["vars"]["PROFILE"]["sh"].as_str(),
        Some(
            "conductor config show --format json | jq -r '.instances[0].roles[] | \
             select(.role == \"conductor\") | .profile // empty'"
        ),
        "{task:?}"
    );
    // The start itself is the binary's (`story:session-envelope-seam`): the role's settings,
    // else this checkout's, which the task's working directory is.
    assert_eq!(
        shell_commands(task),
        ["conductor session start --role conductor"],
        "{task:?}"
    );
}

/// `story:session-envelope-seam`: `conductor:start`, `conductor:restart` and `dev:start` start
/// their session with `conductor session start --role …`, and `conductor:restart` stops the old
/// one with `conductor session stop --role conductor`; no task starts or stops a session with
/// `claude --bg`, `claude stop` or `claude rm` itself.
#[test]
fn the_session_tasks_start_and_stop_through_conductor_session() {
    let taskfile = taskfile();
    for (name, expected) in [
        (
            "conductor:start",
            "conductor session start --role conductor",
        ),
        (
            "conductor:restart",
            "conductor session stop --role conductor",
        ),
        ("dev:start", "conductor session start --role conductor-dev"),
    ] {
        let task = &taskfile["tasks"][name];
        assert_eq!(shell_commands(task), [expected], "{name}: {task:?}");
        let text = serde_yaml::to_string(&task["cmds"]).expect("the commands as text");
        for old in ["claude --bg", "claude stop", "claude rm"] {
            assert!(!text.contains(old), "{name} runs {old}: {text}");
        }
    }
}

#[test]
fn the_dashboard_reads_the_instances_records_not_this_checkout() {
    let taskfile = taskfile();
    let commands = shell_commands(&taskfile["tasks"]["dashboard"]).join("\n");
    assert!(
        commands.contains("dashboard serve"),
        "the dashboard task serves the dashboard: {commands}"
    );
    // Without --root the dashboard reads the active instance's records (`dashboard serve --help`);
    // this checkout holds no decisions/ or dispatches/ once the records live apart.
    assert!(
        !commands.contains("--root"),
        "the dashboard task passes no --root: {commands}"
    );
}

/// `task trust` is `conductor trust`, with no shell of its own (`story:portable-trust`): the
/// records directory's physical path and the refusals are the command's, `tests/trust.rs`.
#[test]
fn trust_runs_the_conductor_command() {
    let taskfile = taskfile();
    let task = &taskfile["tasks"]["trust"];
    let commands: Vec<&str> = task["cmds"]
        .as_sequence()
        .expect("trust has cmds")
        .iter()
        .map(|cmd| cmd.as_str().expect("a shell command"))
        .collect();
    assert_eq!(commands, ["conductor trust"], "{task:?}");
    assert!(
        task["vars"].is_null(),
        "trust reads nothing itself: {task:?}"
    );
    assert!(
        task["preconditions"].is_null(),
        "trust refuses through the command: {task:?}"
    );
}

#[test]
fn agents_link_names_the_three_adapters_and_never_overwrites() {
    let taskfile = taskfile();
    let task = &taskfile["tasks"]["agents:link"];
    assert!(!task.is_null(), "Taskfile.yml has no task agents:link");
    let commands = shell_commands(task).join("\n");
    assert!(
        commands.contains("for name in conductor repo-controller conductor-dev"),
        "{commands}"
    );
    for forbidden in ["ln -sf", "ln -fs", "ln -f", "mv ", "--force"] {
        assert!(
            !commands.contains(forbidden),
            "agents:link may overwrite ({forbidden}): {commands}"
        );
    }
    // The one removal: a symbolic link under ~/.claude/agents/ that points into this checkout,
    // which an earlier agents:link made; nothing in REPO/.claude/agents/ is ever removed.
    assert_eq!(
        commands.matches("rm ").count(),
        1,
        "agents:link removes only an old link into this checkout: {commands}"
    );
    for guard in [
        "old=\"$HOME/.claude/agents/$name.md\"",
        "if [ -L \"$old\" ]; then",
        "\"$root\"/*|\"$real\"/*) rm \"$old\"",
    ] {
        assert!(
            commands.contains(guard),
            "agents:link removes only an old link into this checkout ({guard}): {commands}"
        );
    }
    for agent in AGENTS {
        assert!(
            root().join(format!(".claude/agents/{agent}.md")).is_file(),
            "this checkout has no .claude/agents/{agent}.md"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The tasks, run
// ---------------------------------------------------------------------------------------------

/// `program` on `PATH`, if it is there.
fn which(program: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
}

/// One case: `home/`, `bin/`, `records/` and `checkouts/` under its own directory.
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
    /// The case `name`, or `None` with the reason printed when `task` is not on `PATH`.
    fn new(name: &str) -> Option<Self> {
        if which("task").is_none() {
            eprintln!("skipped {name}: `task` (go-task) is not on PATH, so the task is not run");
            return None;
        }
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("taskfile")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "bin", "records", "checkouts", "claude"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let dir = dir.canonicalize().expect("the case's directory");
        let binary = common::conductor(dir.join("home"));
        std::os::unix::fs::symlink(binary.get_program(), dir.join("bin/conductor"))
            .expect("link the built conductor");
        // A claude that answers `agents --json` with a live session `s-0001` while
        // `claude/live` exists, named as `claude/live` says or `conductor` when it is empty,
        // removes that file on `stop`, and logs every other call: its working directory, a tab,
        // its arguments.
        let claude = dir.join("bin/claude");
        fs::write(
            &claude,
            format!(
                "#!/bin/sh\n\
                 state='{state}'\n\
                 if [ \"$1\" = agents ]; then\n\
                 \x20 if [ -e \"$state/live\" ]; then\n\
                 \x20   name=$(cat \"$state/live\"); name=${{name:-conductor}}\n\
                 \x20   printf '[{{\"kind\":\"background\",\"name\":\"%s\",\"pid\":4242,\"id\":\"s-0001\"}}]\\n' \"$name\"\n\
                 \x20 else printf '[]\\n'; fi\n\
                 \x20 exit 0\n\
                 fi\n\
                 [ \"$1\" = stop ] && rm -f \"$state/live\"\n\
                 printf '%s\\t%s\\n' \"$(pwd -P)\" \"$*\" >> \"$state/log\"\n",
                state = dir.join("claude").display()
            ),
        )
        .expect("write a fake claude");
        fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        let case = Self { dir };
        case.accept_disclaimer();
        Some(case)
    }

    /// The case's user settings, where accepting Claude Code's bypass-permissions disclaimer
    /// interactively records `skipDangerousModePermissionPrompt: true`.
    fn user_settings(&self) -> PathBuf {
        self.dir.join("home/.claude/settings.json")
    }

    /// Records the disclaimer as accepted, as a first interactive `claude
    /// --dangerously-skip-permissions` does; every case starts with it.
    fn accept_disclaimer(&self) {
        fs::create_dir_all(self.dir.join("home/.claude")).expect("create ~/.claude");
        fs::write(
            self.user_settings(),
            "{\"skipDangerousModePermissionPrompt\": true}\n",
        )
        .expect("write ~/.claude/settings.json");
    }

    /// Removes the record of the disclaimer: a home that never ran claude interactively.
    fn withdraw_disclaimer(&self) {
        fs::remove_file(self.user_settings()).expect("remove ~/.claude/settings.json");
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    /// Pins the settings file `path` to the instance `alpha`, keeping what it holds: a role's own
    /// settings name its instance (`story:instance-isolation`).
    fn pin(&self, path: &Path) {
        let mut settings: Value = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        settings["env"]["CONDUCTOR_INSTANCE"] = Value::from("alpha");
        fs::create_dir_all(path.parent().expect("a parent")).expect("create its directory");
        fs::write(path, settings.to_string()).expect("write the pinned settings");
    }

    /// The settings a session of `alpha` named `session` starts with when its role names none:
    /// the role's default written pinned under the instance's state (`story:instance-isolation`).
    fn pinned_default(&self, session: &str) -> PathBuf {
        self.dir
            .join("home/.b10x/conductor/alpha/state/sessions")
            .join(format!("{session}.settings.json"))
    }

    /// Writes the config file of one instance `alpha` whose records are `records/`, with the
    /// role conductor on `model` and, when given, `settings`, pinned to `alpha`.
    fn config(&self, model: &str, settings: Option<&Path>) -> PathBuf {
        if let Some(path) = settings {
            self.pin(path);
        }
        let settings = settings.map_or(String::new(), |path| {
            format!(", settings: {}", path.display())
        });
        let text = format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   sources:\n\
             \x20     - github: example-org\n\
             \x20   checkouts:\n\
             \x20     root: {root}\n\
             \x20     trees: {trees}\n\
             \x20   records: {records}\n\
             \x20   roles:\n\
             \x20     - {{role: conductor, harness: claude, model: {model}{settings}}}\n\
             \x20     - {{role: conductor-dev, harness: claude, model: opus}}\n\
             \x20     - {{role: controller, harness: claude, model: opus}}\n\
             {UNBOXED}",
            root = self.dir.join("checkouts").display(),
            trees = self.dir.join("trees").display(),
            records = self.records().display(),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    /// A git repository `name` in the case's directory, made with `git init`.
    fn git_repository(&self, name: &str) -> PathBuf {
        let repo = self.dir.join(name);
        fs::create_dir_all(&repo).expect("create the repository");
        let init = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&repo)
            .env("HOME", self.dir.join("home"))
            .output()
            .expect("git runs");
        assert!(init.status.success(), "{init:?}");
        repo
    }

    /// `git status --porcelain --untracked-files=all` in `repo`.
    fn git_status(&self, repo: &Path) -> String {
        let status = Command::new("git")
            .args(["status", "--porcelain", "--untracked-files=all"])
            .current_dir(repo)
            .env("HOME", self.dir.join("home"))
            .env("GIT_CEILING_DIRECTORIES", &self.dir)
            .output()
            .expect("git runs");
        assert!(status.status.success(), "{status:?}");
        String::from_utf8_lossy(&status.stdout).into_owned()
    }

    /// `task agents:link REPO=<repo>`, isolated as [`Case::task`] is.
    fn agents_link(&self, repo: &Path) -> Output {
        self.command("agents:link", None)
            .arg(format!("REPO={}", repo.display()))
            .output()
            .expect("task runs")
    }

    /// `task <name>` on this checkout's `Taskfile.yml`, isolated as the module says.
    fn task(&self, name: &str, config: Option<&Path>) -> Output {
        self.command(name, config).output().expect("task runs")
    }

    /// The command [`Case::task`] runs, for a case that changes its environment further.
    fn command(&self, name: &str, config: Option<&Path>) -> Command {
        let mut path = vec![self.dir.join("bin")];
        path.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
        let mut command = Command::new(which("task").expect("task is on PATH"));
        command
            .arg("--taskfile")
            .arg(root().join("Taskfile.yml"))
            .arg(name)
            .current_dir(&self.dir)
            .env("HOME", self.dir.join("home"))
            .env("PATH", env::join_paths(path).expect("a PATH"))
            // The case lies inside this checkout's work tree; git looks no further up than
            // the case's directory, so a plain directory of the case is not a checkout.
            .env("GIT_CEILING_DIRECTORIES", &self.dir)
            .env_remove("CONDUCTOR_CONFIG")
            .env_remove("CONDUCTOR_INSTANCE")
            .env_remove("CFG")
            // Claude Code reads its user settings and `.claude.json` there instead of `~`.
            .env_remove("CLAUDE_CONFIG_DIR")
            .stdin(Stdio::null());
        if let Some(config) = config {
            command.env("CONDUCTOR_CONFIG", config);
        }
        command
    }

    /// The calls the fake claude logged: working directory and arguments.
    fn calls(&self) -> Vec<(PathBuf, String)> {
        fs::read_to_string(self.dir.join("claude/log"))
            .unwrap_or_default()
            .lines()
            .map(|line| {
                let (cwd, args) = line.split_once('\t').expect("a logged call");
                (PathBuf::from(cwd), args.to_owned())
            })
            .collect()
    }

    /// Puts the agent `name` where claude finds it when conductor starts in the records
    /// directory with `--setting-sources project,local`, which loads no user-level agent: the
    /// records' own `.claude/agents/<name>.md` (story:agent-needs-user-setting-source).
    fn link_agent(&self, name: &str) {
        let agents = self.records().join(".claude/agents");
        fs::create_dir_all(&agents).expect("create the records' .claude/agents");
        fs::write(agents.join(format!("{name}.md")), "an agent\n").expect("write the agent");
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

/// Its settings are this checkout's, written pinned to the instance (`story:instance-isolation`).
#[test]
fn conductor_start_runs_claude_in_the_records_directory_with_this_checkouts_settings() {
    let Some(case) = Case::new("start-default-settings") else {
        return;
    };
    case.link_agent("conductor");
    let config = case.config("sonnet", None);
    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let (cwd, args) = &calls[0];
    assert_eq!(cwd, &case.records(), "{args}");
    let settings = case.pinned_default("conductor");
    let pinned: Value =
        serde_json::from_str(&fs::read_to_string(&settings).expect("the pinned settings"))
            .expect("JSON");
    assert_eq!(pinned["env"]["CONDUCTOR_INSTANCE"], "alpha", "{pinned}");
    assert!(
        args.starts_with(&format!(
            "--bg -n conductor --agent conductor --model sonnet --permission-mode \
             bypassPermissions --setting-sources project,local --settings {} Start of session.",
            settings.display()
        )),
        "{args}"
    );
    assert!(
        args.contains("-- docs/handoff/") && !args.contains("ROOT_DIR"),
        "the prompt reads docs/handoff/ of the directory conductor runs in: {args}"
    );
}

#[test]
fn conductor_start_takes_the_settings_of_the_role_conductor() {
    let Some(case) = Case::new("start-role-settings") else {
        return;
    };
    case.link_agent("conductor");
    let settings = case.dir.join("home/settings.json");
    let config = case.config("opus", Some(&settings));
    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].0, case.records());
    assert!(
        calls[0]
            .1
            .contains(&format!("--settings {} ", settings.display())),
        "{calls:?}"
    );
}

#[test]
fn conductor_start_refuses_a_records_directory_that_is_not_there() {
    let Some(case) = Case::new("start-no-records") else {
        return;
    };
    let config = case.config("opus", None);
    fs::remove_dir(case.records()).expect("remove records/");
    let output = case.task("conductor:start", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(case.calls().is_empty(), "{:?}", case.calls());
}

#[test]
fn conductor_attach_and_restart_run_in_the_records_directory() {
    let Some(case) = Case::new("attach-restart") else {
        return;
    };
    case.link_agent("conductor");
    let config = case.config("opus", None);
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case.task("conductor:attach", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let output = case.task("conductor:restart", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));

    let calls = case.calls();
    let verbs: Vec<&str> = calls
        .iter()
        .map(|(_, args)| args.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(verbs, ["attach", "stop", "rm", "--bg"], "{calls:?}");
    assert_eq!(calls[0].1, "attach s-0001");
    assert_eq!(calls[1].1, "stop s-0001");
    assert_eq!(calls[2].1, "rm s-0001");
    for (cwd, args) in &calls {
        assert_eq!(cwd, &case.records(), "{args}");
    }
}

/// `story:conductor-start-reading`: when conductor-dev is unreachable, conductor restarts itself
/// from its records directory, naming this checkout's `Taskfile.yml`, in the form its profile
/// gives: the task stops the live conductor and starts the new one in the records directory.
#[test]
fn conductor_restart_runs_from_the_records_directory() {
    let Some(case) = Case::new("restart-from-records") else {
        return;
    };
    case.link_agent("conductor");
    let config = case.config("opus", None);
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case
        .command("conductor:restart", Some(&config))
        .current_dir(case.records())
        .output()
        .expect("task runs");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    let verbs: Vec<&str> = calls
        .iter()
        .map(|(_, args)| args.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(verbs, ["stop", "rm", "--bg"], "{calls:?}");
    for (cwd, args) in &calls {
        assert_eq!(cwd, &case.records(), "{args}");
    }

    let conductor = fs::read_to_string(root().join(".agents/conductor.md"))
        .expect("read the conductor profile");
    let conductor = conductor.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        conductor.contains("task --taskfile <product checkout>/Taskfile.yml conductor:restart"),
        "conductor.md names the form this case runs"
    );
}

#[test]
fn trust_marks_the_records_directory_and_each_checkout_trusted() {
    let Some(case) = Case::new("trust") else {
        return;
    };
    let config = case.config("opus", None);
    let checkout = case.dir.join("checkouts/alpha");
    fs::create_dir_all(&checkout).expect("create a checkout");
    let init = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&checkout)
        .env("HOME", case.dir.join("home"))
        .output()
        .expect("git runs");
    assert!(init.status.success(), "{init:?}");
    fs::create_dir_all(case.dir.join("checkouts/not-git")).expect("create a plain directory");
    let file = case.dir.join("home/.claude.json");
    fs::write(&file, "{\"projects\": {}}\n").expect("write .claude.json");

    let output = case.task("trust", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let written: Value =
        serde_json::from_str(&fs::read_to_string(&file).expect("read .claude.json"))
            .expect(".claude.json is JSON");
    let projects = written["projects"].as_object().expect("projects");
    let mut keys: Vec<&str> = projects.keys().map(String::as_str).collect();
    keys.sort_unstable();
    let checkout = checkout.display().to_string();
    let records = case.records().display().to_string();
    let mut expected = vec![checkout.as_str(), records.as_str()];
    expected.sort_unstable();
    assert_eq!(keys, expected, "{written:#}");
    for key in keys {
        assert_eq!(
            projects[key]["hasTrustDialogAccepted"],
            Value::Bool(true),
            "{key}"
        );
    }
}

#[test]
fn agents_link_links_each_adapter_into_the_repository_once_and_refuses_a_different_file() {
    let Some(case) = Case::new("agents-link") else {
        return;
    };
    let repo = case.git_repository("repo");
    let agents = repo.join(".claude/agents");
    let home = case.dir.join("home/.claude/agents");
    let exclude = repo.join(".git/info/exclude");

    // Without REPO the task refuses and writes nothing.
    let output = case.task("agents:link", None);
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("REPO=<path>"),
        "{}",
        shown(&output)
    );
    assert!(!agents.exists(), "{}", shown(&output));

    // An earlier agents:link left a link into this checkout under ~/.claude/agents/; the user
    // keeps a file of their own there too.
    fs::create_dir_all(&home).expect("create ~/.claude/agents");
    let old = home.join("conductor.md");
    std::os::unix::fs::symlink(root().join(".claude/agents/conductor.md"), &old)
        .expect("an old link into this checkout");
    let own = home.join("repo-controller.md");
    fs::write(&own, "the user's own\n").expect("write the user's file");

    // Linked into the repository and left out of its status; the old link is removed, the
    // user's file kept, and nothing is written to ~/.claude/agents/.
    let output = case.agents_link(&repo);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    for agent in AGENTS {
        let link = agents.join(format!("{agent}.md"));
        let target = fs::read_link(&link).unwrap_or_else(|error| panic!("{agent}: {error}"));
        assert_eq!(target, root().join(format!(".claude/agents/{agent}.md")));
    }
    assert_eq!(case.git_status(&repo), "", "{}", shown(&output));
    assert!(
        fs::symlink_metadata(&old).is_err(),
        "the old link into this checkout is removed: {}",
        shown(&output)
    );
    assert_eq!(fs::read_to_string(&own).expect("read"), "the user's own\n");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(&format!("note: left {}", own.display())),
        "{}",
        shown(&output)
    );
    for name in ["conductor.md", "conductor-dev.md"] {
        assert!(
            fs::symlink_metadata(home.join(name)).is_err(),
            "agents:link wrote ~/.claude/agents/{name}"
        );
    }
    let excluded = fs::read_to_string(&exclude).expect("read the exclude file");
    for agent in AGENTS {
        let line = format!(".claude/agents/{agent}.md");
        assert_eq!(
            excluded.lines().filter(|l| *l == line).count(),
            1,
            "{excluded}"
        );
    }

    // Linked again: nothing changes.
    let output = case.agents_link(&repo);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(
        fs::read_to_string(&exclude).expect("read the exclude file"),
        excluded
    );
    assert_eq!(case.git_status(&repo), "");
    for agent in AGENTS {
        let link = agents.join(format!("{agent}.md"));
        assert_eq!(
            fs::read_link(&link).expect("the link"),
            root().join(format!(".claude/agents/{agent}.md"))
        );
    }

    // A different file and a link elsewhere are refused and kept as they are.
    let file = agents.join("conductor.md");
    fs::remove_file(&file).expect("remove the link");
    fs::write(&file, "the user's own\n").expect("write a different file");
    let elsewhere = case.dir.join("home/elsewhere.md");
    fs::write(&elsewhere, "another\n").expect("write another file");
    let link = agents.join("repo-controller.md");
    fs::remove_file(&link).expect("remove the link");
    std::os::unix::fs::symlink(&elsewhere, &link).expect("link elsewhere");
    let missing = agents.join("conductor-dev.md");
    fs::remove_file(&missing).expect("remove the link");

    let output = case.agents_link(&repo);
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&file.display().to_string())
            && stderr.contains(&link.display().to_string()),
        "the refusal names each file in the way: {}",
        shown(&output)
    );
    assert_eq!(
        fs::read_to_string(&file).expect("read the file"),
        "the user's own\n"
    );
    assert!(!fs::symlink_metadata(&file).expect("the file").is_symlink());
    assert_eq!(fs::read_link(&link).expect("the link"), elsewhere);
    assert!(
        fs::symlink_metadata(&missing).is_err(),
        "nothing is linked while one file is refused"
    );
}

// ---------------------------------------------------------------------------------------------
// Adversary cases (wave 01, U2, pass 1)
// ---------------------------------------------------------------------------------------------

/// Writes a config file of one instance `alpha` whose `records` is the text `records`, written
/// as a JSON string (which YAML reads as a double-quoted scalar), and whose role conductor
/// carries `role_extra` (such as `, agent: other`).
fn adv_config(case: &Case, records: &str, role_extra: &str) -> PathBuf {
    let text = format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   sources:\n\
         \x20     - github: example-org\n\
         \x20   checkouts:\n\
         \x20     root: {root}\n\
         \x20     trees: {trees}\n\
         \x20   records: {records}\n\
         \x20   roles:\n\
         \x20     - {{role: conductor, harness: claude, model: opus{role_extra}}}\n\
         \x20     - {{role: conductor-dev, harness: claude, model: opus}}\n\
         \x20     - {{role: controller, harness: claude, model: opus}}\n\
         {UNBOXED}",
        root = case.dir.join("checkouts").display(),
        trees = case.dir.join("trees").display(),
        records = serde_json::to_string(records).expect("records as a JSON string"),
    );
    let file = case.dir.join("conductor.yaml");
    fs::write(&file, text).expect("write the config file");
    file
}

/// The agent is found in the records directory's own `.claude/agents/`, the one place
/// `--setting-sources project,local` reads it from there (story:agent-needs-user-setting-source).
#[test]
fn conductor_start_finds_the_agent_in_the_records_directory() {
    let Some(case) = Case::new("start-agent-in-records") else {
        return;
    };
    let agents = case.records().join(".claude/agents");
    fs::create_dir_all(&agents).expect("create the records' agents");
    fs::write(agents.join("conductor.md"), "an agent\n").expect("write the agent");
    let config = case.config("opus", None);
    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(case.calls().len(), 1, "{:?}", case.calls());
}

/// The refusal says why the agent is not found and names the profile as the way to start.
#[test]
fn conductor_start_without_the_agent_names_the_profile() {
    let Some(case) = Case::new("start-no-agent-message") else {
        return;
    };
    let config = case.config("opus", None);
    let output = case.task("conductor:start", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("name a profile for the role conductor"),
        "{}",
        shown(&output)
    );
}

/// `conductor:restart` refuses before it stops anything when the agent its new session would
/// start with is not found (coordinator decision 3, pass 1): the running conductor is kept.
#[test]
fn conductor_restart_stops_nothing_when_the_agent_is_not_found() {
    let Some(case) = Case::new("restart-no-agent") else {
        return;
    };
    let config = case.config("opus", None);
    fs::write(case.dir.join("claude/live"), "").expect("a live session");
    let output = case.task("conductor:restart", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("name a profile for the role conductor"),
        "{}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
    assert!(
        case.dir.join("claude/live").exists(),
        "the session was stopped"
    );
}

// ---------------------------------------------------------------------------------------------
// Sessions without the user's global config (wave 02, U1)
// ---------------------------------------------------------------------------------------------

/// The flag every session start passes so that the user's own settings (`~/.claude/settings.json`)
/// are not loaded: only the project's and the local ones, beside the `--settings` file.
const SETTING_SOURCES: &str = "--setting-sources project,local";

/// Writes a config file of one instance `alpha` whose records are the case's `records/`, whose
/// role conductor carries `conductor` and whose role conductor-dev carries `dev` (each such as
/// `, profile: /abs/p.md`).
fn w02_config(case: &Case, conductor: &str, dev: &str) -> PathBuf {
    let text = format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   sources:\n\
         \x20     - github: example-org\n\
         \x20   checkouts:\n\
         \x20     root: {root}\n\
         \x20     trees: {trees}\n\
         \x20   records: {records}\n\
         \x20   roles:\n\
         \x20     - {{role: conductor, harness: claude, model: opus{conductor}}}\n\
         \x20     - {{role: conductor-dev, harness: claude, model: sonnet{dev}}}\n\
         \x20     - {{role: controller, harness: claude, model: opus}}\n\
         {UNBOXED}",
        root = case.dir.join("checkouts").display(),
        trees = case.dir.join("trees").display(),
        records = case.records().display(),
    );
    let file = case.dir.join("conductor.yaml");
    fs::write(&file, text).expect("write the config file");
    file
}

/// Writes a profile file under the case's home and returns its path.
fn w02_profile(case: &Case, name: &str) -> PathBuf {
    let dir = case.dir.join("home/profiles");
    fs::create_dir_all(&dir).expect("create the profiles directory");
    let file = dir.join(name);
    fs::write(&file, "a profile\n").expect("write the profile");
    file
}

/// With a role `profile`, conductor starts with that file appended to its system prompt and no
/// `--agent`, so no adapter in `~/.claude/agents/` is needed; and it starts without the user's
/// settings, with the role's settings file (by default this checkout's).
#[test]
fn conductor_start_with_a_role_profile_appends_it_and_passes_no_agent() {
    let Some(case) = Case::new("w02-start-profile") else {
        return;
    };
    let profile = w02_profile(&case, "conductor.md");
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");
    assert!(!case.dir.join("home/.claude/agents/conductor.md").exists());

    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let (cwd, args) = &calls[0];
    assert_eq!(cwd, &case.records(), "{args}");
    assert!(
        args.starts_with(&format!(
            "--bg -n conductor --append-system-prompt-file {} --model opus --permission-mode \
             bypassPermissions {SETTING_SOURCES} --settings {} Start of session.",
            profile.display(),
            case.pinned_default("conductor").display()
        )),
        "{args}"
    );
    assert!(!args.contains("--agent"), "{args}");
}

/// A role `profile` that is not a file is refused before anything starts, by its path, even when
/// the agent is linked.
#[test]
fn conductor_start_refuses_a_role_profile_that_is_not_a_file() {
    let Some(case) = Case::new("w02-start-profile-missing") else {
        return;
    };
    case.link_agent("conductor");
    let profile = case.dir.join("home/profiles/missing.md");
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");

    let output = case.task("conductor:start", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&profile.display().to_string()),
        "the refusal names the profile: {}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
}

/// Without a profile, conductor keeps `--agent` and still starts without the user's settings.
#[test]
fn conductor_start_without_a_profile_keeps_the_agent_and_drops_user_settings() {
    let Some(case) = Case::new("w02-start-no-profile") else {
        return;
    };
    case.link_agent("conductor");
    let settings = case.dir.join("home/conductor-settings.json");
    case.pin(&settings);
    let config = w02_config(&case, &format!(", settings: {}", settings.display()), "");

    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let args = &calls[0].1;
    assert!(
        args.starts_with(&format!(
            "--bg -n conductor --agent conductor --model opus --permission-mode \
             bypassPermissions {SETTING_SOURCES} --settings {} Start of session.",
            settings.display()
        )),
        "{args}"
    );
    assert!(!args.contains("--append-system-prompt-file"), "{args}");
}

/// `conductor:restart` with a role profile needs no linked agent: it stops the running conductor
/// and starts one with the profile.
#[test]
fn conductor_restart_with_a_role_profile_needs_no_agent() {
    let Some(case) = Case::new("w02-restart-profile") else {
        return;
    };
    let profile = w02_profile(&case, "conductor.md");
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case.task("conductor:restart", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    let verbs: Vec<&str> = calls
        .iter()
        .map(|(_, args)| args.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(verbs, ["stop", "rm", "--bg"], "{calls:?}");
    let start = &calls[2].1;
    assert!(
        start.contains(&format!(
            "--append-system-prompt-file {} ",
            profile.display()
        )) && start.contains(SETTING_SOURCES)
            && !start.contains("--agent"),
        "{start}"
    );
}

/// `conductor:restart` stops nothing when the role profile its new session would start with is
/// not a file.
#[test]
fn conductor_restart_stops_nothing_when_the_role_profile_is_not_a_file() {
    let Some(case) = Case::new("w02-restart-profile-missing") else {
        return;
    };
    case.link_agent("conductor");
    let profile = case.dir.join("home/profiles/missing.md");
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case.task("conductor:restart", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&profile.display().to_string()),
        "the refusal names the profile: {}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
    assert!(
        case.dir.join("claude/live").exists(),
        "the session was stopped"
    );
}

/// Without a settings file or profile on the role conductor-dev, conductor-dev starts in this
/// checkout as the agent conductor-dev, without the user's settings, and with settings that only
/// pin its instance (`story:instance-isolation`).
#[test]
fn dev_start_drops_user_settings() {
    let Some(case) = Case::new("w02-dev-start") else {
        return;
    };
    let config = w02_config(&case, "", "");

    let output = case.task("dev:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let (cwd, args) = &calls[0];
    assert_eq!(cwd, &root(), "{args}");
    assert!(
        args.starts_with(&format!(
            "--bg -n conductor-dev --agent conductor-dev --model sonnet --permission-mode \
             bypassPermissions {SETTING_SOURCES} --settings {} Start of session.",
            case.pinned_default("conductor-dev").display()
        )),
        "{args}"
    );
}

/// The role conductor-dev's `settings` and `profile` reach its start command: the profile in
/// place of `--agent`.
#[test]
fn dev_start_takes_the_settings_and_profile_of_the_role_conductor_dev() {
    let Some(case) = Case::new("w02-dev-start-role") else {
        return;
    };
    let profile = w02_profile(&case, "conductor-dev.md");
    let settings = case.dir.join("home/dev-settings.json");
    case.pin(&settings);
    let config = w02_config(
        &case,
        "",
        &format!(
            ", settings: {}, profile: {}",
            settings.display(),
            profile.display()
        ),
    );

    let output = case.task("dev:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let args = &calls[0].1;
    assert!(
        args.starts_with(&format!(
            "--bg -n conductor-dev --append-system-prompt-file {} --model sonnet \
             --permission-mode bypassPermissions {SETTING_SOURCES} --settings {} Start of session.",
            profile.display(),
            settings.display()
        )),
        "{args}"
    );
    assert!(!args.contains("--agent"), "{args}");
}

/// A conductor-dev profile that is not a file is refused before anything starts.
#[test]
fn dev_start_refuses_a_role_profile_that_is_not_a_file() {
    let Some(case) = Case::new("w02-dev-start-profile-missing") else {
        return;
    };
    let profile = case.dir.join("home/profiles/missing.md");
    let config = w02_config(&case, "", &format!(", profile: {}", profile.display()));

    let output = case.task("dev:start", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&profile.display().to_string()),
        "the refusal names the profile: {}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
}

/// `conductor:start` now runs claude outside this checkout, where the project-level
/// `.claude/agents/conductor.md` is not found; `--agent conductor` resolves only when the
/// records carry their own adapter (`--setting-sources project,local` loads no user-level
/// agent, story:agent-needs-user-setting-source). Without it, start must
/// refuse rather than start a bypass-permissions session without conductor's agent; this is the
/// first `conductor:restart` after the merge, which conductor-dev runs on its own.
#[test]
fn adv_conductor_start_refuses_when_the_conductor_agent_is_not_found_from_the_records() {
    let Some(case) = Case::new("adv-start-no-agent") else {
        return;
    };
    let config = case.config("opus", None);
    assert!(!case.dir.join("home/.claude/agents/conductor.md").exists());
    assert!(!case.records().join(".claude/agents/conductor.md").exists());

    let output = case.task("conductor:start", Some(&config));
    assert_ne!(
        output.status.code(),
        Some(0),
        "started with --agent conductor where no conductor agent is found: {}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
}

/// The specification's `conductor.config.Role`: "the harness agent the role starts with". The
/// task reads the role's model and settings and ignores its `agent`.
#[test]
fn adv_conductor_start_uses_the_agent_the_role_conductor_names() {
    let Some(case) = Case::new("adv-start-role-agent") else {
        return;
    };
    case.link_agent("alt-conductor");
    let records = case.records().display().to_string();
    let config = adv_config(&case, &records, ", agent: alt-conductor");

    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(
        calls[0].1.contains("--agent alt-conductor "),
        "the role conductor names the agent alt-conductor: {calls:?}"
    );
}

/// The refusal of `agents:link` is all or nothing: when only the last adapter is in the way,
/// the first two are not linked either. The unit's own case puts the conflict on the first
/// adapter, where `ln -s` failing on it stops the loop anyway, so it stays green with the
/// `[ "$refused" = 0 ] || exit 1` gate deleted; this one does not.
#[test]
fn adv_agents_link_links_nothing_when_only_the_last_adapter_is_in_the_way() {
    let Some(case) = Case::new("adv-agents-link-last") else {
        return;
    };
    let repo = case.git_repository("repo");
    let agents = repo.join(".claude/agents");
    fs::create_dir_all(&agents).expect("create REPO/.claude/agents");
    let last = agents.join("conductor-dev.md");
    fs::write(&last, "the user's own\n").expect("write a different file");

    let output = case.agents_link(&repo);
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    for agent in ["conductor", "repo-controller"] {
        assert!(
            fs::symlink_metadata(agents.join(format!("{agent}.md"))).is_err(),
            "{agent}.md was linked while conductor-dev.md is refused: {}",
            shown(&output)
        );
    }
    assert_eq!(fs::read_to_string(&last).expect("read"), "the user's own\n");
}

/// A records path with a space, a quote and a command substitution reaches the shell as one
/// word: claude starts in that directory and nothing runs.
#[test]
fn adv_conductor_start_quotes_a_records_path_with_shell_metacharacters() {
    let Some(case) = Case::new("adv-start-metachar") else {
        return;
    };
    let records = case.dir.join("rec ords 'q' $(touch pwned) `touch pwned2`");
    fs::create_dir_all(records.join(".claude/agents")).expect("create the records directory");
    fs::write(records.join(".claude/agents/conductor.md"), "an agent\n").expect("the agent");
    let config = adv_config(&case, &records.display().to_string(), "");

    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].0, records, "{calls:?}");
    for planted in ["pwned", "pwned2"] {
        assert!(!case.dir.join(planted).exists(), "{planted} ran");
        assert!(!root().join(planted).exists(), "{planted} ran");
    }
}

// ---------------------------------------------------------------------------------------------
// Adversary cases (wave 01, U2, pass 2)
// ---------------------------------------------------------------------------------------------

/// Writes a config file of one instance `alpha` whose checkouts root is the text `checkouts`,
/// whose records are the case's `records/`, and whose role conductor is `{role: conductor,
/// <conductor>}`.
fn adv2_config(case: &Case, checkouts: &str, conductor: &str) -> PathBuf {
    let text = format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   sources:\n\
         \x20     - github: example-org\n\
         \x20   checkouts:\n\
         \x20     root: {checkouts}\n\
         \x20     trees: {trees}\n\
         \x20   records: {records}\n\
         \x20   roles:\n\
         \x20     - {{role: conductor, {conductor}}}\n\
         \x20     - {{role: conductor-dev, harness: claude, model: opus}}\n\
         \x20     - {{role: controller, harness: claude, model: opus}}\n\
         {UNBOXED}",
        checkouts = serde_json::to_string(checkouts).expect("checkouts as a JSON string"),
        trees = case.dir.join("trees").display(),
        records = case.records().display(),
    );
    let file = case.dir.join("conductor.yaml");
    fs::write(&file, text).expect("write the config file");
    file
}

/// The config lets a role's `agent` hold `.` and `/`, so `../../stray` is a valid agent. The
/// precondition tests `"$HOME"/.claude/agents/../../stray.md`, which is `~/stray.md`: a file
/// outside `~/.claude/agents/` satisfies the check whose message says the agent is in
/// `~/.claude/agents/`, and start runs `claude --agent ../../stray`.
#[test]
fn adv2_conductor_start_refuses_an_agent_found_only_outside_the_agents_directories() {
    let Some(case) = Case::new("adv2-start-agent-climbs-out") else {
        return;
    };
    fs::write(case.dir.join("home/stray.md"), "not an agent\n").expect("write a stray file");
    let config = adv2_config(
        &case,
        &case.dir.join("checkouts").display().to_string(),
        "harness: claude, model: opus, agent: ../../stray",
    );

    let output = case.task("conductor:start", Some(&config));
    assert_ne!(
        output.status.code(),
        Some(0),
        "started with an agent in neither agents directory: {}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
}

/// `conductor:restart` checks the agent before it stops (pass 1, decision 3) but not the other
/// preconditions of the `conductor:start` it ends with: with the role conductor on another
/// harness, it stops and removes the running conductor and then `conductor:start` refuses, so
/// no conductor runs.
#[test]
fn adv2_conductor_restart_stops_nothing_when_start_would_refuse() {
    let Some(case) = Case::new("adv2-restart-start-refuses") else {
        return;
    };
    case.link_agent("conductor");
    let config = adv2_config(
        &case,
        &case.dir.join("checkouts").display().to_string(),
        "harness: codex, model: opus",
    );
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case.task("conductor:restart", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        case.calls().is_empty(),
        "restart stopped the running conductor and started none: {:?}\n{}",
        case.calls(),
        shown(&output)
    );
    assert!(
        case.dir.join("claude/live").exists(),
        "the session was stopped"
    );
}

/// `conductor:restart` keeps its own copy of the `AGENT` variable for its pre-stop check, and no
/// case runs it with a role agent other than the default: with `AGENT` there replaced by the
/// constant `conductor`, the suite stays green. The role conductor names `alt-conductor`, only
/// `conductor.md` is linked: restart must stop nothing.
#[test]
fn adv2_conductor_restart_checks_the_agent_the_role_conductor_names() {
    let Some(case) = Case::new("adv2-restart-role-agent") else {
        return;
    };
    case.link_agent("conductor");
    let config = adv2_config(
        &case,
        &case.dir.join("checkouts").display().to_string(),
        "harness: claude, model: opus, agent: alt-conductor",
    );
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case.task("conductor:restart", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("alt-conductor"),
        "the refusal names the role's agent: {}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
    assert!(
        case.dir.join("claude/live").exists(),
        "the session was stopped"
    );
}

// ---------------------------------------------------------------------------------------------
// The dashboard without systemd (wave 02, U6, `story:portable-dashboard-service`)
// ---------------------------------------------------------------------------------------------

/// The programs the dashboard tasks and the fake `conductor` run, besides `conductor` itself.
/// `systemctl` is not among them.
const DASHBOARD_TOOLS: [&str; 9] = [
    "sh", "jq", "nohup", "ps", "cat", "mkdir", "rm", "sleep", "grep",
];

/// The pid file and log of the dashboard in the state directory `conductor config show` names
/// for the case's instance (`~/.b10x/conductor/alpha/state` by default).
fn dashboard_files(case: &Case) -> (PathBuf, PathBuf) {
    let state = case.dir.join("home/.b10x/conductor/alpha/state");
    (state.join("dashboard.pid"), state.join("dashboard.log"))
}

/// Replaces the case's `conductor` with one that logs a `dashboard ...` call to
/// `claude/conductor-log` and then runs until it is killed, without binding a port, and passes
/// every other call to the built `conductor`.
fn fake_dashboard_conductor(case: &Case) {
    let conductor = case.dir.join("bin/conductor");
    fs::remove_file(&conductor).expect("remove the linked conductor");
    fs::write(
        &conductor,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = dashboard ]; then\n\
             \x20 printf '%s\\n' \"$*\" >> '{log}'\n\
             \x20 while :; do sleep 1; done\n\
             fi\n\
             exec '{real}' \"$@\"\n",
            log = case.dir.join("claude/conductor-log").display(),
            real = Path::new(common::conductor(case.dir.join("home")).get_program()).display(),
        ),
    )
    .expect("write a fake conductor");
    fs::set_permissions(&conductor, fs::Permissions::from_mode(0o755)).expect("make it runnable");
}

/// A `PATH` of one directory, `nosystemd/`, holding links to [`DASHBOARD_TOOLS`] and the case's
/// `conductor`: no `systemctl`, whatever the host has.
fn path_without_systemctl(case: &Case) -> PathBuf {
    let dir = case.dir.join("nosystemd");
    fs::create_dir_all(&dir).expect("create nosystemd/");
    for tool in DASHBOARD_TOOLS {
        let found = which(tool).unwrap_or_else(|| panic!("{tool} is not on PATH"));
        std::os::unix::fs::symlink(found, dir.join(tool)).expect("link a tool");
    }
    std::os::unix::fs::symlink(case.dir.join("bin/conductor"), dir.join("conductor"))
        .expect("link conductor");
    assert!(!dir.join("systemctl").exists());
    dir
}

/// The calls the fake conductor logged for `dashboard`.
fn dashboard_calls(case: &Case) -> Vec<String> {
    fs::read_to_string(case.dir.join("claude/conductor-log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Whether `pid` is a process that has not exited (a zombie has).
fn running(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
        stat.rsplit_once(") ")
            .is_some_and(|(_, rest)| !rest.starts_with('Z'))
    })
}

/// Kills the process `pid` when dropped, so a failed case leaves no fake dashboard running.
struct Reap(u32);

impl Drop for Reap {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .args(["-9", &self.0.to_string()])
            .stderr(Stdio::null())
            .status();
    }
}

/// Where `systemctl` is not on `PATH`, `task dashboard` starts `conductor dashboard serve` in the
/// background with its pid and log in the instance's state directory, refuses a second start while
/// that pid runs, and `task dashboard:stop` stops it and removes the pid file.
#[test]
fn dashboard_without_systemctl_runs_in_the_background_with_its_pid_in_the_state_directory() {
    let Some(case) = Case::new("w02-dashboard-no-systemd") else {
        return;
    };
    let config = case.config("opus", None);
    fake_dashboard_conductor(&case);
    let path = path_without_systemctl(&case);
    let (pid_file, log) = dashboard_files(&case);

    let output = case
        .command("dashboard", Some(&config))
        .env("PATH", &path)
        .output()
        .expect("task runs");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("http://127.0.0.1:7313/"),
        "{}",
        shown(&output)
    );
    let pid: u32 = fs::read_to_string(&pid_file)
        .unwrap_or_else(|error| panic!("{}: {error}\n{}", pid_file.display(), shown(&output)))
        .trim()
        .parse()
        .expect("the pid file holds a pid");
    let _reap = Reap(pid);
    assert!(running(pid), "the dashboard {pid} is not running");
    assert!(
        log.is_file(),
        "no log beside the pid file: {}",
        log.display()
    );
    let mut calls = dashboard_calls(&case);
    for _ in 0..50 {
        if !calls.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        calls = dashboard_calls(&case);
    }
    assert_eq!(calls, ["dashboard serve --port 7313"], "{}", shown(&output));

    // A second start while that pid runs is refused and starts nothing.
    let output = case
        .command("dashboard", Some(&config))
        .env("PATH", &path)
        .output()
        .expect("task runs");
    // A second dashboard that did start would write its pid over the first one's.
    let _reap_second = fs::read_to_string(&pid_file)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
        .filter(|second| *second != pid)
        .map(Reap);
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&pid.to_string()),
        "the refusal names the running pid: {}",
        shown(&output)
    );
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(
        dashboard_calls(&case).len(),
        1,
        "a second dashboard started"
    );
    assert_eq!(
        fs::read_to_string(&pid_file).expect("the pid file").trim(),
        pid.to_string()
    );

    let output = case
        .command("dashboard:stop", Some(&config))
        .env("PATH", &path)
        .output()
        .expect("task runs");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(!pid_file.exists(), "the pid file is still there");
    let mut alive = running(pid);
    for _ in 0..50 {
        if !alive {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        alive = running(pid);
    }
    assert!(
        !alive,
        "the dashboard {pid} still runs after dashboard:stop"
    );
}

/// A pid file whose process has gone does not stop a new start, and `dashboard:stop` on it only
/// removes the file.
#[test]
fn dashboard_without_systemctl_replaces_a_stale_pid_file() {
    let Some(case) = Case::new("w02-dashboard-stale-pid") else {
        return;
    };
    let config = case.config("opus", None);
    fake_dashboard_conductor(&case);
    let path = path_without_systemctl(&case);
    let (pid_file, _) = dashboard_files(&case);
    fs::create_dir_all(pid_file.parent().expect("the state directory")).expect("create state/");
    // A process that ran and exited: its pid is free.
    let gone = Command::new("true").spawn().expect("run true");
    let gone_pid = gone.id();
    let mut gone = gone;
    gone.wait().expect("true exits");
    fs::write(&pid_file, format!("{gone_pid}\n")).expect("write a stale pid file");

    let output = case
        .command("dashboard:stop", Some(&config))
        .env("PATH", &path)
        .output()
        .expect("task runs");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(!pid_file.exists(), "the stale pid file is still there");

    fs::write(&pid_file, format!("{gone_pid}\n")).expect("write a stale pid file");
    let output = case
        .command("dashboard", Some(&config))
        .env("PATH", &path)
        .output()
        .expect("task runs");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let pid: u32 = fs::read_to_string(&pid_file)
        .expect("the pid file")
        .trim()
        .parse()
        .expect("a pid");
    let _reap = Reap(pid);
    assert_ne!(pid, gone_pid, "{}", shown(&output));
    assert!(running(pid), "the new dashboard {pid} is not running");
}

/// Where `systemctl` is on `PATH`, the dashboard stays the user service `conductor-dashboard`: no
/// pid file. The case's `systemctl` and `systemd-run` only log, so no real unit is touched.
#[test]
fn dashboard_with_systemctl_keeps_the_user_service() {
    let Some(case) = Case::new("w02-dashboard-systemd") else {
        return;
    };
    let config = case.config("opus", None);
    let log = case.dir.join("claude/systemd-log");
    // `is-active` answers "inactive" (3), so the task starts the unit.
    for tool in ["systemctl", "systemd-run"] {
        let file = case.dir.join("bin").join(tool);
        fs::write(
            &file,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"{tool} $*\" >> '{}'\n\
                 case \"$*\" in *is-active*) exit 3 ;; esac\nexit 0\n",
                log.display()
            ),
        )
        .expect("write a fake systemd tool");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    let output = case.task("dashboard", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let output = case.task("dashboard:stop", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));

    let calls = fs::read_to_string(&log).expect("the systemd log");
    let calls: Vec<&str> = calls.lines().collect();
    assert_eq!(calls.len(), 3, "{calls:?}");
    assert_eq!(
        calls[0],
        "systemctl --user is-active --quiet conductor-dashboard"
    );
    assert!(
        calls[1].starts_with("systemd-run --user --unit conductor-dashboard --collect ")
            && calls[1].ends_with(&format!(
                "{} dashboard serve --port 7313",
                case.dir.join("bin/conductor").display()
            )),
        "{calls:?}"
    );
    assert_eq!(calls[2], "systemctl --user stop conductor-dashboard");
    assert!(!dashboard_files(&case).0.exists());
}

// ---------------------------------------------------------------------------------------------
// The bypass-permissions disclaimer (wave 02, U6, `story:first-run-bypass-disclaimer`)
// ---------------------------------------------------------------------------------------------

/// Claude Code 2.1.295 refuses `claude --bg` in bypassPermissions mode until the disclaimer is
/// accepted: `skipDangerousModePermissionPrompt: true` in the user's settings (which the
/// interactive acceptance writes), the local or `--settings` file, or the older
/// `bypassPermissionsModeAccepted: true` of `~/.claude.json`. Without any of them `conductor:start`
/// refuses before claude runs, and says how to accept.
#[test]
fn conductor_start_refuses_when_the_bypass_disclaimer_is_not_accepted() {
    let Some(case) = Case::new("w02-start-no-disclaimer") else {
        return;
    };
    case.link_agent("conductor");
    case.withdraw_disclaimer();
    let config = case.config("opus", None);

    let output = case.task("conductor:start", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    for part in [
        "claude --dangerously-skip-permissions",
        "skipDangerousModePermissionPrompt",
        "~/.claude.json",
        &case.records().display().to_string(),
        &root()
            .join(".claude/conductor-settings.json")
            .display()
            .to_string(),
    ] {
        assert!(stderr.contains(part), "{part}: {}", shown(&output));
    }
    assert!(case.calls().is_empty(), "{:?}", case.calls());
}

/// The role's settings file setting `skipDangerousModePermissionPrompt: true` is enough.
#[test]
fn conductor_start_takes_the_disclaimer_from_the_role_settings_file() {
    let Some(case) = Case::new("w02-start-disclaimer-role-settings") else {
        return;
    };
    case.link_agent("conductor");
    case.withdraw_disclaimer();
    let settings = case.dir.join("home/role-settings.json");
    fs::write(&settings, "{\"skipDangerousModePermissionPrompt\": true}\n")
        .expect("write the role settings");
    let config = case.config("opus", Some(&settings));

    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(case.calls().len(), 1, "{:?}", case.calls());

    // `false` there is not an acceptance.
    fs::write(
        &settings,
        "{\"skipDangerousModePermissionPrompt\": false}\n",
    )
    .expect("write the role settings");
    let output = case.task("conductor:start", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(case.calls().len(), 1, "{:?}", case.calls());
}

/// The older record of the acceptance, `bypassPermissionsModeAccepted: true` in `~/.claude.json`,
/// still counts (Claude Code moves it into the user settings when it next starts).
#[test]
fn conductor_start_takes_the_disclaimer_from_claude_json() {
    let Some(case) = Case::new("w02-start-disclaimer-claude-json") else {
        return;
    };
    case.link_agent("conductor");
    case.withdraw_disclaimer();
    fs::write(
        case.dir.join("home/.claude.json"),
        "{\"bypassPermissionsModeAccepted\": true}\n",
    )
    .expect("write .claude.json");
    let config = case.config("opus", None);

    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(case.calls().len(), 1, "{:?}", case.calls());
}

/// With `CLAUDE_CONFIG_DIR` set, Claude Code keeps its user settings there, and so does the check.
#[test]
fn conductor_start_takes_the_disclaimer_from_claude_config_dir() {
    let Some(case) = Case::new("w02-start-disclaimer-config-dir") else {
        return;
    };
    case.link_agent("conductor");
    case.withdraw_disclaimer();
    let dir = case.dir.join("claude-config");
    fs::create_dir_all(&dir).expect("create the config dir");
    fs::write(
        dir.join("settings.json"),
        "{\"skipDangerousModePermissionPrompt\": true}\n",
    )
    .expect("write its settings");
    let config = case.config("opus", None);

    let output = case
        .command("conductor:start", Some(&config))
        .env("CLAUDE_CONFIG_DIR", &dir)
        .output()
        .expect("task runs");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(case.calls().len(), 1, "{:?}", case.calls());
}

/// `conductor:restart` stops nothing when the start it ends with would refuse for the disclaimer.
#[test]
fn conductor_restart_stops_nothing_when_the_bypass_disclaimer_is_not_accepted() {
    let Some(case) = Case::new("w02-restart-no-disclaimer") else {
        return;
    };
    case.link_agent("conductor");
    case.withdraw_disclaimer();
    let config = case.config("opus", None);
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case.task("conductor:restart", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("claude --dangerously-skip-permissions"),
        "{}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
    assert!(
        case.dir.join("claude/live").exists(),
        "the session was stopped"
    );
}

// ---------------------------------------------------------------------------------------------
// `story:instance-session-names`: the sessions start under the instance's prefixed names
// ---------------------------------------------------------------------------------------------

/// Writes a config file of one instance `alpha` whose records are the case's `records/`, with
/// `prefix` as its `session_prefix` when one is given.
fn named_config(case: &Case, prefix: Option<&str>) -> PathBuf {
    let prefix = prefix.map_or(String::new(), |prefix| {
        format!("\x20   session_prefix: {prefix}\n")
    });
    let text = format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: alpha\n\
         {prefix}\
         \x20   sources:\n\
         \x20     - github: example-org\n\
         \x20   checkouts:\n\
         \x20     root: {root}\n\
         \x20     trees: {trees}\n\
         \x20   records: {records}\n\
         {UNBOXED}",
        root = case.dir.join("checkouts").display(),
        trees = case.dir.join("trees").display(),
        records = case.records().display(),
    );
    let file = case.dir.join("conductor.yaml");
    fs::write(&file, text).expect("write the config file");
    file
}

/// `conductor:start` and `dev:start` pass `-n <prefix>-conductor` and `-n <prefix>-conductor-dev`
/// with a prefix, and `-n conductor` and `-n conductor-dev` without one: the name each reads from
/// the role's `session_name` in `conductor config show`.
#[test]
fn the_start_tasks_name_the_session_from_config_show() {
    for (prefix, conductor, dev) in [
        (Some("a"), "a-conductor", "a-conductor-dev"),
        (None, "conductor", "conductor-dev"),
    ] {
        let Some(case) = Case::new(&format!("named-start-{}", prefix.unwrap_or("none"))) else {
            return;
        };
        case.link_agent("conductor");
        let config = named_config(&case, prefix);
        for (task, name, agent) in [
            ("conductor:start", conductor, "conductor"),
            ("dev:start", dev, "conductor-dev"),
        ] {
            let output = case.task(task, Some(&config));
            assert_eq!(output.status.code(), Some(0), "{task}: {}", shown(&output));
            let calls = case.calls();
            let args = &calls.last().expect("a start").1;
            assert!(
                args.starts_with(&format!("--bg -n {name} --agent {agent} --model opus ")),
                "{task}: {args}"
            );
        }
    }
}

/// Each start task and `conductor:restart` read the name from `conductor config show`, and use
/// no literal session name.
#[test]
fn the_session_tasks_read_the_name_from_config_show() {
    let taskfile = taskfile();
    for (task, role) in [
        ("conductor:start", "conductor"),
        ("conductor:restart", "conductor"),
        ("dev:start", "conductor-dev"),
    ] {
        let task_body = &taskfile["tasks"][task];
        assert_eq!(
            task_body["vars"]["NAME"]["sh"].as_str(),
            Some(
                format!(
                    "conductor config show --format json | jq -r '.instances[0].roles[] | \
                     select(.role == \"{role}\") | .session_name // empty'"
                )
                .as_str()
            ),
            "{task}"
        );
        let body = scalars(task_body).join("\n");
        for literal in [
            format!("-n {role} "),
            format!(".name==\"{role}\""),
            format!(".name != \"{role}\""),
            format!("--arg n {role})"),
        ] {
            assert!(!body.contains(&literal), "{task} names {literal:?}: {body}");
        }
    }
}

/// With a prefix, `conductor:start` refuses while a live session holds `<prefix>-conductor`, and
/// starts beside a live `conductor` that is not this instance's.
#[test]
fn conductor_start_with_a_prefix_refuses_only_its_own_name() {
    let Some(case) = Case::new("named-start-running") else {
        return;
    };
    case.link_agent("conductor");
    let config = named_config(&case, Some("a"));
    fs::write(case.dir.join("claude/live"), "a-conductor").expect("a live a-conductor");
    let output = case.task("conductor:start", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("a-conductor"),
        "the refusal names the session: {}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());

    fs::write(case.dir.join("claude/live"), "conductor").expect("a live conductor");
    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(case.calls().len(), 1, "{:?}", case.calls());
}

/// With a prefix, `conductor:restart` stops the live `<prefix>-conductor` and starts it again
/// under that name; a live `conductor` of no prefix is not this instance's, and is left running.
#[test]
fn conductor_restart_with_a_prefix_stops_only_its_own_conductor() {
    let Some(case) = Case::new("named-restart") else {
        return;
    };
    case.link_agent("conductor");
    let config = named_config(&case, Some("a"));
    fs::write(case.dir.join("claude/live"), "conductor").expect("a live conductor");
    let output = case.task("conductor:restart", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("a-conductor"),
        "the refusal names the session: {}",
        shown(&output)
    );
    assert!(case.calls().is_empty(), "{:?}", case.calls());
    assert!(
        case.dir.join("claude/live").exists(),
        "conductor was stopped"
    );

    fs::write(case.dir.join("claude/live"), "a-conductor").expect("a live a-conductor");
    let output = case.task("conductor:restart", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    let verbs: Vec<&str> = calls
        .iter()
        .map(|(_, args)| args.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(verbs, ["stop", "rm", "--bg"], "{calls:?}");
    assert_eq!(calls[0].1, "stop s-0001");
    assert!(calls[2].1.starts_with("--bg -n a-conductor "), "{calls:?}");
}

// ---------------------------------------------------------------------------------------------
// The start form of each role under `--setting-sources project,local`
// (story:agent-needs-user-setting-source)
// ---------------------------------------------------------------------------------------------

/// `--setting-sources project,local` loads no user-level agent, so `--agent <name>` resolves only
/// from the start directory's own `.claude/agents/`. The choice the tasks hold: a role that names
/// a `profile` starts with `--append-system-prompt-file` and no `--agent`
/// (`conductor_start_with_a_role_profile_appends_it_and_passes_no_agent`,
/// `dev_start_takes_the_settings_and_profile_of_the_role_conductor_dev`); without one,
/// `conductor:start` and `conductor:restart` start `--agent <agent>` only when the records
/// directory, where they start, holds `.claude/agents/<agent>.md`, and refuse an agent found only
/// in `~/.claude/agents/`; `dev:start` starts `--agent conductor-dev` in this checkout, which
/// holds `.claude/agents/conductor-dev.md`.
#[test]
fn an_agent_start_needs_the_adapter_in_the_start_directory() {
    let dev = &taskfile()["tasks"]["dev:start"];
    assert!(dev["dir"].is_null(), "dev:start starts in this checkout");
    for command in shell_commands(dev) {
        assert!(
            !command.contains("cd "),
            "dev:start starts in this checkout: {command}"
        );
        assert!(
            command.contains("conductor session start --role conductor-dev"),
            "dev:start starts through the binary, which picks the adapter or the profile: {command}"
        );
    }
    assert!(
        root().join(".claude/agents/conductor-dev.md").is_file(),
        "this checkout holds the conductor-dev adapter"
    );

    for task in ["conductor:start", "conductor:restart"] {
        let Some(case) = Case::new(&format!("user-agent-only-{}", task.replace(':', "-"))) else {
            return;
        };
        let agents = case.dir.join("home/.claude/agents");
        fs::create_dir_all(&agents).expect("create ~/.claude/agents");
        fs::write(agents.join("conductor.md"), "an agent\n").expect("write the agent");
        let config = case.config("opus", None);
        if task == "conductor:restart" {
            fs::write(case.dir.join("claude/live"), "").expect("a live session");
        }
        let output = case.task(task, Some(&config));
        assert_ne!(
            output.status.code(),
            Some(0),
            "{task} started --agent conductor found only in ~/.claude/agents/: {}",
            shown(&output)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--setting-sources project,local"),
            "{task}: the refusal says why: {}",
            shown(&output)
        );
        assert!(case.calls().is_empty(), "{task}: {:?}", case.calls());
    }
}

/// Every text scalar of `value`, depth first, as written.
fn scalars(value: &serde_yaml::Value) -> Vec<String> {
    match value {
        serde_yaml::Value::String(text) => vec![text.clone()],
        serde_yaml::Value::Sequence(items) => items.iter().flat_map(scalars).collect(),
        serde_yaml::Value::Mapping(map) => map.values().flat_map(scalars).collect(),
        _ => Vec::new(),
    }
}

/// The conductor profile starts sessions only through the binary (`story:session-envelope-seam`):
/// `conductor spawn` for a controller, `conductor session start` for conductor and conductor-dev,
/// so the session name comes from the instance's config inside the binary and no raw `claude --bg`
/// form, which could name a session after the bare repository, is left in the profile.
#[test]
fn the_conductor_profile_names_controllers_by_their_session_name() {
    let text = fs::read_to_string(root().join(".agents/conductor.md")).expect("read the profile");
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !flat.contains("claude --bg"),
        "the profile starts no session with a raw claude form"
    );
    for form in [
        "`conductor spawn --repository <repo>`",
        "`conductor session start --role <role>`",
    ] {
        assert!(
            flat.contains(form),
            "the profile names the start form {form}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// `story:restart-current-profile`: a start refuses a profile checkout behind its upstream
// ---------------------------------------------------------------------------------------------

/// Runs git in `dir` with no configuration of the user's and answers its standard output; a
/// failing git fails the case.
fn fixture_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {}: {}",
        args.join(" "),
        shown(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// What the upstream of a profile checkout holds.
#[derive(Clone, Copy)]
enum Upstream {
    /// No upstream: the checkout tracks nothing.
    Absent,
    /// The commit the checkout holds.
    Even,
    /// One commit more, pushed from another clone, which the checkout has not fetched.
    Ahead,
}

/// A git checkout `product/` of the case holding the committed profile `.agents/conductor.md`;
/// with an upstream, its branch `main` tracks `origin/main` of the bare `upstream.git` of the
/// case. Answers the checkout and the profile.
fn profile_checkout(case: &Case, upstream: Upstream) -> (PathBuf, PathBuf) {
    let checkout = case.dir.join("product");
    let profile = checkout.join(".agents/conductor.md");
    fs::create_dir_all(checkout.join(".agents")).expect("create the checkout");
    fixture_git(&checkout, &["init", "--quiet"]);
    fs::write(&profile, "a profile\n").expect("write the profile");
    fixture_git(&checkout, &["add", "."]);
    fixture_git(&checkout, &["commit", "--quiet", "-m", "profile"]);
    if let Upstream::Absent = upstream {
        return (checkout, profile);
    }
    fixture_git(&case.dir, &["init", "--quiet", "--bare", "upstream.git"]);
    let bare = case.dir.join("upstream.git").display().to_string();
    fixture_git(&checkout, &["remote", "add", "origin", &bare]);
    fixture_git(&checkout, &["push", "--quiet", "-u", "origin", "main"]);
    if let Upstream::Ahead = upstream {
        fixture_git(&case.dir, &["clone", "--quiet", &bare, "other"]);
        let other = case.dir.join("other");
        fs::write(other.join(".agents/conductor.md"), "a newer profile\n")
            .expect("write the newer profile");
        fixture_git(&other, &["commit", "--quiet", "-am", "newer profile"]);
        fixture_git(&other, &["push", "--quiet", "origin", "main"]);
    }
    (checkout, profile)
}

/// Asserts that `output` is a refusal naming the checkout and the command that fast-forwards it.
fn assert_refused_behind(output: &Output, checkout: &Path) {
    assert_ne!(output.status.code(), Some(0), "{}", shown(output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("git -C {} merge --ff-only", checkout.display())),
        "the refusal names the checkout and the fast-forward: {}",
        shown(output)
    );
}

/// The profile's checkout is behind its upstream once fetched: `conductor:start` refuses, names
/// the checkout and `git -C <checkout> merge --ff-only`, and starts nothing.
#[test]
fn conductor_start_refuses_a_profile_checkout_behind_its_upstream() {
    let Some(case) = Case::new("start-profile-behind") else {
        return;
    };
    let (checkout, profile) = profile_checkout(&case, Upstream::Ahead);
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");

    let output = case.task("conductor:start", Some(&config));
    assert_refused_behind(&output, &checkout);
    assert!(case.calls().is_empty(), "{:?}", case.calls());
}

/// `conductor:restart` stops nothing when the profile's checkout is behind its upstream: the
/// running conductor keeps running and no new one starts.
#[test]
fn conductor_restart_stops_nothing_when_the_profile_checkout_is_behind() {
    let Some(case) = Case::new("restart-profile-behind") else {
        return;
    };
    let (checkout, profile) = profile_checkout(&case, Upstream::Ahead);
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case.task("conductor:restart", Some(&config));
    assert_refused_behind(&output, &checkout);
    assert!(case.calls().is_empty(), "{:?}", case.calls());
    assert!(
        case.dir.join("claude/live").exists(),
        "the session was stopped"
    );
}

/// A fetch that fails does not block the check: the checkout is compared with the upstream ref
/// it already has, which is ahead of it, so `conductor:start` refuses.
#[test]
fn conductor_start_refuses_a_checkout_behind_the_ref_it_has_when_the_fetch_fails() {
    let Some(case) = Case::new("start-profile-behind-no-fetch") else {
        return;
    };
    let (checkout, profile) = profile_checkout(&case, Upstream::Ahead);
    fixture_git(&checkout, &["fetch", "--quiet"]);
    fs::remove_dir_all(case.dir.join("upstream.git")).expect("remove the upstream");
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");

    let output = case.task("conductor:start", Some(&config));
    assert_refused_behind(&output, &checkout);
    assert!(case.calls().is_empty(), "{:?}", case.calls());
}

/// The profile's checkout is even with its upstream: `conductor:start` starts with the profile.
#[test]
fn conductor_start_starts_from_a_profile_checkout_even_with_its_upstream() {
    let Some(case) = Case::new("start-profile-even") else {
        return;
    };
    let (_, profile) = profile_checkout(&case, Upstream::Even);
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");

    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(
        calls[0].1.starts_with(&format!(
            "--bg -n conductor --append-system-prompt-file {} ",
            profile.display()
        )),
        "{calls:?}"
    );
}

/// The profile's checkout is even with its upstream: `conductor:restart` stops the running
/// conductor and starts a new one.
#[test]
fn conductor_restart_restarts_from_a_profile_checkout_even_with_its_upstream() {
    let Some(case) = Case::new("restart-profile-even") else {
        return;
    };
    let (_, profile) = profile_checkout(&case, Upstream::Even);
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case.task("conductor:restart", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    let verbs: Vec<&str> = calls
        .iter()
        .map(|(_, args)| args.split(' ').next().unwrap_or_default())
        .collect();
    assert_eq!(verbs, ["stop", "rm", "--bg"], "{calls:?}");
}

/// A fetch that fails does not block the start: the checkout is even with the upstream ref it
/// has, so `conductor:start` starts.
#[test]
fn conductor_start_starts_when_the_fetch_of_an_even_checkout_fails() {
    let Some(case) = Case::new("start-profile-even-no-fetch") else {
        return;
    };
    let (_, profile) = profile_checkout(&case, Upstream::Even);
    fs::remove_dir_all(case.dir.join("upstream.git")).expect("remove the upstream");
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");

    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(case.calls().len(), 1, "{:?}", case.calls());
}

/// The profile's checkout has no upstream: `conductor:start` starts as before.
#[test]
fn conductor_start_starts_from_a_profile_checkout_without_an_upstream() {
    let Some(case) = Case::new("start-profile-no-upstream") else {
        return;
    };
    let (_, profile) = profile_checkout(&case, Upstream::Absent);
    let config = w02_config(&case, &format!(", profile: {}", profile.display()), "");

    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(case.calls().len(), 1, "{:?}", case.calls());
}

// ---------------------------------------------------------------------------------------------
// `story:instance-scoped-tasks`: every task acts on the active instance's sessions only
// ---------------------------------------------------------------------------------------------

/// Writes a config file of two instances: `alpha`, `session_prefix: a`, whose records are the
/// case's `records/`, and `beta`, `session_prefix: b`, whose records are `records-b/`; neither
/// is the file's default, so each task runs the one `CONDUCTOR_INSTANCE` names. Neither writes a
/// `dashboard_port`: alpha's is 7313 and beta's 7314.
fn two_instance_config(case: &Case) -> PathBuf {
    fs::create_dir_all(case.dir.join("records-b")).expect("create records-b/");
    let text = format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   session_prefix: a\n\
         \x20   sources:\n\
         \x20     - github: example-org\n\
         \x20   checkouts:\n\
         \x20     root: {root}\n\
         \x20     trees: {trees}\n\
         \x20   records: {records}\n\
         {UNBOXED}\
         \x20 - name: beta\n\
         \x20   session_prefix: b\n\
         \x20   sources:\n\
         \x20     - github: example-org\n\
         \x20   checkouts:\n\
         \x20     root: {root}-b\n\
         \x20     trees: {trees}-b\n\
         \x20   records: {records}-b\n\
         {UNBOXED}",
        root = case.dir.join("checkouts").display(),
        trees = case.dir.join("trees").display(),
        records = case.records().display(),
    );
    let file = case.dir.join("conductor.yaml");
    fs::write(&file, text).expect("write the config file");
    file
}

/// Replaces the case's `claude` with one whose live background sessions are the lines `<name>
/// <id>` of `claude/sessions`, written from `sessions`: `agents --json` lists each, `stop <id>`
/// removes its line, and every other call is logged as [`Case::calls`] reads it.
fn sessions_claude(case: &Case, sessions: &[(&str, &str)]) {
    let state = case.dir.join("claude");
    write_sessions(case, sessions);
    let claude = case.dir.join("bin/claude");
    fs::write(
        &claude,
        format!(
            "#!/bin/sh\n\
             state='{state}'\n\
             if [ \"$1\" = agents ]; then\n\
             \x20 jq -R -s -c 'split(\"\\n\") | map(select(length > 0) | split(\" \") | \
             {{kind: \"background\", name: .[0], id: .[1], pid: 4242}})' \"$state/sessions\"\n\
             \x20 exit 0\n\
             fi\n\
             if [ \"$1\" = stop ]; then\n\
             \x20 grep -v \" $2$\" \"$state/sessions\" > \"$state/sessions.new\"\n\
             \x20 mv \"$state/sessions.new\" \"$state/sessions\"\n\
             fi\n\
             printf '%s\\t%s\\n' \"$(pwd -P)\" \"$*\" >> \"$state/log\"\n",
            state = state.display()
        ),
    )
    .expect("write a fake claude");
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).expect("make it runnable");
}

/// Writes the live sessions [`sessions_claude`] lists.
fn write_sessions(case: &Case, sessions: &[(&str, &str)]) {
    let lines: String = sessions
        .iter()
        .map(|(name, id)| format!("{name} {id}\n"))
        .collect();
    fs::write(case.dir.join("claude/sessions"), lines).expect("write the live sessions");
}

/// The live sessions [`sessions_claude`] lists now, as `<name> <id>` lines.
fn live_sessions(case: &Case) -> Vec<String> {
    fs::read_to_string(case.dir.join("claude/sessions"))
        .expect("the live sessions")
        .lines()
        .map(str::to_owned)
        .collect()
}

/// `task <name>` for the instance `instance` of `config`.
fn instance_task(case: &Case, name: &str, config: &Path, instance: &str) -> Output {
    case.command(name, Some(config))
        .env("CONDUCTOR_INSTANCE", instance)
        .output()
        .expect("task runs")
}

/// No task that finds a session names one literally: each reads the names from `conductor config
/// show`, the attach tasks the role's `session_name` as the start tasks do, and `task sessions`
/// every role's.
#[test]
fn every_session_task_reads_the_names_from_config_show() {
    let taskfile = taskfile();
    for (task, role) in [
        ("conductor:attach", "conductor"),
        ("dev:attach", "conductor-dev"),
    ] {
        let task_body = &taskfile["tasks"][task];
        assert_eq!(
            task_body["vars"]["NAME"]["sh"].as_str(),
            Some(
                format!(
                    "conductor config show --format json | jq -r '.instances[0].roles[] | \
                     select(.role == \"{role}\") | .session_name // empty'"
                )
                .as_str()
            ),
            "{task}"
        );
    }
    let sessions = &taskfile["tasks"]["sessions"];
    assert_eq!(
        sessions["vars"]["NAMES"]["sh"].as_str(),
        Some(
            "conductor config show --format json | jq -c '[.instances[0].roles[] | \
             .session_name // empty]'"
        ),
        "{sessions:?}"
    );
    for task in [
        "sessions",
        "conductor:start",
        "conductor:attach",
        "conductor:restart",
        "dev:start",
        "dev:attach",
    ] {
        let body = scalars(&taskfile["tasks"][task]).join("\n");
        for role in ["conductor", "conductor-dev"] {
            for literal in [
                format!(".name==\"{role}\""),
                format!(".name == \"{role}\""),
                format!("--arg n {role})"),
                format!("--arg n {role} "),
            ] {
                assert!(!body.contains(&literal), "{task} names {literal:?}: {body}");
            }
        }
    }
}

/// Two instances in one config, each with its conductor running: `conductor:restart` of one
/// stops and restarts only its own conductor, in its own records directory, and leaves the other
/// instance's running.
#[test]
fn conductor_restart_of_one_instance_leaves_the_other_instances_conductor_running() {
    let Some(case) = Case::new("scoped-restart") else {
        return;
    };
    case.link_agent("conductor");
    let config = two_instance_config(&case);
    let agents = case.dir.join("records-b/.claude/agents");
    fs::create_dir_all(&agents).expect("create records-b/.claude/agents");
    fs::write(agents.join("conductor.md"), "an agent\n").expect("write the agent");
    sessions_claude(&case, &[("a-conductor", "s-a"), ("b-conductor", "s-b")]);

    let output = instance_task(&case, "conductor:restart", &config, "alpha");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    let args: Vec<&str> = calls.iter().map(|(_, args)| args.as_str()).collect();
    assert_eq!(args.len(), 3, "{calls:?}");
    assert_eq!(args[0], "stop s-a", "{calls:?}");
    assert_eq!(args[1], "rm s-a", "{calls:?}");
    assert!(args[2].starts_with("--bg -n a-conductor "), "{calls:?}");
    for (cwd, args) in &calls {
        assert_eq!(cwd, &case.records(), "{args}");
    }
    assert_eq!(live_sessions(&case), ["b-conductor s-b"], "{calls:?}");

    write_sessions(&case, &[("a-conductor", "s-a2"), ("b-conductor", "s-b")]);
    fs::remove_file(case.dir.join("claude/log")).expect("clear the log");
    let output = instance_task(&case, "conductor:restart", &config, "beta");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    let args: Vec<&str> = calls.iter().map(|(_, args)| args.as_str()).collect();
    assert_eq!(args.len(), 3, "{calls:?}");
    assert_eq!(args[0], "stop s-b", "{calls:?}");
    assert_eq!(args[1], "rm s-b", "{calls:?}");
    assert!(args[2].starts_with("--bg -n b-conductor "), "{calls:?}");
    for (cwd, args) in &calls {
        assert_eq!(cwd, &case.dir.join("records-b"), "{args}");
    }
    assert_eq!(live_sessions(&case), ["a-conductor s-a2"], "{calls:?}");
}

/// The attach tasks open the active instance's session, beside another instance's and a bare
/// `conductor` of none, and refuse when no live session holds its name; `task sessions` lists the
/// active instance's sessions only.
#[test]
fn the_attach_tasks_and_sessions_find_the_active_instance_s_sessions() {
    let Some(case) = Case::new("scoped-attach") else {
        return;
    };
    let config = two_instance_config(&case);
    sessions_claude(
        &case,
        &[
            ("a-conductor", "s-a"),
            ("a-conductor-dev", "s-ad"),
            ("b-conductor", "s-b"),
            ("b-conductor-dev", "s-bd"),
            ("conductor", "s-x"),
            ("conductor-dev", "s-xd"),
        ],
    );
    for (task, instance, cwd, attached) in [
        ("conductor:attach", "alpha", case.records(), "attach s-a"),
        ("dev:attach", "alpha", root(), "attach s-ad"),
        (
            "conductor:attach",
            "beta",
            case.dir.join("records-b"),
            "attach s-b",
        ),
        ("dev:attach", "beta", root(), "attach s-bd"),
    ] {
        let _ = fs::remove_file(case.dir.join("claude/log"));
        let output = instance_task(&case, task, &config, instance);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{task} {instance}: {}",
            shown(&output)
        );
        assert_eq!(
            case.calls(),
            [(cwd, attached.to_owned())],
            "{task} {instance}"
        );
    }

    let output = instance_task(&case, "sessions", &config, "alpha");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let names: Vec<&str> = stdout
        .lines()
        .map(|line| line.split('\t').next().unwrap_or_default())
        .collect();
    assert_eq!(
        names,
        ["a-conductor", "a-conductor-dev"],
        "{}",
        shown(&output)
    );

    write_sessions(&case, &[("conductor", "s-x"), ("b-conductor", "s-b")]);
    let _ = fs::remove_file(case.dir.join("claude/log"));
    for (task, name) in [
        ("conductor:attach", "a-conductor"),
        ("dev:attach", "a-conductor-dev"),
    ] {
        let output = instance_task(&case, task, &config, "alpha");
        assert_ne!(output.status.code(), Some(0), "{task}: {}", shown(&output));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains(&format!("no background session named {name}")),
            "{task}: {}",
            shown(&output)
        );
        assert!(case.calls().is_empty(), "{task}: {:?}", case.calls());
    }
}

/// A name that more than one live background session holds is refused by every task that
/// resolves it: `conductor:restart` stops nothing, the attach tasks attach nothing, and `task
/// sessions` exits 1 naming it.
#[test]
fn a_name_more_than_one_live_session_holds_is_refused() {
    let Some(case) = Case::new("scoped-ambiguous") else {
        return;
    };
    case.link_agent("conductor");
    let config = two_instance_config(&case);
    sessions_claude(
        &case,
        &[
            ("a-conductor", "s-1"),
            ("a-conductor", "s-2"),
            ("a-conductor-dev", "s-3"),
            ("a-conductor-dev", "s-4"),
        ],
    );
    let before = live_sessions(&case);
    for (task, name) in [
        ("conductor:restart", "a-conductor"),
        ("conductor:attach", "a-conductor"),
        ("dev:attach", "a-conductor-dev"),
        ("sessions", "a-conductor"),
        ("sessions", "a-conductor-dev"),
    ] {
        let output = instance_task(&case, task, &config, "alpha");
        assert_ne!(output.status.code(), Some(0), "{task}: {}", shown(&output));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(&format!(
                "more than one live background session is named {name}"
            )),
            "{task}: the refusal names {name}: {}",
            shown(&output)
        );
        assert!(case.calls().is_empty(), "{task}: {:?}", case.calls());
        assert_eq!(live_sessions(&case), before, "{task} stopped a session");
    }
}

/// Replaces the case's `conductor` with one that logs a `dashboard ...` call, after the instance
/// `CONDUCTOR_INSTANCE` names, to `claude/conductor-log` and then runs until it is killed, and
/// passes every other call to the built `conductor`.
fn fake_instance_dashboard_conductor(case: &Case) {
    let conductor = case.dir.join("bin/conductor");
    fs::remove_file(&conductor).expect("remove the linked conductor");
    fs::write(
        &conductor,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = dashboard ]; then\n\
             \x20 printf '%s %s\\n' \"$CONDUCTOR_INSTANCE\" \"$*\" >> '{log}'\n\
             \x20 while :; do sleep 1; done\n\
             fi\n\
             exec '{real}' \"$@\"\n",
            log = case.dir.join("claude/conductor-log").display(),
            real = Path::new(common::conductor(case.dir.join("home")).get_program()).display(),
        ),
    )
    .expect("write a fake conductor");
    fs::set_permissions(&conductor, fs::Permissions::from_mode(0o755)).expect("make it runnable");
}

/// The pid the pid file `file` holds.
fn pid_in(file: &Path) -> u32 {
    fs::read_to_string(file)
        .unwrap_or_else(|error| panic!("{}: {error}", file.display()))
        .trim()
        .parse()
        .expect("the pid file holds a pid")
}

/// Without `systemctl`, the second instance's dashboard starts beside the first one's on its own
/// port, serving its own instance, with its pid in its own state directory; `dashboard:stop` of
/// one leaves the other running.
#[test]
fn the_second_instance_s_dashboard_serves_its_own_port_beside_the_first() {
    let Some(case) = Case::new("scoped-dashboard") else {
        return;
    };
    let config = two_instance_config(&case);
    fake_instance_dashboard_conductor(&case);
    let path = path_without_systemctl(&case);
    let pid_file = |instance: &str| {
        case.dir.join(format!(
            "home/.b10x/conductor/{instance}/state/dashboard.pid"
        ))
    };
    let start = |instance: &str| {
        let output = case
            .command("dashboard", Some(&config))
            .env("PATH", &path)
            .env("CONDUCTOR_INSTANCE", instance)
            .output()
            .expect("task runs");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{instance}: {}",
            shown(&output)
        );
        output
    };

    let output = start("alpha");
    let alpha = pid_in(&pid_file("alpha"));
    let _reap_alpha = Reap(alpha);
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("http://127.0.0.1:7313/"),
        "{}",
        shown(&output)
    );
    let output = start("beta");
    let beta = pid_in(&pid_file("beta"));
    let _reap_beta = Reap(beta);
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("http://127.0.0.1:7314/"),
        "{}",
        shown(&output)
    );
    assert_ne!(alpha, beta);
    assert!(running(alpha), "the first dashboard {alpha} is not running");
    assert!(running(beta), "the second dashboard {beta} is not running");

    let mut calls = dashboard_calls(&case);
    for _ in 0..50 {
        if calls.len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        calls = dashboard_calls(&case);
    }
    calls.sort();
    assert_eq!(
        calls,
        [
            "alpha dashboard serve --port 7313",
            "beta dashboard serve --port 7314"
        ]
    );

    let output = case
        .command("dashboard:stop", Some(&config))
        .env("PATH", &path)
        .env("CONDUCTOR_INSTANCE", "beta")
        .output()
        .expect("task runs");
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(!pid_file("beta").exists(), "beta's pid file is still there");
    let mut alive = running(beta);
    for _ in 0..50 {
        if !alive {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        alive = running(beta);
    }
    assert!(
        !alive,
        "the second dashboard {beta} still runs after its stop"
    );
    assert!(running(alpha), "stopping beta's dashboard stopped alpha's");
    assert_eq!(pid_in(&pid_file("alpha")), alpha);
}

/// Where `systemctl` is on `PATH`, each instance's dashboard is a user service of its own,
/// `<session_prefix>-conductor-dashboard`, serving its instance on its port.
#[test]
fn each_instance_s_dashboard_service_is_its_own_unit() {
    let Some(case) = Case::new("scoped-dashboard-systemd") else {
        return;
    };
    let config = two_instance_config(&case);
    let log = case.dir.join("claude/systemd-log");
    for tool in ["systemctl", "systemd-run"] {
        let file = case.dir.join("bin").join(tool);
        fs::write(
            &file,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"{tool} $*\" >> '{}'\n\
                 case \"$*\" in *is-active*) exit 3 ;; esac\nexit 0\n",
                log.display()
            ),
        )
        .expect("write a fake systemd tool");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    for task in ["dashboard", "dashboard:stop"] {
        let output = instance_task(&case, task, &config, "beta");
        assert_eq!(output.status.code(), Some(0), "{task}: {}", shown(&output));
    }
    let calls = fs::read_to_string(&log).expect("the systemd log");
    let calls: Vec<&str> = calls.lines().collect();
    assert_eq!(calls.len(), 3, "{calls:?}");
    assert_eq!(
        calls[0],
        "systemctl --user is-active --quiet b-conductor-dashboard"
    );
    assert!(
        calls[1].starts_with("systemd-run --user --unit b-conductor-dashboard --collect ")
            && calls[1].contains(" --setenv=CONDUCTOR_INSTANCE=beta ")
            && calls[1].ends_with(&format!(
                "{} dashboard serve --port 7314",
                case.dir.join("bin/conductor").display()
            )),
        "{calls:?}"
    );
    assert_eq!(calls[2], "systemctl --user stop b-conductor-dashboard");
}

/// `story:agents-link-local`: `task agents:link` serves a session started by hand in the
/// repository it is given, never every session on the machine through `~/.claude/agents/`; the
/// sessions the tasks start pass `--setting-sources project,local` and start from their role's
/// profile, so the desc says so.
#[test]
fn agents_link_says_it_serves_a_session_started_by_hand_in_the_repository() {
    let taskfile = taskfile();
    let desc = taskfile["tasks"]["agents:link"]["desc"]
        .as_str()
        .expect("agents:link has a desc");
    assert!(
        !desc.contains("so a session started in another directory finds them"),
        "{desc}"
    );
    assert!(desc.contains("REPO/.claude/agents/"), "{desc}");
    assert!(desc.contains("task agents:link REPO=<path>"), "{desc}");
    assert!(desc.contains("a session started by hand in REPO"), "{desc}");
    assert!(
        desc.contains("writes nothing to ~/.claude/agents/"),
        "{desc}"
    );
    assert!(desc.contains("--setting-sources project,local"), "{desc}");
}

/// `story:status-board`: `task status` runs the snapshot collectors (`conductor snapshot
/// start-snapshot`) and then `conductor snapshot publish-board`.
#[test]
fn status_runs_the_collectors_and_then_publishes_the_board() {
    let taskfile = taskfile();
    let task = &taskfile["tasks"]["status"];
    assert!(!task.is_null(), "Taskfile.yml has no task status");
    let commands = shell_commands(task).join("\n");
    let collect = commands
        .find("conductor snapshot start-snapshot")
        .unwrap_or_else(|| panic!("status runs no collectors: {commands}"));
    let publish = commands
        .find("conductor snapshot publish-board")
        .unwrap_or_else(|| panic!("status publishes no board: {commands}"));
    assert!(
        collect < publish,
        "status publishes before it collects: {commands}"
    );
}

/// `task status`, run with a `conductor` that logs its arguments: it publishes the snapshot the
/// collectors took, by its id, and publishes nothing when that snapshot failed.
#[test]
fn status_publishes_the_snapshot_it_took_and_nothing_when_it_failed() {
    let Some(case) = Case::new("status-board") else {
        return;
    };
    let log = case.dir.join("claude/status-log");
    let fail = case.dir.join("claude/fail");
    let conductor = case.dir.join("bin/conductor");
    fs::remove_file(&conductor).expect("remove the linked conductor");
    fs::write(
        &conductor,
        format!(
            "#!/bin/sh\n\
             printf '%s\\n' \"$*\" >> '{log}'\n\
             if [ \"$2\" = start-snapshot ]; then\n\
             \x20 echo snap-0001\n\
             \x20 if [ -e '{fail}' ]; then exit 1; fi\n\
             fi\n\
             exit 0\n",
            log = log.display(),
            fail = fail.display(),
        ),
    )
    .expect("write a fake conductor");
    fs::set_permissions(&conductor, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    let calls = || -> Vec<String> {
        fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    };

    let output = case.task("status", None);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(
        calls(),
        [
            "snapshot start-snapshot",
            "snapshot publish-board --snapshot-id snap-0001"
        ],
        "{}",
        shown(&output)
    );

    fs::remove_file(&log).expect("clear the log");
    fs::write(&fail, "").expect("make the snapshot fail");
    let output = case.task("status", None);
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(calls(), ["snapshot start-snapshot"], "{}", shown(&output));
}

/// `story:self-arming-restart`: `conductor:restart` starts the new session through
/// `conductor:start`, whose prompt names the two recurring jobs by their cadence keys in
/// `conductor config show` and the watch command, so the session arms them without a hand-over.
#[test]
fn the_restart_prompt_names_the_recurring_jobs_and_the_watch() {
    let taskfile = taskfile();
    let restart = serde_yaml::to_string(&taskfile["tasks"]["conductor:restart"]["cmds"])
        .expect("conductor:restart cmds as text");
    assert!(
        restart.contains("task: conductor:start"),
        "conductor:restart starts the new session through conductor:start: {restart}"
    );
    // The prompt is the one `conductor session start --role conductor` passes
    // (`story:session-envelope-seam`): the last argument of the session it starts.
    let Some(case) = Case::new("self-arming-prompt") else {
        return;
    };
    case.link_agent("conductor");
    let config = case.config("opus", None);
    let output = case.task("conductor:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let at = calls[0].1.find("Start of session.").expect("the prompt");
    let prompt = &calls[0].1[at..];
    for part in [
        "CronList",
        "CronCreate",
        "cadence.cycle",
        "cadence.daily",
        "conductor config show",
        "Monitor",
        "conductor watch run --follow",
    ] {
        assert!(prompt.contains(part), "the prompt names {part}: {prompt}");
    }
    let arm = prompt.find("cadence.cycle").expect("cadence.cycle");
    let handoff = prompt.find("-- docs/handoff/").expect("the hand-over");
    assert!(
        arm < handoff,
        "the prompt arms the jobs before it reads the hand-over: {prompt}"
    );
}
