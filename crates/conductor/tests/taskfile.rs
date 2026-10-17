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

/// The task that reaches a conductor session: `conductor:attach`. `conductor run` retired
/// `conductor:start` and `conductor:restart` (`story:run-daemon`).
const CONDUCTOR_TASKS: [&str; 1] = ["conductor:attach"];

/// The records directory of the instance `conductor config show` reports: the one it selects.
const RECORDS: &str = "conductor config show --format json | jq -r '.instances[0].records'";

/// The prefix every shell command of a conductor task starts with.
const CD_RECORDS: &str = "cd {{shellQuote .RECORDS}} && ";

/// The adapters `task agents:link` links; the conductor adapter is retired (`story:run-daemon`).
const AGENTS: [&str; 2] = ["repo-controller", "conductor-dev"];

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

/// `story:run-daemon` retired the conductor adapter: agents:link links the two left, and still
/// takes back an old link of all three under `~/.claude/agents/`.
#[test]
fn agents_link_names_the_two_adapters_and_never_overwrites() {
    let taskfile = taskfile();
    let task = &taskfile["tasks"]["agents:link"];
    assert!(!task.is_null(), "Taskfile.yml has no task agents:link");
    let commands = shell_commands(task).join("\n");
    assert_eq!(
        commands
            .matches("for name in repo-controller conductor-dev; do")
            .count(),
        2,
        "{commands}"
    );
    assert!(
        commands.contains("for name in conductor repo-controller conductor-dev; do\n  old="),
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

    // An earlier agents:link left a link into this checkout under ~/.claude/agents/, of the
    // conductor adapter `story:run-daemon` retired; the user keeps a file of their own there too.
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
    let file = agents.join("repo-controller.md");
    fs::remove_file(&file).expect("remove the link");
    fs::write(&file, "the user's own\n").expect("write a different file");
    let elsewhere = case.dir.join("home/elsewhere.md");
    fs::write(&elsewhere, "another\n").expect("write another file");
    let link = agents.join("conductor-dev.md");
    fs::remove_file(&link).expect("remove the link");
    std::os::unix::fs::symlink(&elsewhere, &link).expect("link elsewhere");

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

    // With one file in the way and the other adapter missing, nothing is linked.
    let missing = link;
    fs::remove_file(&missing).expect("remove the link elsewhere");
    let output = case.agents_link(&repo);
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        fs::symlink_metadata(&missing).is_err(),
        "nothing is linked while one file is refused"
    );
}

// ---------------------------------------------------------------------------------------------
// Adversary cases (wave 01, U2, pass 1)
// ---------------------------------------------------------------------------------------------

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

/// The refusal of `agents:link` is all or nothing: when only the last adapter is in the way,
/// the first is not linked either. The unit's own case puts the conflict on the first
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
    assert!(
        fs::symlink_metadata(agents.join("repo-controller.md")).is_err(),
        "repo-controller.md was linked while conductor-dev.md is refused: {}",
        shown(&output)
    );
    assert_eq!(fs::read_to_string(&last).expect("read"), "the user's own\n");
}

// ---------------------------------------------------------------------------------------------
// Adversary cases (wave 01, U2, pass 2)
// ---------------------------------------------------------------------------------------------

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
                "--bg -n {dev} --agent conductor-dev --model opus "
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
/// stay their names for this release. With `host.yaml` naming a maintainer it starts under the
/// session name `maintainer`, on its model; without one, the maintainer an instance's `roles`
/// still writes starts under the name it has there, as before.
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
        calls
            .last()
            .expect("a start")
            .1
            .starts_with("--bg -n a-conductor-dev --agent conductor-dev --model opus "),
        "{calls:?}"
    );
    write_host(
        &case,
        "  - {role: maintainer, harness: claude, model: haiku}\n",
    );
    for task in ["maintainer:start", "dev:start"] {
        let output = case.task(task, Some(&config));
        assert_eq!(output.status.code(), Some(0), "{task}: {}", shown(&output));
        let calls = case.calls();
        let (cwd, args) = calls.last().expect("a start");
        assert_eq!(cwd, &root(), "{task}: {args}");
        assert!(
            args.starts_with("--bg -n maintainer --agent conductor-dev --model haiku "),
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
        assert_eq!(case.calls(), [(root(), "attach s-m".to_owned())], "{task}");
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
