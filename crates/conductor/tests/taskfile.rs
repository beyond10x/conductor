//! `story:instance-records-start`: the tasks of `Taskfile.yml` that run conductor run it in the
//! active instance's records directory, `task trust` trusts that directory, and `task
//! agents:link` links this checkout's Claude Code adapters into `~/.claude/agents/` without
//! overwriting anything. `task trust` runs `conductor trust` (`story:portable-trust`), whose own
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
        for command in &commands {
            assert!(
                command.starts_with(CD_RECORDS),
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
    let commands = shell_commands(task).join("\n");
    for part in [
        "if [ -n {{shellQuote .PROFILE}} ]; then flag=--append-system-prompt-file; \
         who={{shellQuote .PROFILE}}; else flag=--agent; who={{shellQuote .AGENT}}; fi",
        "claude --bg -n {{shellQuote .NAME}} \"$flag\" \"$who\" ",
        "--model {{shellQuote .MODEL}}",
        "--setting-sources project,local --settings {{shellQuote .SETTINGS}}",
        "{{shellQuote .CONDUCTOR_PROMPT}}",
    ] {
        assert!(commands.contains(part), "{part}: {commands}");
    }
    assert!(
        !commands.contains("--settings .claude/"),
        "a relative settings path names the records directory's: {commands}"
    );
    let prompt = taskfile["vars"]["CONDUCTOR_PROMPT"]
        .as_str()
        .expect("a CONDUCTOR_PROMPT");
    assert!(
        prompt.contains("-- docs/handoff/") && !prompt.contains("ROOT_DIR"),
        "the prompt reads docs/handoff/ of the directory conductor runs in: {prompt}"
    );
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
    for forbidden in ["ln -sf", "ln -fs", "ln -f", "rm ", "mv ", "--force"] {
        assert!(
            !commands.contains(forbidden),
            "agents:link may overwrite ({forbidden}): {commands}"
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

    /// Writes the config file of one instance `alpha` whose records are `records/`, with the
    /// role conductor on `model` and, when given, `settings`.
    fn config(&self, model: &str, settings: Option<&Path>) -> PathBuf {
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
             \x20     - {{role: controller, harness: claude, model: opus}}\n",
            root = self.dir.join("checkouts").display(),
            trees = self.dir.join("trees").display(),
            records = self.records().display(),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
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

    /// Puts the agent `name` where claude finds it from any directory, as `task agents:link`
    /// does: `~/.claude/agents/<name>.md` of the case's home.
    fn link_agent(&self, name: &str) {
        let agents = self.dir.join("home/.claude/agents");
        fs::create_dir_all(&agents).expect("create ~/.claude/agents");
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
    let settings = root().join(".claude/conductor-settings.json");
    assert!(
        args.starts_with(&format!(
            "--bg -n conductor --agent conductor --model sonnet --permission-mode \
             bypassPermissions --setting-sources project,local --settings {} Start of session.",
            settings.display()
        )),
        "{args}"
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
fn agents_link_links_each_adapter_once_and_refuses_a_different_file() {
    let Some(case) = Case::new("agents-link") else {
        return;
    };
    let agents = case.dir.join("home/.claude/agents");

    // Linked into a home that has no ~/.claude/agents yet.
    let output = case.task("agents:link", None);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    for agent in AGENTS {
        let link = agents.join(format!("{agent}.md"));
        let target = fs::read_link(&link).unwrap_or_else(|error| panic!("{agent}: {error}"));
        assert_eq!(target, root().join(format!(".claude/agents/{agent}.md")));
    }

    // Linked again: the same links are fine.
    let output = case.task("agents:link", None);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));

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

    let output = case.task("agents:link", None);
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
         \x20     - {{role: controller, harness: claude, model: opus}}\n",
        root = case.dir.join("checkouts").display(),
        trees = case.dir.join("trees").display(),
        records = serde_json::to_string(records).expect("records as a JSON string"),
    );
    let file = case.dir.join("conductor.yaml");
    fs::write(&file, text).expect("write the config file");
    file
}

/// The agent is found in the records directory's own `.claude/agents/` as well as in
/// `~/.claude/agents/` (coordinator decision 3, pass 1).
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

/// The refusal names the task that puts the agent in place.
#[test]
fn conductor_start_without_the_agent_names_agents_link() {
    let Some(case) = Case::new("start-no-agent-message") else {
        return;
    };
    let config = case.config("opus", None);
    let output = case.task("conductor:start", Some(&config));
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("task agents:link"),
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
        String::from_utf8_lossy(&output.stderr).contains("task agents:link"),
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
         \x20     - {{role: controller, harness: claude, model: opus}}\n",
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
            root().join(".claude/conductor-settings.json").display()
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
/// checkout as the agent conductor-dev, without the user's settings and with no `--settings`.
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
             bypassPermissions {SETTING_SOURCES} Start of session."
        )),
        "{args}"
    );
    assert!(!args.contains("--settings "), "{args}");
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

/// Every session start command conductor's profile writes (a controller's start and its resume
/// after a usage limit) runs without the user's settings and with the controller settings file,
/// and names the profile form beside the agent form.
#[test]
fn the_conductor_profile_starts_and_resumes_controllers_without_user_settings() {
    let text = fs::read_to_string(root().join(".agents/conductor.md")).expect("read the profile");
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let commands: Vec<&str> = flat
        .match_indices("claude --bg")
        .map(|(at, _)| {
            let rest = &flat[at..];
            &rest[..rest.find('`').unwrap_or(rest.len())]
        })
        .collect();
    let starts = commands
        .iter()
        .filter(|command| !command.contains("--resume"))
        .count();
    let resumes = commands
        .iter()
        .filter(|command| command.contains("--resume"))
        .count();
    assert!(
        starts >= 1 && resumes >= 1,
        "a controller start and a resume command: {commands:#?}"
    );
    for command in &commands {
        for part in [
            "--setting-sources project,local --settings <controller settings>",
            "--append-system-prompt-file <controller profile>",
            "--permission-mode bypassPermissions",
        ] {
            assert!(command.contains(part), "{part}: {command}");
        }
        assert!(!command.contains("--agent"), "the profile form: {command}");
    }
    assert!(
        flat.contains("--agent <controller agent>"),
        "the agent form, when the controller role names no profile: {flat}"
    );
}

/// `conductor:start` now runs claude outside this checkout, where the project-level
/// `.claude/agents/conductor.md` is not found; `--agent conductor` resolves only when `task
/// agents:link` has run (or the records carry their own adapter). With neither, start must
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
    let agents = case.dir.join("home/.claude/agents");
    fs::create_dir_all(&agents).expect("create ~/.claude/agents");
    let last = agents.join("conductor-dev.md");
    fs::write(&last, "the user's own\n").expect("write a different file");

    let output = case.task("agents:link", None);
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
    case.link_agent("conductor");
    let records = case.dir.join("rec ords 'q' $(touch pwned) `touch pwned2`");
    fs::create_dir_all(&records).expect("create the records directory");
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
         \x20     - {{role: controller, harness: claude, model: opus}}\n",
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
         \x20   records: {records}\n",
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

/// Every text scalar of `value`, depth first, as written.
fn scalars(value: &serde_yaml::Value) -> Vec<String> {
    match value {
        serde_yaml::Value::String(text) => vec![text.clone()],
        serde_yaml::Value::Sequence(items) => items.iter().flat_map(scalars).collect(),
        serde_yaml::Value::Mapping(map) => map.values().flat_map(scalars).collect(),
        _ => Vec::new(),
    }
}

/// The controller start and resume commands of the conductor profile name the session
/// `<controller session name>`, which the profile says comes from `conductor config show`; no
/// command names it after the bare repository.
#[test]
fn the_conductor_profile_names_controllers_by_their_session_name() {
    let text = fs::read_to_string(root().join(".agents/conductor.md")).expect("read the profile");
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let commands: Vec<&str> = flat
        .match_indices("claude --bg")
        .map(|(at, _)| {
            let rest = &flat[at..];
            &rest[..rest.find('`').unwrap_or(rest.len())]
        })
        .collect();
    assert!(
        commands.len() >= 2,
        "a start and a resume command: {commands:#?}"
    );
    for command in &commands {
        assert!(
            command.contains(" -n <controller session name> "),
            "the session name: {command}"
        );
        assert!(
            !command.contains("-n <repo>"),
            "the bare repository: {command}"
        );
    }
    assert!(
        flat.contains("`<controller session name>` is `<session_prefix>-<repo>`, or `<repo>` when `session_prefix` is null in `conductor config show`"),
        "the profile says where the name comes from: {flat}"
    );
}
