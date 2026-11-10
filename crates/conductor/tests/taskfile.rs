//! `story:instance-records-start`: the tasks of `Taskfile.yml` that run conductor run it in the
//! active instance's records directory and `task trust` trusts that directory;
//! `story:roles-in-the-binary` retired `task agents:link` and `task profiles`, since no adapter
//! or profile is copied anywhere. `task trust` runs `conductor trust` (`story:portable-trust`), whose own
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

/// The task that reaches a conductor session: `conductor:attach`. `conductor run` retired
/// `conductor:start` and `conductor:restart` (`story:run-daemon`).
const CONDUCTOR_TASKS: [&str; 1] = ["conductor:attach"];

/// The records directory of the instance `conductor config show` reports: the one it selects.
const RECORDS: &str = "conductor config show --format json | jq -r '.instances[0].records'";

/// The prefix every shell command of a conductor task starts with.
const CD_RECORDS: &str = "cd {{shellQuote .RECORDS}} && ";

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

/// `story:session-envelope-seam`: `maintainer:start` (`dev:start`) starts its session with
/// `conductor session start --role maintainer`; no task starts or stops a session with `claude
/// --bg`, `claude stop` or `claude rm` itself.
#[test]
fn the_session_tasks_start_and_stop_through_conductor_session() {
    let taskfile = taskfile();
    let task = &taskfile["tasks"]["maintainer:start"];
    assert_eq!(
        shell_commands(task),
        ["conductor session start --role maintainer"],
        "maintainer:start: {task:?}"
    );
    let text = serde_yaml::to_string(&task["cmds"]).expect("the commands as text");
    for old in ["claude --bg", "claude stop", "claude rm"] {
        assert!(!text.contains(old), "maintainer:start runs {old}: {text}");
    }
}

/// `story:run-daemon`: `conductor run` is the instance's loop, so `conductor:start` and
/// `conductor:restart` are retired, and no task starts or stops conductor's session.
#[test]
fn the_conductor_session_tasks_are_retired() {
    let taskfile = taskfile();
    for task in ["conductor:start", "conductor:restart"] {
        assert!(
            taskfile["tasks"][task].is_null(),
            "Taskfile.yml still has {task}"
        );
    }
    let text = fs::read_to_string(root().join("Taskfile.yml")).expect("read Taskfile.yml");
    for gone in [
        "session start --role conductor",
        "session stop --role conductor",
        "task: conductor:start",
        ".agents/conductor.md",
    ] {
        assert!(!text.contains(gone), "Taskfile.yml holds {gone:?}");
    }
}

/// `story:roles-in-the-binary`: the role profiles ship inside the binary, so `task profiles`
/// (which copied them to `profiles/` beside the config file) and `task agents:link` (which linked
/// the committed Claude Code adapters into a repository) are gone, `task install` installs the
/// binary alone, and no task names `.agents/`, `.claude/agents/` or a profiles directory.
#[test]
fn no_task_copies_a_profile_or_links_an_adapter() {
    let taskfile = taskfile();
    for task in ["profiles", "agents:link"] {
        assert!(
            taskfile["tasks"][task].is_null(),
            "Taskfile.yml still has {task}"
        );
    }
    assert_eq!(
        shell_commands(&taskfile["tasks"]["install"]),
        ["cargo install --path crates/conductor --locked"],
        "install installs the binary alone"
    );
    let text = fs::read_to_string(root().join("Taskfile.yml")).expect("read Taskfile.yml");
    for gone in [
        ".agents/",
        ".claude/agents",
        "task: profiles",
        "task profiles",
        "/profiles",
    ] {
        assert!(!text.contains(gone), "Taskfile.yml holds {gone:?}");
    }
}

/// `story:instance-api`: the dashboard is the daemon's page; `task dashboard` starts no process of
/// its own and prints the daemon's URL naming the active instance.
#[test]
fn the_dashboard_task_prints_the_daemon_s_url_for_the_instance() {
    let taskfile = taskfile();
    let commands = shell_commands(&taskfile["tasks"]["dashboard"]).join("\n");
    for gone in [
        "dashboard serve",
        "systemd-run",
        "nohup",
        "-conductor-dashboard",
    ] {
        assert!(
            !commands.contains(gone),
            "the dashboard task runs {gone:?}: {commands}"
        );
    }
    assert!(
        commands
            .contains("http://127.0.0.1:{{shellQuote .PORT}}/?instance={{shellQuote .INSTANCE}}"),
        "the dashboard task prints the daemon's URL: {commands}"
    );
    let port = taskfile["tasks"]["dashboard"]["vars"]["PORT"]["sh"]
        .as_str()
        .expect("PORT is a shell variable");
    assert!(
        port.contains(".daemon.port"),
        "PORT reads daemon.port: {port}"
    );
    assert!(
        taskfile["tasks"]["dashboard:stop"].is_null(),
        "no dashboard process is left to stop"
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
fn conductor_attach_runs_in_the_records_directory() {
    let Some(case) = Case::new("attach") else {
        return;
    };
    let config = case.config("opus", None);
    fs::write(case.dir.join("claude/live"), "").expect("a live session");

    let output = case.task("conductor:attach", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));

    let calls = case.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].1, "attach s-0001");
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

// ---------------------------------------------------------------------------------------------
// Adversary cases (wave 01, U2, pass 1)
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// Sessions without the user's global config (wave 02, U1)
// ---------------------------------------------------------------------------------------------

/// The setting sources of a session the `none` provider starts, which runs `claude --bg`: the
/// project's and the local ones beside the `--settings` file, and the user's, because Claude Code
/// takes the bypass-permissions acceptance a background session needs only from user settings
/// (`story:bg-launch-reads-user-settings`). A provider that runs no `--bg` passes `project,local`.
const SETTING_SOURCES: &str = "--setting-sources user,project,local";

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

/// Without a settings file or profile on the role conductor-dev, conductor-dev starts in this
/// checkout from the maintainer profile the binary carries, written into the state directory
/// (`story:roles-in-the-binary`) and appended to its system prompt, with the setting sources of a
/// `--bg` launch ([`SETTING_SOURCES`]), and with settings that only pin its instance
/// (`story:instance-isolation`). Nothing is written under `~/.claude/agents/`.
#[test]
fn dev_start_appends_the_binary_s_maintainer_profile_with_settings_pinned_to_its_instance() {
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
    let profile = case
        .dir
        .join("home/.b10x/conductor/alpha/state/sessions/conductor-dev.profile.md");
    assert!(
        args.starts_with(&format!(
            "--bg -n conductor-dev --append-system-prompt-file {} --model sonnet --disallowedTools \
             Artifact,ScheduleWakeup,AskUserQuestion,ReportFindings,ShareOnboardingGuide \
             --permission-mode bypassPermissions {SETTING_SOURCES} --settings {} Start of \
             session.",
            profile.display(),
            case.pinned_default("conductor-dev").display()
        )),
        "{args}"
    );
    assert!(!args.contains("--agent"), "{args}");
    assert_eq!(
        fs::read_to_string(&profile).expect("the written profile"),
        fs::read_to_string(root().join("crates/conductor/roles/maintainer.md"))
            .expect("the maintainer profile"),
        "the profile written is the binary's maintainer profile"
    );
    assert!(
        !case.dir.join("home/.claude/agents").exists(),
        "nothing is written under ~/.claude/agents/"
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

// ---------------------------------------------------------------------------------------------
// Adversary cases (wave 01, U2, pass 2)
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// The bypass-permissions disclaimer (wave 02, U6, `story:first-run-bypass-disclaimer`)
// ---------------------------------------------------------------------------------------------

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

/// `dev:start` passes `-n <prefix>-conductor-dev` with a prefix, and `-n conductor-dev` without
/// one: the name it reads from the role's `session_name` in `conductor config show`.
#[test]
fn the_start_tasks_name_the_session_from_config_show() {
    for (prefix, dev) in [(Some("a"), "a-conductor-dev"), (None, "conductor-dev")] {
        let Some(case) = Case::new(&format!("named-start-{}", prefix.unwrap_or("none"))) else {
            return;
        };
        let config = named_config(&case, prefix);
        let output = case.task("dev:start", Some(&config));
        assert_eq!(
            output.status.code(),
            Some(0),
            "dev:start: {}",
            shown(&output)
        );
        let calls = case.calls();
        let args = &calls.last().expect("a start").1;
        assert!(
            args.starts_with(&format!(
                "--bg -n {dev} --append-system-prompt-file {} --model opus ",
                case.dir
                    .join(format!(
                        "home/.b10x/conductor/alpha/state/sessions/{dev}.profile.md"
                    ))
                    .display()
            )),
            "dev:start: {args}"
        );
    }
}

/// The start task reads the name from `conductor config show`, and uses no literal session name.
#[test]
fn the_session_tasks_read_the_name_from_config_show() {
    let taskfile = taskfile();
    // `conductor:start` and `conductor:restart` read the role conductor's name until
    // `story:run-daemon` retired them; the maintainer's is the host's.
    let (task, role) = ("maintainer:start", "conductor-dev");
    {
        let task_body = &taskfile["tasks"][task];
        let wanted = host_select("session_name // empty");
        assert_eq!(
            task_body["vars"]["NAME"]["sh"].as_str(),
            Some(wanted.as_str()),
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

/// The `sh:` of a variable of the maintainer's tasks that reads `field` of the host's maintainer
/// from `conductor config show`: `host.roles`, the role under its name or its old name
/// (`story:maintainer-host-config`).
fn host_select(field: &str) -> String {
    format!(
        "conductor config show --format json | jq -r '.host.roles[] | select(.role == \
         \"maintainer\" or .role == \"conductor-dev\") | .{field}'"
    )
}

/// Writes the host file of the case's home, `~/.b10x/conductor/host/host.yaml`, holding `roles`,
/// the lines of a `roles:` list.
fn write_host(case: &Case, roles: &str) {
    let dir = case.dir.join("home/.b10x/conductor/host");
    fs::create_dir_all(&dir).expect("create the host directory");
    fs::write(
        dir.join("host.yaml"),
        format!("version: conductor.host-config/1\nroles:\n{roles}"),
    )
    .expect("write the host file");
}

/// `story:maintainer-host-config`, item 4: `maintainer:start` and `maintainer:attach` read the
/// maintainer from `host.roles` of `conductor config show`, and `dev:start` and `dev:attach`
/// stay their names for this release. The maintainer is the one the instance's `roles` writes,
/// under the name it has there: the host file that named another is gone (`story:spec-v1`).
#[test]
fn the_maintainer_tasks_read_the_host_s_roles_from_config_show() {
    let taskfile = taskfile();
    for (task, alias) in [
        ("maintainer:start", "dev:start"),
        ("maintainer:attach", "dev:attach"),
    ] {
        let body = &taskfile["tasks"][task];
        assert_eq!(
            body["vars"]["NAME"]["sh"].as_str(),
            Some(host_select("session_name // empty").as_str()),
            "{task}"
        );
        assert!(
            body["aliases"]
                .as_sequence()
                .is_some_and(|aliases| aliases.iter().any(|one| one.as_str() == Some(alias))),
            "{task} keeps {alias} as an alias: {body:?}"
        );
    }
    let start = &taskfile["tasks"]["maintainer:start"];
    assert_eq!(
        shell_commands(start),
        ["conductor session start --role maintainer"],
        "{start:?}"
    );
    for (var, field) in [
        ("HARNESS", "harness"),
        ("MODEL", "model"),
        ("PROFILE", "profile // empty"),
        ("SETTINGS", "settings // empty"),
    ] {
        assert_eq!(
            start["vars"][var]["sh"].as_str(),
            Some(host_select(field).as_str()),
            "maintainer:start {var}"
        );
    }

    let Some(case) = Case::new("maintainer-host") else {
        return;
    };
    let config = named_config(&case, Some("a"));
    // No host file: the instance's built-in conductor-dev, as before.
    let output = case.task("maintainer:start", Some(&config));
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let calls = case.calls();
    assert!(
        calls.last().expect("a start").1.starts_with(&format!(
            "--bg -n a-conductor-dev --append-system-prompt-file {} --model opus ",
            case.dir
                .join("home/.b10x/conductor/alpha/state/sessions/a-conductor-dev.profile.md")
                .display()
        )),
        "{calls:?}"
    );
    // The host file is gone in v1 (`story:spec-v1`): one naming another maintainer is read by
    // nothing, and the instance's conductor-dev still starts and is attached to.
    write_host(
        &case,
        "  - {role: maintainer, harness: claude, model: haiku}\n",
    );
    for task in ["maintainer:start", "dev:start"] {
        let _ = fs::remove_file(case.dir.join("claude/log"));
        let output = case.task(task, Some(&config));
        assert_eq!(output.status.code(), Some(0), "{task}: {}", shown(&output));
        let calls = case.calls();
        let (cwd, args) = calls.last().expect("a start");
        assert_eq!(cwd, &root(), "{task}: {args}");
        assert!(
            args.starts_with(&format!(
                "--bg -n a-conductor-dev --append-system-prompt-file {} --model opus ",
                case.dir
                    .join("home/.b10x/conductor/alpha/state/sessions/a-conductor-dev.profile.md")
                    .display()
            )),
            "{task}: {args}"
        );
    }

    sessions_claude(
        &case,
        &[("a-conductor-dev", "s-old"), ("maintainer", "s-m")],
    );
    for task in ["maintainer:attach", "dev:attach"] {
        let _ = fs::remove_file(case.dir.join("claude/log"));
        let output = case.task(task, Some(&config));
        assert_eq!(output.status.code(), Some(0), "{task}: {}", shown(&output));
        assert_eq!(
            case.calls(),
            [(root(), "attach s-old".to_owned())],
            "{task}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The start form of each role under `--setting-sources project,local`
// (story:agent-needs-user-setting-source)
// ---------------------------------------------------------------------------------------------

/// Every text scalar of `value`, depth first, as written.
fn scalars(value: &serde_yaml::Value) -> Vec<String> {
    match value {
        serde_yaml::Value::String(text) => vec![text.clone()],
        serde_yaml::Value::Sequence(items) => items.iter().flat_map(scalars).collect(),
        serde_yaml::Value::Mapping(map) => map.values().flat_map(scalars).collect(),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------------------------
// `story:restart-current-profile`: a start refuses a profile checkout behind its upstream
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// `story:instance-scoped-tasks`: every task acts on the active instance's sessions only
// ---------------------------------------------------------------------------------------------

/// Writes a config file of two instances: `alpha`, `session_prefix: a`, whose records are the
/// case's `records/`, and `beta`, `session_prefix: b`, whose records are `records-b/`; neither
/// is the file's default, so each task runs the one `CONDUCTOR_INSTANCE` names.
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
    assert_eq!(
        taskfile["tasks"]["conductor:attach"]["vars"]["NAME"]["sh"].as_str(),
        Some(
            "conductor config show --format json | jq -r '.instances[0].roles[] | \
             select(.role == \"conductor\") | .session_name // empty'"
        ),
    );
    assert_eq!(
        taskfile["tasks"]["maintainer:attach"]["vars"]["NAME"]["sh"].as_str(),
        Some(host_select("session_name // empty").as_str()),
    );
    // The instance's roles' names and the host's (`story:maintainer-host-config`), each once.
    let sessions = &taskfile["tasks"]["sessions"];
    assert_eq!(
        sessions["vars"]["NAMES"]["sh"].as_str(),
        Some(
            "conductor config show --format json | jq -c '[.instances[0].roles[], .host.roles[] \
             | .session_name // empty] | unique'"
        ),
        "{sessions:?}"
    );
    for task in [
        "sessions",
        "conductor:attach",
        "maintainer:start",
        "maintainer:attach",
    ] {
        assert!(
            !taskfile["tasks"][task].is_null(),
            "Taskfile.yml has no {task}"
        );
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

/// A name that more than one live session holds is refused by every task that resolves it: the
/// attach tasks attach nothing, and `task sessions` exits 1 naming it. `task sessions` counts a
/// live session of any kind (one the systemd provider started lists as interactive), the attach
/// tasks a background one.
#[test]
fn a_name_more_than_one_live_session_holds_is_refused() {
    let Some(case) = Case::new("scoped-ambiguous") else {
        return;
    };
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
    for (task, name, kind) in [
        ("conductor:attach", "a-conductor", "live background session"),
        ("dev:attach", "a-conductor-dev", "live background session"),
        ("sessions", "a-conductor", "live session"),
        ("sessions", "a-conductor-dev", "live session"),
    ] {
        let output = instance_task(&case, task, &config, "alpha");
        assert_ne!(output.status.code(), Some(0), "{task}: {}", shown(&output));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains(&format!("more than one {kind} is named {name}")),
            "{task}: the refusal names {name}: {}",
            shown(&output)
        );
        assert!(case.calls().is_empty(), "{task}: {:?}", case.calls());
        assert_eq!(live_sessions(&case), before, "{task} stopped a session");
    }
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

// ---------------------------------------------------------------------------------------------
// The host's supervisor and teacher (`story:host-roles-by-daemon`)

/// The host roles' start tasks, the role each asks the daemon for, and the profile it starts from.
const HOST_TASKS: [(&str, &str, &str); 2] = [
    ("supervisor:start", "supervisor", "conductor-supervisor"),
    ("teacher:start", "teacher", "conductor-teacher"),
];

/// `story:host-roles-by-daemon`: `task supervisor:start` and `task teacher:start` ask the daemon
/// for their role (`conductor daemon start-role --role <role>`), which starts it from the profile
/// the binary carries (`story:roles-in-the-binary`); neither runs the harness itself. Replaces the
/// case of `story:host-session-profiles`, whose tasks ran the profile's `claude --bg` start
/// command.
#[test]
fn the_host_session_tasks_ask_the_daemon_for_their_role() {
    let taskfile = taskfile();
    for (task, role, profile) in HOST_TASKS {
        assert_eq!(
            shell_commands(&taskfile["tasks"][task]),
            [format!("conductor daemon start-role --role {role}")],
            "{task} asks the daemon for the {role}"
        );
        let file = root().join(format!("crates/conductor/roles/{role}.md"));
        let text =
            fs::read_to_string(&file).unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        assert!(
            !text.contains(&format!("`claude --bg --agent {profile} ")),
            "{}: still writes a claude --bg start command",
            file.display()
        );
    }
}
