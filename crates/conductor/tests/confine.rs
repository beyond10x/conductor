//! `story:session-envelope-seam`: every session start goes through a confinement provider.
//!
//! - The place resolver turns each `writable` word into absolute paths, whether or not they
//!   exist, for a controller and for conductor.
//! - `conductor session start --role` over a fake provider ([`fake_provider`]): one case per
//!   declared refusal (`ProviderUnavailable` and `MeasurementShort` under `strict`,
//!   `UnknownRole`, `SessionRunning`, `LaunchFailed`), the unboxed start under `report`, the
//!   request the provider is given, and the measurement written under the state directory and
//!   read into `conductor snapshot sessions`.
//! - `conductor session stop --role` stops through the provider that started the session, and
//!   refuses an unknown role and a role with no live session.
//!
//! Each case is a directory of its own: `home/` (the `HOME` the command runs with), `records/`,
//! `state/`, `work/` (the working directory), `fake/` (the fake provider's files) and `bin/`
//! with a `claude` whose `agents --json` prints the case's `agents.json` and which logs any other
//! call to `claude.log` and does nothing. No session starts.

mod common;
mod fake_provider;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::{Duration, SystemTime};

use conductor_cli::collect::Collector;
use conductor_cli::collect::sessions::{self, Sources};
use conductor_cli::config;
use conductor_cli::confine::places::{self, Placement};
use conductor_cli::session::{CONDUCTOR_PROMPT, DEV_PROMPT};
use conductor_cli::snapshot;
use conductor_model::config::{Checkouts, Harness, Writable};
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

/// A case directory of its own; see the module documentation.
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

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

impl Case {
    /// The case `name`, its instance's `confinement` written as `confinement`.
    fn new(name: &str, confinement: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("confine")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "records", "state", "work", "fake", "bin"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        write(
            &case.config(),
            &format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: alpha\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n\
                 \x20   roles:\n\
                 \x20     - {{role: conductor, harness: claude, model: opus, limits: \
                 {{memory_max: 4G, tasks_max: 256}}, writable: [checkout, records], \
                 forge_token_env: [GH_TOKEN]}}\n\
                 \x20     - {{role: conductor-dev, harness: claude, model: sonnet, writable: \
                 [checkout]}}\n\
                 \x20     - {{role: controller, harness: claude, model: opus}}\n\
                 \x20   confinement: {confinement}\n",
                root = case.home().join("example-org").display(),
                trees = case.dir.join("trees").display(),
                records = case.records().display(),
                state = case.state().display(),
            ),
        );
        case.agents(&json!([]));
        let claude = case.dir.join("bin/claude");
        write(
            &claude,
            &format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = agents ]; then cat '{agents}'; exit 0; fi\n\
                 printf '%s\\n' \"$*\" >> '{log}'\n",
                agents = case.dir.join("agents.json").display(),
                log = case.dir.join("claude.log").display(),
            ),
        );
        fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn work(&self) -> PathBuf {
        self.dir.join("work")
    }

    fn fake(&self) -> PathBuf {
        self.dir.join("fake")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// The live sessions `claude agents --json` prints.
    fn agents(&self, sessions: &Value) {
        write(&self.dir.join("agents.json"), &sessions.to_string());
    }

    /// Writes `text` to the fake directory's file `name`.
    fn fake_file(&self, name: &str, text: &str) {
        write(&self.fake().join(name), text);
    }

    /// `conductor --config <case> <args>` over the fake providers, from `work/`.
    fn run(&self, args: &[&str]) -> Output {
        let config = self.config();
        let mut all = vec!["--config", config.to_str().expect("a UTF-8 path")];
        all.extend_from_slice(args);
        let path = format!("{}:/usr/bin:/bin", self.dir.join("bin").display());
        let output = fake_provider::command(&self.home(), &self.fake(), &all)
            .current_dir(self.work())
            .env("PATH", path)
            .env("TMPDIR", self.dir.join("tmp"))
            .output()
            .expect("the child runs");
        fake_provider::without_harness(output)
    }

    fn start(&self, role: &str) -> Output {
        self.run(&["session", "start", "--role", role])
    }

    fn stop(&self, role: &str) -> Output {
        self.run(&["session", "stop", "--role", role])
    }

    /// What the fake `claude` was asked to do but list sessions.
    fn claude_calls(&self) -> String {
        fs::read_to_string(self.dir.join("claude.log")).unwrap_or_default()
    }

    /// The measurement of the session `name`, if one was written.
    fn record(&self, name: &str) -> Option<Value> {
        let file = self
            .state()
            .join("sessions")
            .join(format!("{name}.envelope.json"));
        fs::read_to_string(file)
            .ok()
            .map(|text| serde_json::from_str(&text).expect("the record is JSON"))
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Asserts `output` is a refusal of `command` named `refusal`: exit 1, one line naming it.
fn assert_refused(output: &Output, command: &str, refusal: &str) {
    assert_eq!(output.status.code(), Some(1), "{}", describe(output));
    let text = stderr(output);
    assert!(
        text.starts_with(&format!("conductor: {command}: refused: {refusal}: ")),
        "{}",
        describe(output)
    );
    assert_eq!(text.lines().count(), 1, "{}", describe(output));
}

// -------------------------------------------------------------------------------------------
// The place resolver.
// -------------------------------------------------------------------------------------------

fn words(list: &[&str]) -> Vec<Writable> {
    list.iter()
        .map(|word| Writable((*word).to_owned()))
        .collect()
}

/// Each place word of a controller resolves to its absolute paths, a path entry stays as
/// written, and none of them needs to exist.
#[test]
fn places_resolve_each_word_for_a_controller() {
    let resolved = places::resolve(
        &words(&[
            "checkout",
            "trees",
            "build-cache",
            "scratch",
            "harness-state",
            "records",
            "/srv/cache",
        ]),
        &Placement {
            home: PathBuf::from("/h"),
            tmpdir: PathBuf::from("/t"),
            start: PathBuf::from("/h/example-org/group/alpha"),
            repository: Some("group/alpha".to_owned()),
            trees: PathBuf::from("/h/trees"),
            records: PathBuf::from("/h/records"),
            harness: Harness::Claude,
            controller: true,
        },
    );
    let shown: Vec<(String, Vec<String>)> = resolved
        .into_iter()
        .map(|place| (place.place.0, place.paths))
        .collect();
    let expected: Vec<(String, Vec<String>)> = [
        ("checkout", vec!["/h/example-org/group/alpha"]),
        ("trees", vec!["/h/trees/group/alpha"]),
        ("build-cache", vec!["/h/.cargo", "/h/.npm"]),
        ("scratch", vec!["/t", "/h/.cache/alpha-*"]),
        ("harness-state", vec!["/h/.claude", "/h/.claude.json"]),
        ("records", vec!["/h/records"]),
        ("/srv/cache", vec!["/srv/cache"]),
    ]
    .into_iter()
    .map(|(word, paths)| {
        (
            word.to_owned(),
            paths.into_iter().map(str::to_owned).collect(),
        )
    })
    .collect();
    assert_eq!(shown, expected);
}

/// Conductor's places: it starts in the records directory, which is its checkout; its scratch
/// is `$TMPDIR` only; a `codex` harness's state is `~/.codex`.
#[test]
fn places_resolve_each_word_for_conductor() {
    let at = Placement {
        home: PathBuf::from("/h"),
        tmpdir: PathBuf::from("/t"),
        start: PathBuf::from("/h/.b10x/conductor/alpha/records"),
        repository: None,
        trees: PathBuf::from("/h/trees"),
        records: PathBuf::from("/h/.b10x/conductor/alpha/records"),
        harness: Harness::Codex,
        controller: false,
    };
    let resolved = places::resolve(
        &words(&["checkout", "trees", "scratch", "harness-state", "records"]),
        &at,
    );
    let shown: Vec<Vec<String>> = resolved.into_iter().map(|place| place.paths).collect();
    assert_eq!(
        shown,
        [
            vec!["/h/.b10x/conductor/alpha/records"],
            vec!["/h/trees/records"],
            vec!["/t"],
            vec!["/h/.codex"],
            vec!["/h/.b10x/conductor/alpha/records"],
        ]
        .map(|paths| paths.into_iter().map(str::to_owned).collect::<Vec<_>>())
    );
}

// -------------------------------------------------------------------------------------------
// `session start`.
// -------------------------------------------------------------------------------------------

const STRICT: &str = "{mode: strict, provider: systemd}";
const REPORT: &str = "{mode: report, provider: systemd}";

/// Under `strict`, a provider that is not available starts nothing: the refusal names the
/// reason, the provider is asked for no start, and nothing is written.
#[test]
fn strict_and_a_provider_not_available_starts_nothing_naming_why() {
    let case = Case::new("strict-unavailable", STRICT);
    case.fake_file("systemd.unavailable", "no delegated cgroup controllers");
    let output = case.start("conductor");
    assert_refused(&output, "session start", "ProviderUnavailable");
    assert!(
        stderr(&output).contains("no delegated cgroup controllers"),
        "{}",
        describe(&output)
    );
    assert!(
        call_names(&case.fake()).is_empty(),
        "{:?}",
        calls(&case.fake())
    );
    assert_eq!(case.claude_calls(), "");
    assert_eq!(case.record("conductor"), None);
}

/// Under `strict`, a box whose measurement is short of the request is stopped again, and the
/// command exits 1 naming what is short; no measurement is kept.
#[test]
fn strict_and_a_short_measurement_stops_the_session() {
    let case = Case::new("strict-short", STRICT);
    case.fake_file("measure.json", "{\"memory_max\": null}");
    let output = case.start("conductor");
    assert_refused(&output, "session start", "MeasurementShort");
    assert!(
        stderr(&output).contains("memory_max"),
        "{}",
        describe(&output)
    );
    assert_eq!(
        call_names(&case.fake()),
        ["start", "prompt", "measure", "stop"]
    );
    assert_eq!(case.record("conductor"), None);
}

/// A role that `session start` does not start, or that the instance does not name, is refused
/// before anything starts.
#[test]
fn an_unknown_role_is_refused() {
    let case = Case::new("unknown-role", STRICT);
    for role in ["controller", "nobody"] {
        let output = case.start(role);
        assert_refused(&output, "session start", "UnknownRole");
    }
    assert!(
        call_names(&case.fake()).is_empty(),
        "{:?}",
        calls(&case.fake())
    );
}

/// A live session that holds the role's session name is refused before anything starts.
#[test]
fn a_session_already_running_under_the_name_is_refused() {
    let case = Case::new("running", STRICT);
    case.agents(&json!([
        {"id": "s-live", "name": "conductor", "cwd": case.records(), "pid": 4242,
         "kind": "background", "status": "idle"}
    ]));
    let output = case.start("conductor");
    assert_refused(&output, "session start", "SessionRunning");
    assert!(stderr(&output).contains("s-live"), "{}", describe(&output));
    assert!(
        call_names(&case.fake()).is_empty(),
        "{:?}",
        calls(&case.fake())
    );
}

/// A start that fails is refused naming what the provider said; nothing is measured.
#[test]
fn a_launch_that_fails_is_refused() {
    let case = Case::new("launch-failed", STRICT);
    case.fake_file("start.fails", "the unit failed to start");
    let output = case.start("conductor");
    assert_refused(&output, "session start", "LaunchFailed");
    assert!(
        stderr(&output).contains("the unit failed to start"),
        "{}",
        describe(&output)
    );
    assert_eq!(call_names(&case.fake()), ["start"]);
    assert_eq!(case.record("conductor"), None);
}

/// Under `report`, a provider that is not available starts the session unboxed through `none`,
/// and its measurement says so with the reason.
#[test]
fn report_and_a_provider_not_available_starts_unboxed_with_the_reason() {
    let case = Case::new("report-unavailable", REPORT);
    case.fake_file("systemd.unavailable", "no delegated cgroup controllers");
    let output = case.start("conductor");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let providers: Vec<String> = calls(&case.fake())
        .iter()
        .map(|call| format!("{} {}", call["call"], call["provider"]))
        .collect();
    assert_eq!(
        providers,
        [
            "\"start\" \"none\"",
            "\"prompt\" \"none\"",
            "\"measure\" \"none\""
        ]
    );
    let record = case.record("conductor").expect("a measurement");
    let unboxed = record["unboxed"].as_str().expect("unboxed with a reason");
    assert!(
        unboxed.contains("systemd") && unboxed.contains("no delegated cgroup controllers"),
        "{record:#}"
    );
    for field in ["provider", "cgroup", "memory_max", "writable"] {
        assert_eq!(record[field], Value::Null, "{field}: {record:#}");
    }
    assert_eq!(record["session"]["provider"], "none", "{record:#}");
}

/// The provider is given the role's request (absolute places, limits, forge token variables) and
/// harness arguments, and conductor's first prompt; conductor starts in the records directory,
/// conductor-dev in the working directory, each with its own prompt. The started session is
/// printed.
#[test]
fn the_provider_is_given_the_role_s_request_and_first_prompt() {
    let case = Case::new("request", STRICT);
    let output = case.start("conductor");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "started conductor: provider systemd, unit conductor-conductor.service, session \
         fake-conductor\n"
    );
    let logged = calls(&case.fake());
    let records = case.records().display().to_string();
    assert_eq!(
        logged[0]["request"],
        json!({
            "role": "conductor", "session_name": "conductor", "cwd": records,
            "memory_max": 4, "cpu_weight": null, "tasks_max": 256,
            "writable": [
                {"place": "checkout", "paths": [records]},
                {"place": "records", "paths": [records]},
            ],
            "forge_token_env": ["GH_TOKEN"],
        })
    );
    let settings = case.work().join(".claude/conductor-settings.json");
    assert_eq!(
        logged[0]["args"],
        json!([
            "-n",
            "conductor",
            "--agent",
            "conductor",
            "--model",
            "opus",
            "--permission-mode",
            "bypassPermissions",
            "--setting-sources",
            "project,local",
            "--settings",
            settings.display().to_string(),
        ])
    );
    assert_eq!(logged[1]["prompt"], CONDUCTOR_PROMPT);
    assert_eq!(case.claude_calls(), "", "the fake provider ran no claude");

    let output = case.start("conductor-dev");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let logged = calls(&case.fake());
    let dev = logged
        .iter()
        .find(|call| call["call"] == "start" && call["request"]["role"] == "conductor-dev")
        .expect("a start of conductor-dev");
    assert_eq!(dev["request"]["cwd"], case.work().display().to_string());
    assert_eq!(
        dev["args"],
        json!([
            "-n",
            "conductor-dev",
            "--agent",
            "conductor-dev",
            "--model",
            "sonnet",
            "--permission-mode",
            "bypassPermissions",
            "--setting-sources",
            "project,local",
        ])
    );
    assert!(
        logged
            .iter()
            .any(|call| call["call"] == "prompt" && call["prompt"] == DEV_PROMPT),
        "{logged:#?}"
    );
}

/// The measurement of a started session is written to `<state>/sessions/<name>.envelope.json`,
/// and a snapshot reads it into that session's `envelope_*` fields of `conductor snapshot
/// sessions`.
#[test]
fn the_measurement_is_written_and_read_into_the_sessions_view() {
    let case = Case::new("measured", STRICT);
    let output = case.start("conductor");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let records = case.records().display().to_string();
    let record = case.record("conductor").expect("a measurement");
    assert_eq!(
        record["session"]["session_id"], "fake-conductor",
        "{record:#}"
    );
    assert_eq!(record["places"][0]["paths"], json!([records]), "{record:#}");

    // The session list now shows the session the provider started.
    let list = case.dir.join("list.json");
    write(
        &list,
        &json!([
            {"id": "fake-conductor", "name": "conductor", "cwd": records, "pid": 4242,
             "kind": "background", "status": "idle"}
        ])
        .to_string(),
    );
    let sources = Sources {
        agents: vec![OsString::from("cat"), list.into_os_string()],
        codex: case.dir.join("no-codex"),
        home: case.home(),
        now: SystemTime::now(),
        bound: Duration::from_secs(30),
    };
    let checkouts = Checkouts {
        root: case.home().join("example-org").display().to_string(),
        trees: case.dir.join("trees").display().to_string(),
    };
    let records_dir = case.records();
    let collector = Collector::new("sessions", move |record| {
        sessions::collect_excluding(&sources, &checkouts, Some(&records_dir), &[], record)
    });
    snapshot::take_in(&case.state(), &[collector]).expect("the snapshot is taken");

    let view = common::conductor(case.home())
        .current_dir(case.work())
        .arg("--state-dir")
        .arg(case.state())
        .args(["snapshot", "sessions", "--format", "jsonl"])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert!(view.status.success(), "{}", describe(&view));
    let rows: Vec<Value> = String::from_utf8_lossy(&view.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("a JSON row"))
        .collect();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let row = &rows[0];
    assert_eq!(row["name"], "conductor", "{row:#}");
    assert_ne!(row["envelope_provider"], Value::Null, "{row:#}");
    assert_eq!(row["envelope_cgroup"], "/fake/conductor", "{row:#}");
    assert_eq!(row["envelope_memory_max"], 4_i64 << 30, "{row:#}");
    assert_eq!(row["envelope_tasks_max"], 256, "{row:#}");
    assert_eq!(row["envelope_cpu_weight"], Value::Null, "{row:#}");
    assert_eq!(
        row["envelope_writable"],
        format!("{records}:{records}"),
        "{row:#}"
    );
    assert_eq!(row["envelope_unboxed"], Value::Null, "{row:#}");
}

// -------------------------------------------------------------------------------------------
// `session stop`.
// -------------------------------------------------------------------------------------------

/// `session stop` stops the role's live session through the provider its measurement names,
/// once, with the list's id, and removes the measurement.
#[test]
fn session_stop_stops_through_the_provider_that_started_it() {
    let case = Case::new("stop", STRICT);
    let output = case.start("conductor");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    case.agents(&json!([
        {"id": "fake-conductor", "name": "conductor", "cwd": case.records(), "pid": 4242,
         "kind": "background", "status": "idle"}
    ]));
    let output = case.stop("conductor");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let stops: Vec<Value> = calls(&case.fake())
        .into_iter()
        .filter(|call| call["call"] == "stop")
        .collect();
    assert_eq!(stops.len(), 1, "{stops:#?}");
    assert_eq!(stops[0]["provider"], "systemd");
    assert_eq!(stops[0]["session_id"], "fake-conductor");
    assert_eq!(stops[0]["cwd"], case.records().display().to_string());
    assert_eq!(case.record("conductor"), None, "the measurement is removed");
}

/// `session stop` refuses a role it does not stop and a role with no live session, stopping
/// nothing.
#[test]
fn session_stop_refuses_an_unknown_role_and_no_session_running() {
    let case = Case::new("stop-refused", STRICT);
    assert_refused(&case.stop("controller"), "session stop", "UnknownRole");
    assert_refused(&case.stop("conductor"), "session stop", "NotRunning");
    assert!(
        call_names(&case.fake()).is_empty(),
        "{:?}",
        calls(&case.fake())
    );
}
