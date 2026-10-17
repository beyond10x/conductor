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
                 [checkout], settings: {dev_settings}}}\n\
                 \x20     - {{role: controller, harness: claude, model: opus}}\n\
                 \x20   confinement: {confinement}\n",
                root = case.home().join("example-org").display(),
                trees = case.dir.join("trees").display(),
                records = case.records().display(),
                state = case.state().display(),
                dev_settings = case.dev_settings().display(),
            ),
        );
        // `story:instance-isolation`: conductor-dev's own settings pin the instance; conductor names
        // none, and starts with the default of the working directory, which pins none, written
        // pinned under the state directory.
        write(&case.conductor_settings(), SHIPPED);
        write(&case.dev_settings(), &pinned("alpha"));
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

    /// The conductor role's default settings, `.claude/conductor-settings.json` of the working
    /// directory, as this checkout ships them: no instance pinned.
    fn conductor_settings(&self) -> PathBuf {
        self.work().join(".claude/conductor-settings.json")
    }

    /// The conductor-dev role's settings file.
    fn dev_settings(&self) -> PathBuf {
        self.dir.join("settings/conductor-dev.json")
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

/// Settings as this checkout ships them: a hook that runs `conductor` for no instance.
const SHIPPED: &str = r#"{"hooks": {"PreToolUse": [{"matcher": "*", "hooks": [{"type": "command",
  "command": "conductor guard record-guard-decision --from-pre-tool-use || exit 2"}]}]}}"#;

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
            cache: PathBuf::from("/h/.cache/b10x/conductor/alpha"),
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
        (
            "scratch",
            vec!["/t", "/h/.cache/b10x/conductor/alpha/scratch/group-alpha"],
        ),
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
        cache: PathBuf::from("/h/.cache/b10x/conductor/alpha"),
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

impl Case {
    /// Names `own` as the conductor role's own settings file in the config file.
    fn own_settings(&self, own: &Path) {
        let config = fs::read_to_string(self.config()).expect("read the config");
        write(
            &self.config(),
            &config.replace(
                "forge_token_env: [GH_TOKEN]}",
                &format!("forge_token_env: [GH_TOKEN], settings: {}}}", own.display()),
            ),
        );
    }
}

/// `story:instance-isolation`, item 1: a role whose own settings file is not there, or pins
/// another instance than the role's (its `env.CONDUCTOR_INSTANCE`, or a hook command prefixed
/// with another), is refused before anything starts: its hooks would act for that instance. One
/// that pins no instance starts with a pinned copy
/// (`a_role_whose_settings_pin_no_instance_starts_with_a_pinned_copy`), and one with no settings
/// of its own starts pinned (`the_provider_is_given_the_role_s_request_and_first_prompt`).
#[test]
fn a_role_whose_settings_pin_another_instance_is_refused() {
    let case = Case::new("not-pinned", STRICT);
    let own = case.dir.join("settings/conductor.json");
    case.own_settings(&own);
    let foreign_hook = json!({
        "env": {"CONDUCTOR_INSTANCE": "alpha"},
        "hooks": {"PreToolUse": [{"matcher": "*", "hooks": [{
            "type": "command",
            "command": "CONDUCTOR_INSTANCE=beta conductor guard record-guard-decision \
                        --from-pre-tool-use || exit 2",
        }]}]},
    })
    .to_string();
    for (what, settings) in [
        ("another instance", pinned("beta")),
        ("a hook of another instance", foreign_hook),
        ("no JSON", "{\n".to_owned()),
    ] {
        write(&own, &settings);
        let output = case.start("conductor");
        assert_refused(&output, "session start", "InstanceNotPinned");
        assert!(
            stderr(&output).contains("CONDUCTOR_INSTANCE=alpha"),
            "{what}: the refusal names what to write: {}",
            describe(&output)
        );
    }
    assert!(
        !case
            .state()
            .join("sessions/conductor.settings.json")
            .exists(),
        "a refused start writes no pinned copy"
    );
    fs::remove_file(&own).expect("remove the settings");
    assert_refused(
        &case.start("conductor"),
        "session start",
        "InstanceNotPinned",
    );
    assert!(
        call_names(&case.fake()).is_empty(),
        "{:?}",
        calls(&case.fake())
    );
}

/// `story:instance-isolation`, item 1: a role whose own settings file pins no instance, as the
/// shipped `.claude/conductor-settings.json` the guides name does, starts with a copy of it pinned
/// to the role's instance, `<state>/sessions/<session name>.settings.json`: every other key kept,
/// `env.CONDUCTOR_INSTANCE` set and each hook command that runs `conductor` prefixed
/// `CONDUCTOR_INSTANCE=alpha `. The file itself is not changed. A file that pins the instance
/// already starts as it is.
#[test]
fn a_role_whose_settings_pin_no_instance_starts_with_a_pinned_copy() {
    let shipped = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.claude/conductor-settings.json"),
    )
    .expect("the shipped conductor settings");
    let env_only = json!({
        "env": {"CONDUCTOR_INSTANCE": "alpha", "TMPDIR": "/tmp/alpha"},
        "hooks": {"PreToolUse": [{"matcher": "*", "hooks": [{
            "type": "command",
            "command": "conductor guard record-guard-decision --from-pre-tool-use || exit 2",
        }]}]},
    })
    .to_string();
    for (name, text) in [
        ("unpinned-shipped", shipped),
        ("unpinned-empty", "{}\n".to_owned()),
        ("unpinned-hook", env_only),
    ] {
        let case = Case::new(name, STRICT);
        let own = case.dir.join("settings/conductor.json");
        case.own_settings(&own);
        write(&own, &text);
        let output = case.start("conductor");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{name}: {}",
            describe(&output)
        );
        let copy = case.state().join("sessions/conductor.settings.json");
        let args = &calls(&case.fake())[0]["args"];
        assert_eq!(
            args.as_array().and_then(|args| args.last()),
            Some(&json!(copy.display().to_string())),
            "{name}: the session starts with the pinned copy: {args}"
        );
        let original: Value = serde_json::from_str(&text).expect("JSON");
        let pinned: Value =
            serde_json::from_str(&fs::read_to_string(&copy).expect("the pinned copy"))
                .expect("JSON");
        assert_eq!(
            pinned["env"]["CONDUCTOR_INSTANCE"], "alpha",
            "{name}: {pinned}"
        );
        for (key, value) in original.as_object().expect("an object") {
            if key == "env" || key == "hooks" {
                continue;
            }
            assert_eq!(&pinned[key], value, "{name}: {key} is kept: {pinned}");
        }
        if let Some(tmp) = original["env"].get("TMPDIR") {
            assert_eq!(&pinned["env"]["TMPDIR"], tmp, "{name}: {pinned}");
        }
        let hooks = pinned["hooks"].as_object().cloned().unwrap_or_default();
        assert_eq!(
            hooks.len(),
            original["hooks"]
                .as_object()
                .map_or(0, serde_json::Map::len),
            "{name}: {pinned}"
        );
        for matchers in hooks.values() {
            for matcher in matchers.as_array().expect("matchers") {
                for hook in matcher["hooks"].as_array().expect("hooks") {
                    let command = hook["command"].as_str().expect("a command");
                    assert!(
                        command.starts_with("CONDUCTOR_INSTANCE=alpha conductor "),
                        "{name}: {command}"
                    );
                }
            }
        }
        assert_eq!(
            fs::read_to_string(&own).expect("the own settings"),
            text,
            "{name}: the role's own file is not changed"
        );
    }

    // Pinned already: it starts with the file itself, and no copy is written.
    let case = Case::new("pinned-own", STRICT);
    let own = case.dir.join("settings/conductor.json");
    case.own_settings(&own);
    write(&own, &pinned("alpha"));
    let output = case.start("conductor");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let args = &calls(&case.fake())[0]["args"];
    assert_eq!(
        args.as_array().and_then(|args| args.last()),
        Some(&json!(own.display().to_string())),
        "{args}"
    );
    assert!(
        !case
            .state()
            .join("sessions/conductor.settings.json")
            .exists()
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

/// `story:instance-names-unique`: a session start writes its config file to the host registry,
/// `~/.b10x/conductor/host/instances.json`, with the file's instances, even for an instance that
/// has no `session_prefix` yet: it still starts, under its bare name, for one release.
#[test]
fn a_session_start_writes_its_config_file_to_the_host_registry() {
    let case = Case::new("registry", REPORT);
    case.fake_file("systemd.unavailable", "no delegated cgroup controllers");
    let output = case.start("conductor");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let file = case.home().join(".b10x/conductor/host/instances.json");
    let registry: Value = serde_json::from_str(
        &fs::read_to_string(&file).unwrap_or_else(|error| panic!("{}: {error}", file.display())),
    )
    .expect("the registry is JSON");
    let files = registry["files"].as_array().expect("the registry's files");
    assert_eq!(files.len(), 1, "{registry:#}");
    assert_eq!(
        files[0]["config"],
        case.config().display().to_string(),
        "{registry:#}"
    );
    let alpha = &files[0]["instances"][0];
    assert_eq!(alpha["name"], "alpha", "{registry:#}");
    assert_eq!(alpha["session_prefix"], Value::Null, "{registry:#}");
    assert_eq!(
        alpha["state"],
        case.state().display().to_string(),
        "{registry:#}"
    );
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
    // Conductor names no settings: it starts with the working directory's, pinned to its
    // instance under the state directory (`story:instance-isolation`).
    let settings = case.state().join("sessions/conductor.settings.json");
    let pinned: Value =
        serde_json::from_str(&fs::read_to_string(&settings).expect("the pinned settings"))
            .expect("JSON");
    assert_eq!(pinned["env"]["CONDUCTOR_INSTANCE"], "alpha", "{pinned}");
    assert_eq!(
        pinned["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
        "CONDUCTOR_INSTANCE=alpha conductor guard record-guard-decision --from-pre-tool-use || \
         exit 2",
        "{pinned}"
    );
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
            "--settings",
            case.dev_settings().display().to_string(),
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

/// `story:role-name-aliases`, item 3: `--role director` and `--role conductor` start the same
/// role, the file's `conductor`, under its session name as today; the old name is noted on
/// standard error, the new one is not. `--role maintainer` starts the file's `conductor-dev`.
#[test]
fn director_and_conductor_start_the_same_role_the_old_name_with_a_note() {
    for (name, role, noted) in [
        ("role-director", "director", false),
        ("role-conductor", "conductor", true),
    ] {
        let case = Case::new(name, STRICT);
        let output = case.start(role);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{role}: {}",
            describe(&output)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "started conductor: provider systemd, unit conductor-conductor.service, session \
             fake-conductor\n",
            "{role}"
        );
        let logged = calls(&case.fake());
        assert_eq!(
            logged[0]["request"]["cwd"],
            case.records().display().to_string()
        );
        assert_eq!(logged[1]["prompt"], CONDUCTOR_PROMPT, "{role}");
        let note = stderr(&output);
        assert_eq!(
            note.contains("--role conductor is the old name of director"),
            noted,
            "{role}: {}",
            describe(&output)
        );
    }
    let case = Case::new("role-maintainer", STRICT);
    let output = case.start("maintainer");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stdout).starts_with("started conductor-dev: "),
        "{}",
        describe(&output)
    );
    assert!(stderr(&output).is_empty(), "{}", describe(&output));
}

/// `story:role-name-aliases`, item 3: a file that writes the new name `director` starts its
/// session as `director`, with the agent `conductor` (the adapter's file name for this release);
/// while a session still named `conductor` lives, start refuses it as running, and `session stop
/// --role director` and `--role conductor` stop that session as the director's.
#[test]
fn a_director_finds_a_live_session_still_named_conductor() {
    let case = Case::new("director-named", STRICT);
    let config = fs::read_to_string(case.config()).expect("read the config");
    write(
        &case.config(),
        &config.replace("{role: conductor,", "{role: director,"),
    );
    let output = case.start("director");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stdout).starts_with("started director: "),
        "{}",
        describe(&output)
    );
    let start = calls(&case.fake())
        .into_iter()
        .find(|call| call["call"] == "start")
        .expect("a start");
    let args = start["args"].as_array().expect("args");
    let agent = args
        .iter()
        .position(|arg| arg == "--agent")
        .expect("an --agent");
    assert_eq!(args[agent + 1], "conductor", "{args:?}");

    for role in ["director", "conductor"] {
        let case = Case::new(&format!("director-stops-{role}"), STRICT);
        let config = fs::read_to_string(case.config()).expect("read the config");
        write(
            &case.config(),
            &config.replace("{role: conductor,", "{role: director,"),
        );
        case.agents(&json!([
            {"id": "s-old", "name": "conductor", "cwd": case.records(), "pid": 4242,
             "kind": "background", "status": "idle"}
        ]));
        let output = case.start("director");
        assert_refused(&output, "session start", "SessionRunning");
        assert!(stderr(&output).contains("s-old"), "{}", describe(&output));
        let output = case.stop(role);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{role}: {}",
            describe(&output)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "stopped conductor (s-old)\n",
            "{role}"
        );
        let stops: Vec<Value> = calls(&case.fake())
            .into_iter()
            .filter(|call| call["call"] == "stop")
            .collect();
        assert_eq!(stops.len(), 1, "{role}: {stops:#?}");
        assert_eq!(stops[0]["session_id"], "s-old", "{role}");
    }
}

/// `story:maintainer-host-config`, item 4: with `host.yaml` naming a maintainer, `session start
/// --role maintainer` (and `--role conductor-dev`, with its note) starts that role, not the one
/// the instance's `roles` still writes: under the session name `maintainer`, with no instance
/// prefix, its model and places, in the working directory with the maintainer's first prompt,
/// pinned to the instance it runs for; `session stop --role maintainer` stops it.
#[test]
fn the_maintainer_starts_from_the_host_file() {
    for (name, role) in [
        ("host-maintainer", "maintainer"),
        ("host-maintainer-old-name", "conductor-dev"),
    ] {
        let case = Case::new(name, STRICT);
        write(
            &case.home().join(".b10x/conductor/host/host.yaml"),
            &format!(
                "version: conductor.host-config/1\n\
                 roles:\n\
                 \x20 - {{role: maintainer, harness: claude, model: haiku, writable: [checkout], \
                 settings: {settings}}}\n",
                settings = case.dev_settings().display(),
            ),
        );
        let output = case.start(role);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{role}: {}",
            describe(&output)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).starts_with("started maintainer: "),
            "{role}: {}",
            describe(&output)
        );
        assert_eq!(
            stderr(&output).contains("--role conductor-dev is the old name of maintainer"),
            role == "conductor-dev",
            "{role}: {}",
            describe(&output)
        );
        let logged = calls(&case.fake());
        let start = logged
            .iter()
            .find(|call| call["call"] == "start")
            .expect("a start");
        let work = case.work().display().to_string();
        assert_eq!(
            start["request"],
            json!({
                "role": "maintainer", "session_name": "maintainer", "cwd": work,
                "memory_max": null, "cpu_weight": null, "tasks_max": null,
                "writable": [{"place": "checkout", "paths": [work]}],
                "forge_token_env": [],
            }),
            "{role}"
        );
        assert_eq!(
            start["args"],
            json!([
                "-n",
                "maintainer",
                "--agent",
                "conductor-dev",
                "--model",
                "haiku",
                "--permission-mode",
                "bypassPermissions",
                "--setting-sources",
                "project,local",
                "--settings",
                case.dev_settings().display().to_string(),
            ]),
            "{role}"
        );
        assert!(
            logged
                .iter()
                .any(|call| call["call"] == "prompt" && call["prompt"] == DEV_PROMPT),
            "{role}: {logged:#?}"
        );

        case.agents(&json!([
            {"id": "s-maintainer", "name": "maintainer", "cwd": case.work(), "pid": 4242,
             "kind": "background", "status": "idle"}
        ]));
        assert_refused(&case.start(role), "session start", "SessionRunning");
        let output = case.stop(role);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{role}: {}",
            describe(&output)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "stopped maintainer (s-maintainer)\n",
            "{role}"
        );
    }
}

/// `story:maintainer-host-config`, item 4: a `host.yaml` that does not read refuses `session start
/// --role maintainer` (`UnknownRole`), naming the file, and starts nothing; the director starts
/// as before, reading no host file.
#[test]
fn a_host_file_that_does_not_read_refuses_the_maintainer_only() {
    let case = Case::new("host-maintainer-invalid", STRICT);
    let host = case.home().join(".b10x/conductor/host/host.yaml");
    write(
        &host,
        "version: conductor.host-config/1\nroles:\n  - {role: director, harness: claude, model: opus}\n",
    );
    let output = case.start("maintainer");
    assert_refused(&output, "session start", "UnknownRole");
    assert!(
        stderr(&output).contains(&host.display().to_string()),
        "{}",
        describe(&output)
    );
    assert!(
        !calls(&case.fake())
            .iter()
            .any(|call| call["call"] == "start"),
        "nothing started"
    );
    let output = case.start("director");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
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

/// `story:scratch-per-instance`: two instances that track a repository of the same name resolve
/// two distinct scratch directories, each under its own instance's cache; the old
/// `~/.cache/<repo>-*` is no part of the place.
#[test]
fn two_instances_with_one_repository_name_have_two_scratch_directories() {
    let scratch_of = |cache: &str| {
        let resolved = places::resolve(
            &words(&["scratch"]),
            &Placement {
                home: PathBuf::from("/h"),
                tmpdir: PathBuf::from("/t"),
                start: PathBuf::from("/h/example-org/alpha"),
                repository: Some("alpha".to_owned()),
                trees: PathBuf::from("/h/trees"),
                records: PathBuf::from("/h/records"),
                cache: PathBuf::from(cache),
                harness: Harness::Claude,
                controller: true,
            },
        );
        resolved[0].paths.clone()
    };
    let first = scratch_of("/h/.cache/b10x/conductor/first");
    let second = scratch_of("/h/.cache/b10x/conductor/second");
    assert_eq!(
        first,
        ["/t", "/h/.cache/b10x/conductor/first/scratch/alpha"]
    );
    assert_eq!(
        second,
        ["/t", "/h/.cache/b10x/conductor/second/scratch/alpha"]
    );
    assert_ne!(first[1], second[1]);
}
