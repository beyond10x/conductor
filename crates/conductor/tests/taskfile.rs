//! `story:instance-records-start`: the tasks of `Taskfile.yml` that run conductor run it in the
//! active instance's records directory, `task trust` trusts that directory, and `task
//! agents:link` links this checkout's Claude Code adapters into `~/.claude/agents/` without
//! overwriting anything.
//!
//! The first tests read `Taskfile.yml` as YAML. The others run the tasks with `task` against a
//! case directory under this test target's temporary directory: `HOME` names the case's home,
//! `CONDUCTOR_CONFIG` a config file the case writes (or is removed), `CONDUCTOR_INSTANCE` is
//! removed, and `PATH` starts with the case's `bin/`, which holds the built `conductor` and a
//! `claude` that logs its working directory and arguments, so no case starts a session or reads
//! the operator's config. When `task` is not on `PATH` those tests print why and return.

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
    let commands = shell_commands(task).join("\n");
    for part in [
        "claude --bg -n conductor --agent conductor ",
        "--model {{shellQuote .MODEL}}",
        "--settings {{shellQuote .SETTINGS}}",
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
fn trust_covers_the_records_directory() {
    let taskfile = taskfile();
    let task = &taskfile["tasks"]["trust"];
    assert_eq!(task["vars"]["RECORDS"]["sh"].as_str(), Some(RECORDS));
    let commands = shell_commands(task).join("\n");
    assert!(
        commands.contains("records={{shellQuote .RECORDS}}"),
        "trust names the records directory: {commands}"
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
        std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_conductor"), dir.join("bin/conductor"))
            .expect("link the built conductor");
        // A claude that answers `agents --json` with a live session `s-0001` while
        // `claude/live` exists, removes that file on `stop`, and logs every other call: its
        // working directory, a tab, its arguments.
        let claude = dir.join("bin/claude");
        fs::write(
            &claude,
            format!(
                "#!/bin/sh\n\
                 state='{state}'\n\
                 if [ \"$1\" = agents ]; then\n\
                 \x20 if [ -e \"$state/live\" ]; then\n\
                 \x20   printf '%s\\n' '[{{\"kind\":\"background\",\"name\":\"conductor\",\"pid\":4242,\"id\":\"s-0001\"}}]'\n\
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
        Some(Self { dir })
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
        let mut path = vec![self.dir.join("bin")];
        path.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
        let mut command = Command::new("task");
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
            .stdin(Stdio::null());
        if let Some(config) = config {
            command.env("CONDUCTOR_CONFIG", config);
        }
        command.output().expect("task runs")
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
             bypassPermissions --settings {} Start of session.",
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
