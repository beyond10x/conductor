//! `story:spawn`: `conductor spawn --repository <repo>` starts the repository's controller session
//! with the controller start form of the conductor profile, built from the config, and records the
//! controller `Running`; it refuses, launching nothing and recording nothing, while a session of
//! that name runs, and for every other reason the specification's `SpawnRefusal` names.
//!
//! Each case is a directory of its own under this test target's temporary directory: `home/`, the
//! `HOME` the binary runs with, holding the checkouts root `example-org/` with the checkout
//! `alpha` and `~/.claude.json` marking it trusted; `records/` with the charter
//! `charters/alpha.md`; `state/`; `roles/` with the controller role's profile and settings; and the
//! config file `conductor.yaml` naming them, under the session prefix `ex`.
//!
//! `claude` is a stand-in first on `PATH`: `claude agents --json` prints the case's session list;
//! any other call is a launch, which writes each argument to a file of its own under `launch/`
//! (`1`, `2`, …), its physical working directory to `launch/cwd`, counts itself in `launches`,
//! and prints a session id, or, when the case holds `fail`, exits 1. No real session starts.
//!
//! `story:session-envelope-seam`: spawn starts the controller through a confinement provider. The
//! cases that name a fake provider run the command over it ([`fake_provider`]); the others run
//! the built binary with the instance's `confinement` the `none` provider ([`UNBOXED`]), which
//! starts every controller with the start form above.

mod common;
mod fake_provider;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use conductor_cli::cli::{Access, LEAVES};
use conductor_cli::config;
use fake_provider::{Child, call_names, calls};
use serde_json::{Value, json};

/// The child run of this test binary ([`fake_provider::command`]); a plain run of the suite
/// passes it at once.
#[test]
fn child() {
    if let Some(run) = Child::from_env() {
        if let Some(file) = run.config_file() {
            config::set_active(Some(&file)).expect("the case's config loads");
        }
        run.exit();
    }
}

/// The id the stand-in prints for a session it started.
const SESSION_ID: &str = "bg-session-0001";

/// The `confinement` the config file writes until a case sets its own ([`Case::confine`]): the
/// `none` provider, so the built binary launches the case's `claude` itself and never reaches the
/// host's `systemd-run`, whatever cgroup controllers the host delegates
/// (`story:session-limits-systemd`).
const UNBOXED: &str = "\x20   confinement: {provider: none}\n";

/// A charter whose text a shell would expand: a `$(…)`, a backtick, both quotes and a line break.
/// It reaches `claude` as one argument, byte for byte, and nothing in it runs.
const CHARTER: &str = "Charter for alpha: $(touch expanded) `touch ticked` \"quoted\" 'single'\n\
                       forge: github\n";

/// A case directory of its own; see the module documentation.
struct Case {
    dir: PathBuf,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("spawn")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        case.fixture();
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn root(&self) -> PathBuf {
        self.home().join("example-org")
    }

    fn checkout(&self) -> PathBuf {
        self.root().join("alpha")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn charter(&self) -> PathBuf {
        self.records().join("charters/alpha.md")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn profile(&self) -> PathBuf {
        self.dir.join("roles/repo-controller.md")
    }

    fn settings(&self) -> PathBuf {
        self.dir.join("roles/controller-settings.json")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn launch(&self) -> PathBuf {
        self.dir.join("launch")
    }

    /// Writes the config file; `controller` is the controller role's entry of `roles`.
    fn write_config(&self, controller: &str) {
        write(
            &self.config(),
            &format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   session_prefix: ex\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n\
                 \x20   roles:\n\
                 \x20     - {{role: conductor, harness: claude, model: opus}}\n\
                 \x20     - {controller}\n\
                 {UNBOXED}",
                root = self.root().display(),
                trees = self.dir.join("trees").display(),
                records = self.records().display(),
                state = self.state().display(),
            ),
        );
    }

    /// The controller role every case starts with but the one that changes it.
    fn full_role(&self) -> String {
        format!(
            "{{role: controller, harness: claude, model: sonnet, profile: {}, settings: {}}}",
            self.profile().display(),
            self.settings().display()
        )
    }

    /// The live sessions `claude agents --json` prints.
    fn sessions(&self, sessions: &Value) {
        write(&self.dir.join("agents.json"), &sessions.to_string());
    }

    /// Marks the checkout trusted in `~/.claude.json`, or not.
    fn trust(&self, trusted: bool) {
        let checkout = self.checkout().display().to_string();
        write(
            &self.home().join(".claude.json"),
            &json!({"numStartups": 3, "projects": {checkout: {"hasTrustDialogAccepted": trusted}}})
                .to_string(),
        );
    }

    fn fixture(&self) {
        write(&self.checkout().join(".git/HEAD"), "ref: refs/heads/main\n");
        write(&self.charter(), CHARTER);
        write(&self.profile(), "# Repository controller\n");
        // The settings pin the instance (`story:instance-isolation`).
        write(&self.settings(), &pinned("example-org"));
        fs::create_dir_all(self.state()).expect("create the state directory");
        self.write_config(&self.full_role());
        self.trust(true);
        // A live session of another name, and an exited one of the controller's name: neither
        // holds the name.
        self.sessions(&json!([
            {"id": "s-other", "name": "ex-beta", "cwd": self.root().join("beta"), "pid": 101,
             "status": "busy"},
            {"id": "s-gone", "name": "ex-alpha", "cwd": self.checkout(), "status": "idle"}
        ]));
        let bin = self.dir.join("bin");
        let path = bin.join("claude");
        write(
            &path,
            &format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = agents ]; then cat '{agents}'; exit 0; fi\n\
                 n=$(( $(cat '{launches}' 2>/dev/null || echo 0) + 1 ))\n\
                 echo \"$n\" > '{launches}'\n\
                 rm -rf '{launch}'\n\
                 mkdir -p '{launch}'\n\
                 i=0\n\
                 for arg; do i=$((i + 1)); printf '%s' \"$arg\" > '{launch}'/\"$i\"; done\n\
                 pwd -P > '{launch}/cwd'\n\
                 if [ -e '{fail}' ]; then echo 'Workspace not trusted' >&2; exit 1; fi\n\
                 echo '{SESSION_ID}'\n",
                agents = self.dir.join("agents.json").display(),
                launches = self.dir.join("launches").display(),
                launch = self.launch().display(),
                fail = self.dir.join("fail").display(),
            ),
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make it executable");
    }

    /// `conductor <args>` from the records directory, with `HOME` naming the case's home, the
    /// case's config file, and the stand-in first on `PATH`.
    fn run(&self, args: &[&str]) -> Output {
        let path = format!("{}:/usr/bin:/bin", self.dir.join("bin").display());
        common::conductor(self.home())
            .args(args)
            .current_dir(self.records())
            .env("CONDUCTOR_CONFIG", self.config())
            .env("PATH", path)
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    fn spawn(&self) -> Output {
        self.run(&["spawn", "--repository", "alpha"])
    }

    /// The fake provider's directory.
    fn fake(&self) -> PathBuf {
        self.dir.join("fake")
    }

    /// Sets the instance's `confinement` in the config file, in place of [`UNBOXED`].
    fn confine(&self, confinement: &str) {
        let text = fs::read_to_string(self.config())
            .expect("read the config file")
            .replace(UNBOXED, "");
        write(
            &self.config(),
            &format!("{text}\x20   confinement: {confinement}\n"),
        );
    }

    /// `conductor spawn --repository alpha` as [`Case::run`] runs it, over the fake providers.
    fn spawn_fake(&self) -> Output {
        fs::create_dir_all(self.fake()).expect("create the fake's directory");
        let path = format!("{}:/usr/bin:/bin", self.dir.join("bin").display());
        let output = fake_provider::command(
            &self.home(),
            &self.fake(),
            &["spawn", "--repository", "alpha"],
        )
        .current_dir(self.records())
        .env("CONDUCTOR_CONFIG", self.config())
        .env("PATH", path)
        .output()
        .expect("the child runs");
        fake_provider::without_harness(output)
    }

    /// `conductor <args>` as [`Case::spawn_fake`] runs spawn, over the fake providers.
    fn run_fake(&self, args: &[&str]) -> Output {
        fs::create_dir_all(self.fake()).expect("create the fake's directory");
        let path = format!("{}:/usr/bin:/bin", self.dir.join("bin").display());
        let output = fake_provider::command(&self.home(), &self.fake(), args)
            .current_dir(self.records())
            .env("CONDUCTOR_CONFIG", self.config())
            .env("PATH", path)
            .output()
            .expect("the child runs");
        fake_provider::without_harness(output)
    }

    /// How many sessions the stand-in launched.
    fn launches(&self) -> usize {
        fs::read_to_string(self.dir.join("launches"))
            .map_or(0, |text| text.trim().parse().expect("a launch count"))
    }

    /// The arguments of the last launch, in order.
    fn argv(&self) -> Vec<String> {
        (1..)
            .map(|at| self.launch().join(at.to_string()))
            .take_while(|file| file.exists())
            .map(|file| fs::read_to_string(file).expect("read an argument"))
            .collect()
    }

    /// The physical working directory of the last launch.
    fn cwd(&self) -> String {
        fs::read_to_string(self.launch().join("cwd"))
            .expect("read the launch's working directory")
            .trim_end()
            .to_owned()
    }

    /// The rows of `controller controllers --format json`.
    fn controllers(&self) -> Vec<Value> {
        let output = self.run(&["controller", "controllers", "--format", "json"]);
        assert!(
            output.status.success(),
            "controllers: {}",
            describe(&output)
        );
        match serde_json::from_slice(&output.stdout).expect("the view prints JSON") {
            Value::Array(rows) => rows,
            other => panic!("the view printed {other}, not an array"),
        }
    }

    /// Whether the store was created at all.
    fn recorded(&self) -> bool {
        self.state().join("tree").exists()
    }
}

/// A settings file that pins `instance`: its `env` names it, and its hook command runs
/// `conductor` with it first (`story:instance-isolation`).
fn pinned(instance: &str) -> String {
    json!({
        "env": {"CONDUCTOR_INSTANCE": instance},
        "hooks": {"PreToolUse": [{"matcher": "*", "hooks": [{
            "type": "command",
            "command": format!(
                "CONDUCTOR_INSTANCE={instance} conductor guard record-guard-decision \
                 --from-pre-tool-use || exit 2"
            ),
        }]}]},
    })
    .to_string()
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The controller start form the `none` provider launches (`story:session-envelope-seam`), with
/// the case's values.
fn start_form(case: &Case) -> Vec<String> {
    [
        "--bg",
        "-n",
        "ex-alpha",
        "--append-system-prompt-file",
        &case.profile().display().to_string(),
        "--model",
        "sonnet",
        "--permission-mode",
        "bypassPermissions",
        "--setting-sources",
        "project,local",
        "--settings",
        &case.settings().display().to_string(),
        CHARTER,
    ]
    .map(str::to_owned)
    .to_vec()
}

#[test]
fn spawn_is_declared_a_writing_leaf_only_conductor_runs() {
    let declared = LEAVES
        .iter()
        .find(|(words, _)| *words == ["spawn"])
        .map(|(_, access)| *access);
    assert_eq!(declared, Some(Access::Writes));
    assert!(!Access::Writes.controller_runs());
}

#[test]
fn spawn_starts_the_controller_start_form_in_the_checkout_and_records_it_running() {
    let case = Case::new("starts");
    let output = case.spawn();
    assert!(output.status.success(), "spawn: {}", describe(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("{SESSION_ID}\n"),
        "spawn prints the session id claude --bg printed"
    );
    assert_eq!(case.launches(), 1, "one session is launched");
    assert_eq!(case.argv(), start_form(&case), "the controller start form");
    assert_eq!(
        case.cwd(),
        case.checkout().display().to_string(),
        "the session starts in the checkout"
    );
    for word in ["expanded", "ticked"] {
        assert!(
            !case.checkout().join(word).exists() && !case.records().join(word).exists(),
            "no shell expanded the charter: {word}"
        );
    }

    let rows = case.controllers();
    assert_eq!(rows.len(), 1, "one controller: {rows:#?}");
    let row = &rows[0];
    assert_eq!(row["state"], "Running", "{row}");
    assert_eq!(row["repository"], "alpha", "{row}");
    assert_eq!(row["harness"], "Claude", "{row}");
    assert_eq!(row["session_name"], "ex-alpha", "{row}");
    assert_eq!(row["charter_revision"], 1, "{row}");
}

/// The records lines of `log` (`dispatches` or `decisions`) under the case's records directory,
/// every `*.jsonl` file in name order.
fn log_lines(case: &Case, log: &str) -> Vec<Value> {
    let Ok(entries) = fs::read_dir(case.records().join(log)) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
        })
        .collect();
    files.sort();
    files
        .iter()
        .flat_map(|file| {
            fs::read_to_string(file)
                .expect("read a log file")
                .lines()
                .map(|line| serde_json::from_str(line).expect("a log line is JSON"))
                .collect::<Vec<Value>>()
        })
        .collect()
}

/// `story:records-from-commands`: `spawn` records the dispatch it starts, one `sent` line in the
/// records' dispatch log with the session id `claude --bg` printed, as well as the controller in
/// the store; a refused spawn appends nothing.
#[test]
fn spawn_records_the_dispatch_it_starts_with_the_session_id_and_the_controller() {
    let case = Case::new("records-sent");
    let output = case.spawn();
    assert!(output.status.success(), "spawn: {}", describe(&output));

    let rows = case.controllers();
    assert_eq!(rows.len(), 1, "one controller: {rows:#?}");
    let controller = &rows[0]["controller_id"];

    let lines = log_lines(&case, "dispatches");
    assert_eq!(lines.len(), 1, "one dispatch line: {lines:#?}");
    let line = &lines[0];
    assert_eq!(line["event"], "sent", "{line}");
    assert_eq!(line["repo"], "alpha", "{line}");
    assert_eq!(line["session_id"], SESSION_ID, "{line}");
    assert_eq!(&line["controller_id"], controller, "{line}");
    assert_eq!(line["to"], "ex-alpha", "{line}");
    assert!(line.get("repository").is_none(), "the key is repo: {line}");

    let again = case.spawn();
    assert_eq!(again.status.code(), Some(1), "{}", describe(&again));
    assert_eq!(
        log_lines(&case, "dispatches").len(),
        1,
        "a refused spawn appends nothing"
    );
}

#[test]
fn spawn_refuses_while_a_session_of_that_name_runs_launching_and_recording_nothing() {
    let case = Case::new("session-running");
    case.sessions(&json!([
        {"id": "s-live", "name": "ex-alpha", "cwd": case.checkout(), "pid": 4242,
         "status": "idle"}
    ]));
    let output = case.spawn();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        stderr.contains("SessionRunning") && stderr.contains("ex-alpha"),
        "the refusal names its error and the session: {stderr}"
    );
    assert!(output.stdout.is_empty(), "{}", describe(&output));
    assert_eq!(case.launches(), 0, "nothing is launched");
    assert!(!case.recorded(), "nothing is recorded");
}

#[test]
fn spawn_refuses_a_repository_whose_controller_is_recorded_running() {
    let case = Case::new("already-running");
    let first = case.spawn();
    assert!(first.status.success(), "first spawn: {}", describe(&first));
    let second = case.spawn();
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert_eq!(second.status.code(), Some(1), "{}", describe(&second));
    assert!(stderr.contains("AlreadyRunning"), "{stderr}");
    assert_eq!(case.launches(), 1, "the second spawn launches nothing");
    assert_eq!(case.controllers().len(), 1, "and records nothing");
}

/// Each refusal found before anything is recorded: it names its `SpawnRefusal`, launches nothing
/// and creates no store.
#[test]
fn spawn_refuses_what_it_cannot_start_before_recording_anything() {
    type Setup = fn(&Case);
    let cases: [(&str, &str, Setup); 11] = [
        ("no-charter", "NoCharter", |case| {
            fs::remove_file(case.charter()).expect("remove the charter");
        }),
        ("empty-charter", "NoCharter", |case| {
            write(&case.charter(), "")
        }),
        ("dash-charter", "NoCharter", |case| {
            write(&case.charter(), "--dangerously-skip-permissions\n");
        }),
        ("untrusted", "UntrustedCheckout", |case| case.trust(false)),
        ("no-checkout", "NoCheckout", |case| {
            fs::remove_dir_all(case.checkout().join(".git")).expect("remove .git");
        }),
        ("no-profile", "NoProfile", |case| {
            case.write_config(&format!(
                "{{role: controller, harness: claude, model: sonnet, settings: {}}}",
                case.settings().display()
            ));
        }),
        ("missing-profile", "NoProfile", |case| {
            fs::remove_file(case.profile()).expect("remove the profile");
        }),
        ("no-settings", "NoSettings", |case| {
            case.write_config(&format!(
                "{{role: controller, harness: claude, model: sonnet, profile: {}}}",
                case.profile().display()
            ));
        }),
        // `story:instance-isolation`, item 1: settings that pin another instance. Settings that
        // pin none start with a pinned copy (`spawn_starts_unpinned_settings_with_a_pinned_copy`).
        (
            "settings-of-another-instance",
            "InstanceNotPinned",
            |case| {
                write(&case.settings(), &pinned("other-org"));
            },
        ),
        ("hook-of-another-instance", "InstanceNotPinned", |case| {
            write(
                &case.settings(),
                &json!({
                    "env": {"CONDUCTOR_INSTANCE": "example-org"},
                    "hooks": {"PreToolUse": [{"matcher": "*", "hooks": [{
                        "type": "command",
                        "command": "CONDUCTOR_INSTANCE=other-org conductor guard \
                                    record-guard-decision --from-pre-tool-use",
                    }]}]},
                })
                .to_string(),
            );
        }),
        ("codex", "NotClaude", |case| {
            case.write_config(
                &case
                    .full_role()
                    .replace("harness: claude", "harness: codex"),
            );
        }),
    ];
    let mut problems = Vec::new();
    for (name, refusal, setup) in cases {
        let case = Case::new(name);
        setup(&case);
        let output = case.spawn();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.code() != Some(1)
            || !stderr.contains(&format!("refused: {refusal}:"))
            || stderr.lines().count() != 1
            || case.launches() != 0
            || case.recorded()
        {
            problems.push(format!(
                "{name}: expected exit 1 naming {refusal} on one line, nothing launched or \
                 recorded; {}, {} launch(es), store created: {}",
                describe(&output),
                case.launches(),
                case.recorded()
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// `story:instance-isolation`, item 1: a controller role whose settings file pins no instance, as
/// the shipped `.claude/controller-settings.json` the guides name does, starts with a copy of it
/// pinned to the instance, `<state>/sessions/<session name>.settings.json`; the file itself is
/// not changed. A file whose env pins the instance and one hook does not is pinned the same way.
#[test]
fn spawn_starts_unpinned_settings_with_a_pinned_copy() {
    let shipped = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.claude/controller-settings.json"),
    )
    .expect("the shipped controller settings");
    let hook_only = json!({
        "env": {"CONDUCTOR_INSTANCE": "example-org"},
        "hooks": {"PreToolUse": [{"matcher": "*", "hooks": [{
            "type": "command",
            "command": "conductor guard record-guard-decision --from-pre-tool-use",
        }]}]},
    })
    .to_string();
    for (name, text) in [("unpinned-shipped", shipped), ("unpinned-hook", hook_only)] {
        let case = Case::new(name);
        write(&case.settings(), &text);
        let output = case.spawn();
        assert!(output.status.success(), "{name}: {}", describe(&output));
        let copy = case.state().join("sessions/ex-alpha.settings.json");
        let argv = case.argv();
        let at = argv
            .iter()
            .position(|arg| arg == "--settings")
            .expect("a --settings argument");
        assert_eq!(
            argv[at + 1],
            copy.display().to_string(),
            "{name}: the controller starts with the pinned copy: {argv:?}"
        );
        let pinned: Value =
            serde_json::from_str(&fs::read_to_string(&copy).expect("the pinned copy"))
                .expect("JSON");
        assert_eq!(
            pinned["env"]["CONDUCTOR_INSTANCE"], "example-org",
            "{pinned}"
        );
        assert_eq!(
            pinned["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
                .as_str()
                .map(|command| command.starts_with("CONDUCTOR_INSTANCE=example-org conductor ")),
            Some(true),
            "{name}: {pinned}"
        );
        let original: Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(
            pinned["disableAllHooks"], original["disableAllHooks"],
            "{name}: {pinned}"
        );
        assert_eq!(
            fs::read_to_string(case.settings()).expect("the role's settings"),
            text,
            "{name}: the role's own file is not changed"
        );
    }
}

#[test]
fn spawn_refuses_a_repository_that_is_no_checkout_under_the_root() {
    let case = Case::new("outside");
    for repository in [
        "../alpha", "alpha/..", "-alpha", "", "a/b/c", "/alpha", "alpha/",
    ] {
        let output = case.run(&["spawn", &format!("--repository={repository}")]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.code() == Some(1) && stderr.contains("refused: NoCheckout:"),
            "{repository:?}: {}",
            describe(&output)
        );
    }
    assert_eq!(case.launches(), 0, "nothing is launched");
    assert!(!case.recorded(), "nothing is recorded");
}

#[test]
fn a_launch_that_fails_is_refused_and_the_controller_recorded_stopped() {
    let case = Case::new("launch-fails");
    write(&case.dir.join("fail"), "");
    let output = case.spawn();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        stderr.contains("refused: LaunchFailed:") && stderr.contains("Workspace not trusted"),
        "the refusal names its error and what claude said: {stderr}"
    );
    assert_eq!(case.launches(), 1);
    let rows = case.controllers();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0]["state"], "Stopped", "{}", rows[0]);

    // Stopped, the repository may be spawned again.
    fs::remove_file(case.dir.join("fail")).expect("let the launch succeed");
    let again = case.spawn();
    assert!(again.status.success(), "{}", describe(&again));
    assert_eq!(case.launches(), 2);
}

// Security review of `story:spawn` (wave 11). Each case below states the contract it checks.

/// Replaces the stand-in `claude` with its own script, `edit` applied to the text.
fn edit_stand_in(case: &Case, edit: impl FnOnce(String) -> String) {
    let path = case.dir.join("bin/claude");
    let text = fs::read_to_string(&path).expect("read the stand-in");
    let edited = edit(text.clone());
    assert_ne!(text, edited, "the stand-in was edited");
    fs::write(&path, edited).expect("write the stand-in");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

/// Writes `~/.claude.json` marking each of `paths` trusted.
fn trust_paths(case: &Case, paths: &[&Path]) {
    let projects: serde_json::Map<String, Value> = paths
        .iter()
        .map(|path| {
            (
                path.display().to_string(),
                json!({"hasTrustDialogAccepted": true}),
            )
        })
        .collect();
    write(
        &case.home().join(".claude.json"),
        &json!({ "projects": projects }).to_string(),
    );
}

/// `CONDUCTOR_CONFIG` handed to the session must name the config file spawn itself read, as the
/// session resolves it: the session runs in the checkout, not where spawn ran. A relative path
/// passed through unchanged names `<checkout>/<path>` instead, a file of the repository, and the
/// session's guard hook (`guard.rs` `config_file`, `cli.rs` `set_active`) then loads that file's
/// places and rules, or falls back to the built-in instance when it is absent.
#[test]
fn spawn_hands_the_session_the_config_file_it_read_when_given_a_relative_path() {
    let case = Case::new("relative-config");
    let launch = case.launch();
    edit_stand_in(&case, |text| {
        text.replacen(
            "pwd -P > ",
            &format!(
                "readlink -f \"$CONDUCTOR_CONFIG\" > '{}/config'\npwd -P > ",
                launch.display()
            ),
            1,
        )
    });
    let path = format!("{}:/usr/bin:/bin", case.dir.join("bin").display());
    let output = common::conductor(case.home())
        .args(["spawn", "--repository", "alpha"])
        .current_dir(&case.dir)
        .env("CONDUCTOR_CONFIG", "conductor.yaml")
        .env("PATH", path)
        .stdin(Stdio::null())
        .output()
        .expect("conductor runs");
    assert!(output.status.success(), "spawn: {}", describe(&output));
    assert_eq!(case.cwd(), case.checkout().display().to_string());
    let seen = fs::read_to_string(case.launch().join("config"))
        .unwrap_or_default()
        .trim_end()
        .to_owned();
    assert_eq!(
        seen,
        case.config().display().to_string(),
        "the session's CONDUCTOR_CONFIG, resolved in its working directory, names the config \
         file spawn read"
    );
}

/// `<checkouts.root>/<repo>` that is a symbolic link out of the root is not a checkout under the
/// root: spawn must not start a bypass-permissions session in the link's target, even when
/// `~/.claude.json` trusts the target (`conductor trust` canonicalises, so it marks the target).
#[test]
fn spawn_refuses_a_checkout_that_links_out_of_the_root() {
    let case = Case::new("symlink-out");
    // Not `beta`: the fixture's session list holds a live `ex-beta`.
    let outside = case.dir.join("elsewhere/gamma");
    write(&outside.join(".git/HEAD"), "ref: refs/heads/main\n");
    std::os::unix::fs::symlink(&outside, case.root().join("gamma")).expect("link gamma");
    write(&case.records().join("charters/gamma.md"), CHARTER);
    trust_paths(&case, &[case.checkout().as_path(), outside.as_path()]);
    let output = case.run(&["spawn", "--repository", "gamma"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(1) && stderr.contains("refused: NoCheckout:"),
        "spawn of a link out of the root: {}",
        describe(&output)
    );
    assert_eq!(case.launches(), 0, "nothing is launched outside the root");
    assert!(!case.recorded(), "nothing is recorded");
}

/// A config may put conductor's records in a git checkout directly under the checkouts root (the
/// guard names that layout, `guard/rules.rs` `Places::session`). The guard gives a session whose
/// working directory is in `<records>` conductor's rule, whatever its name; so spawn must not start
/// a bypass-permissions "controller" there.
#[test]
fn spawn_refuses_conductors_records_as_a_controller_checkout() {
    let case = Case::new("records-under-root");
    let records = case.root().join("conductor-records");
    write(&records.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&records.join("charters/conductor-records.md"), CHARTER);
    let config = fs::read_to_string(case.config()).expect("read the config");
    let moved = config.replace(
        &format!("records: {}", case.records().display()),
        &format!("records: {}", records.display()),
    );
    assert_ne!(config, moved, "the config names the records directory");
    write(&case.config(), &moved);
    trust_paths(&case, &[case.checkout().as_path(), records.as_path()]);
    let output = case.run(&["spawn", "--repository", "conductor-records"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(1)
            && stderr.contains("refused: ")
            && !["SessionRunning", "UntrustedCheckout", "NoCharter"]
                .iter()
                .any(|other| stderr.contains(&format!("refused: {other}:"))),
        "spawn of conductor's records is refused for being conductor's records: {}",
        describe(&output)
    );
    assert_eq!(
        case.launches(),
        0,
        "no session starts in conductor's records"
    );
    assert!(!case.recorded(), "nothing is recorded");
}

/// A launch that fails after `claude --bg` started the session (it exits non-zero, or hangs past
/// the bound and only the launcher process is killed, `spawn.rs` `run_launch`) leaves a live
/// session of the controller's name. The store must not then say the controller is stopped.
#[test]
fn a_failed_launch_whose_session_is_live_is_not_recorded_stopped() {
    let case = Case::new("orphan");
    write(&case.dir.join("fail"), "");
    let agents = case.dir.join("agents.json");
    edit_stand_in(&case, |text| {
        text.replacen(
            "if [ -e '",
            &format!(
                "printf '%s' '[{{\"id\":\"s-orphan\",\"name\":\"ex-alpha\",\"pid\":999,\
                 \"status\":\"busy\"}}]' > '{}'\nif [ -e '",
                agents.display()
            ),
            1,
        )
    });
    let output = case.spawn();
    assert_eq!(case.launches(), 1, "{}", describe(&output));
    let rows = case.controllers();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_ne!(
        rows[0]["state"],
        "Stopped",
        "a live session ex-alpha runs, yet the controller is recorded stopped: {}",
        describe(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// story:session-envelope-seam: spawn starts the controller through the confinement provider.
// ---------------------------------------------------------------------------------------------

/// Through a fake provider, one spawn is one `start` with the controller role's request (the
/// checkout, its places resolved to absolute paths) and its start form's arguments, and one
/// `prompt` with the charter; `claude` launches nothing, and the controller is recorded running.
#[test]
fn spawn_starts_through_the_provider_with_one_start_and_one_prompt() {
    let case = Case::new("provider");
    let output = case.spawn_fake();
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(call_names(&case.fake()), ["start", "prompt", "measure"]);
    let logged = calls(&case.fake());
    let request = &logged[0]["request"];
    let checkout = case.checkout().display().to_string();
    assert_eq!(request["role"], "controller", "{request:#}");
    assert_eq!(request["session_name"], "ex-alpha", "{request:#}");
    assert_eq!(request["cwd"], checkout.as_str(), "{request:#}");
    let places: Vec<(&str, &Value)> = request["writable"]
        .as_array()
        .expect("the places")
        .iter()
        .map(|place| (place["place"].as_str().unwrap_or_default(), &place["paths"]))
        .collect();
    assert_eq!(
        places.iter().map(|(word, _)| *word).collect::<Vec<_>>(),
        [
            "checkout",
            "trees",
            "build-cache",
            "scratch",
            "harness-state"
        ],
        "{request:#}"
    );
    assert_eq!(places[0].1, &json!([checkout]), "{request:#}");
    assert_eq!(
        places[1].1,
        &json!([case.dir.join("trees/alpha").display().to_string()]),
        "{request:#}"
    );
    let mut form = start_form(&case);
    form.remove(0);
    form.pop();
    assert_eq!(
        logged[0]["args"],
        json!(form),
        "the start form but --bg and the charter"
    );
    assert_eq!(
        logged[1]["prompt"], CHARTER,
        "the charter is the first prompt"
    );
    assert_eq!(case.launches(), 0, "claude launched nothing");
    let rows = case.controllers();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0]["state"], "Running", "{}", rows[0]);
}

/// The controller commands stop and resume the session through the provider: `pause-controller`
/// stops a live session through the provider its measurement names and records it `Paused`;
/// `resume-controller` resumes it through the instance's provider from the recorded id and
/// records it `Running`; `stop-controller` stops nothing once the session list shows none of the
/// name, and records it `Stopped`.
#[test]
fn the_controller_commands_stop_and_resume_the_session_through_the_provider() {
    let case = Case::new("provider-lifecycle");
    let spawned = case.spawn_fake();
    assert_eq!(spawned.status.code(), Some(0), "{}", describe(&spawned));
    let controller = case.controllers()[0]["controller_id"]
        .as_str()
        .expect("a controller id")
        .to_owned();
    let live = json!([{"id": "fake-ex-alpha", "name": "ex-alpha", "pid": 4242}]);

    case.sessions(&live);
    let paused = case.run_fake(&[
        "controller",
        "pause-controller",
        "--controller-id",
        &controller,
    ]);
    assert_eq!(paused.status.code(), Some(0), "{}", describe(&paused));
    assert_eq!(
        String::from_utf8_lossy(&paused.stdout),
        "stopped ex-alpha (fake-ex-alpha)\n"
    );
    let logged = calls(&case.fake());
    let stop = logged.last().expect("a call");
    assert_eq!(stop["call"], "stop", "{stop}");
    assert_eq!(stop["provider"], "none", "{stop}");
    assert_eq!(stop["session_id"], "fake-ex-alpha", "{stop}");
    assert_eq!(case.controllers()[0]["state"], "Paused");

    case.sessions(&json!([]));
    let resumed = case.run_fake(&[
        "controller",
        "resume-controller",
        "--controller-id",
        &controller,
    ]);
    assert_eq!(resumed.status.code(), Some(0), "{}", describe(&resumed));
    assert_eq!(
        String::from_utf8_lossy(&resumed.stdout),
        "resumed ex-alpha (fake-ex-alpha)\n"
    );
    let names = call_names(&case.fake());
    assert_eq!(
        &names[names.len() - 2..],
        ["resume", "measure"],
        "{names:?}"
    );
    assert_eq!(case.controllers()[0]["state"], "Running");

    let before = call_names(&case.fake()).len();
    let stopped = case.run_fake(&[
        "controller",
        "stop-controller",
        "--controller-id",
        &controller,
        "--reason",
        "the wave closed",
    ]);
    assert_eq!(stopped.status.code(), Some(0), "{}", describe(&stopped));
    assert_eq!(
        call_names(&case.fake()).len(),
        before,
        "no live session, nothing to stop"
    );
    assert_eq!(case.controllers()[0]["state"], "Stopped");
}

/// With `confinement.provider: none`, spawn launches today's start form, and the measurement it
/// writes says the controller runs unboxed.
#[test]
fn the_none_provider_keeps_the_start_form() {
    let case = Case::new("provider-none");
    case.confine("{provider: none}");
    let output = case.spawn();
    assert!(output.status.success(), "spawn: {}", describe(&output));
    assert_eq!(case.launches(), 1);
    assert_eq!(case.argv(), start_form(&case), "the controller start form");
    let record: Value = serde_json::from_str(
        &fs::read_to_string(case.state().join("sessions/ex-alpha.envelope.json"))
            .expect("the measurement is written"),
    )
    .expect("the measurement is JSON");
    assert!(record["unboxed"].is_string(), "{record:#}");
    assert_eq!(record["session"]["session_id"], SESSION_ID, "{record:#}");
}

/// Under `strict`, a provider that is not available refuses the spawn before anything is
/// recorded or started.
#[test]
fn strict_spawn_is_refused_when_the_provider_is_not_available() {
    let case = Case::new("strict-unavailable");
    case.confine("{mode: strict, provider: systemd}");
    fs::create_dir_all(case.fake()).expect("create the fake's directory");
    write(
        &case.fake().join("systemd.unavailable"),
        "no cgroup delegation",
    );
    let output = case.spawn_fake();
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("conductor: spawn: refused: ProviderUnavailable: ")
            && stderr.contains("no cgroup delegation"),
        "{}",
        describe(&output)
    );
    assert!(
        call_names(&case.fake()).is_empty(),
        "{:?}",
        calls(&case.fake())
    );
    assert!(!case.recorded(), "nothing is recorded");
    assert_eq!(case.launches(), 0);
}

/// Under `strict`, a box short of the request is stopped, the controller recorded stopped, and
/// the spawn refused as `MeasurementShort`.
#[test]
fn strict_spawn_is_refused_when_the_measurement_is_short() {
    let case = Case::new("strict-short");
    case.write_config(&format!(
        "{{role: controller, harness: claude, model: sonnet, profile: {}, settings: {}, limits: \
         {{tasks_max: 128}}}}",
        case.profile().display(),
        case.settings().display()
    ));
    case.confine("{mode: strict, provider: systemd}");
    fs::create_dir_all(case.fake()).expect("create the fake's directory");
    write(&case.fake().join("measure.json"), "{\"tasks_max\": null}");
    let output = case.spawn_fake();
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("conductor: spawn: refused: MeasurementShort: ")
            && stderr.contains("tasks_max"),
        "{}",
        describe(&output)
    );
    assert_eq!(
        call_names(&case.fake()),
        ["start", "prompt", "measure", "stop"]
    );
    let rows = case.controllers();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0]["state"], "Stopped", "{}", rows[0]);
}
