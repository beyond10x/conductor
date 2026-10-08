//! `story:config-file`: one config file names conductor's instances, and `conductor config
//! validate` and `conductor config show` read it.
//!
//! Each case is a directory of its own in this test target's temporary directory, holding a home
//! directory `home/`, the working directory `work/` and the state directory `state/`. The built
//! binary runs from `work/` with `HOME` naming `home/`, `--state-dir` naming `state/`, and neither
//! environment variable of the specification's settings set unless the case sets it, so no case
//! reads the real home's config. The config files are the fixtures under
//! `tests/fixtures/config/`, or files the case writes under its own directory.
//!
//! The environment variables are read from the `settings:` of the `conductor-cli` component, as
//! `ess specify compile --path spec` resolves them, upper-snake-cased as `ess-runtime/1` derives
//! them; `ess` is taken from `PATH`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

use clap::Parser as _;
use conductor_cli::cli::Cli;
use conductor_cli::config;
use conductor_cli::config::ORGANIZATION;
use serde_json::Value;

/// The file version every config file carries.
const VERSION: &str = "conductor.config/1";

/// One case: `home/`, `work/` and `state/` under its own directory.
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
            .join("config")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("work")).expect("create the case's working directory");
        Self { dir }
    }

    /// The case's home directory, as `HOME` names it to the binary.
    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    /// The case's working directory, canonical, as the binary sees it.
    fn work(&self) -> PathBuf {
        fs::canonicalize(self.dir.join("work")).expect("the case's working directory exists")
    }

    /// Writes `text` to `relative` under the case's directory and answers its path.
    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.dir.join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        fs::write(&path, text).expect("write a case file");
        path
    }

    /// Runs `conductor --state-dir <case>/state <args>` from `work/`, with `HOME` naming the
    /// case's home, both settings' variables removed and then `env` set, and asserts it wrote
    /// nothing: `work/` stays empty and `state/` is not created.
    fn conductor(&self, args: &[&OsStr], env: &[(&str, &OsStr)]) -> Output {
        let (config, instance) = variables();
        let mut command = Command::new(env!("CARGO_BIN_EXE_conductor"));
        command
            .current_dir(self.dir.join("work"))
            .env("HOME", self.dir.join("home"))
            .env_remove(&config)
            .env_remove(&instance)
            .arg("--state-dir")
            .arg(self.dir.join("state"))
            .args(args)
            .stdin(Stdio::null());
        for (name, value) in env {
            command.env(name, value);
        }
        let output = command.output().expect("the conductor binary runs");
        assert_eq!(
            fs::read_dir(self.dir.join("work"))
                .expect("read work/")
                .count(),
            0,
            "`{args:?}` wrote into the working directory: {}",
            describe(&output)
        );
        assert!(
            !self.dir.join("state").exists(),
            "`{args:?}` created the state directory: {}",
            describe(&output)
        );
        output
    }

    /// `conductor <words>`, each word as text.
    fn run(&self, words: &[&str]) -> Output {
        let args: Vec<&OsStr> = words.iter().map(OsStr::new).collect();
        self.conductor(&args, &[])
    }

    /// `conductor --config <file> <words>`.
    fn with_file(&self, file: &Path, words: &[&str]) -> Output {
        let mut args: Vec<&OsStr> = vec![OsStr::new("--config"), file.as_os_str()];
        args.extend(words.iter().map(OsStr::new));
        self.conductor(&args, &[])
    }

    /// `conductor --config <file> config show --format json <extra>`, answered as JSON.
    fn show(&self, file: &Path, extra: &[&str]) -> Value {
        let mut words = vec!["config", "show", "--format", "json"];
        words.extend(extra);
        let output = self.with_file(file, &words);
        assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
        json(&output)
    }
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/config")
        .join(name)
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\n--- stdout\n{}\n--- stderr\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("stdout is JSON ({error}): {}", describe(output)))
}

/// The environment variables of the `conductor-cli` component's settings `conductor-config` and
/// `conductor-instance`: each setting's name upper-snake-cased.
fn variables() -> (String, String) {
    static VARIABLES: OnceLock<(String, String)> = OnceLock::new();
    VARIABLES.get_or_init(read_variables).clone()
}

/// The specification, as `ess specify compile --path spec --format json` resolves it.
fn model() -> &'static Value {
    static MODEL: OnceLock<Value> = OnceLock::new();
    MODEL.get_or_init(|| {
        let spec = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec");
        let output = Command::new("ess")
            .args(["specify", "compile", "--format", "json", "--path"])
            .arg(&spec)
            .output()
            .expect("ess runs; install the version spec/ess-inputs.yaml requires");
        assert!(output.status.success(), "{}", describe(&output));
        serde_json::from_slice(&output.stdout).expect("the compiled model is JSON")
    })
}

fn read_variables() -> (String, String) {
    let settings = model()["components"]["conductor-cli"]["settings"]
        .as_array()
        .expect("the conductor-cli component declares settings");
    let variable = |name: &str| {
        let setting = settings
            .iter()
            .find(|setting| setting["name"] == name)
            .unwrap_or_else(|| panic!("the conductor-cli component declares setting `{name}`"));
        setting["name"]
            .as_str()
            .expect("a setting's name is text")
            .replace('-', "_")
            .to_ascii_uppercase()
    };
    (variable("conductor-config"), variable("conductor-instance"))
}

/// The one instance a `config show` document holds.
fn instance(shown: &Value) -> &Value {
    assert_eq!(shown["version"], VERSION, "{shown:#}");
    let instances = shown["instances"].as_array().expect("instances is a list");
    assert_eq!(instances.len(), 1, "show prints one instance: {shown:#}");
    &instances[0]
}

/// Seconds in a shown duration: a whole number with `s`, `m`, `h` or `d`.
fn seconds(shown: &Value) -> u64 {
    let text = shown.as_str().expect("a duration is text");
    let (number, unit) = text.split_at(text.len() - 1);
    let number: u64 = number
        .parse()
        .expect("a duration is a whole number and a unit");
    number
        * match unit {
            "s" => 1,
            "m" => 60,
            "h" => 3600,
            "d" => 86_400,
            other => panic!("{text}: unit {other} is not s, m, h or d"),
        }
}

/// GiB in a shown size: a whole number with `G`.
fn gib(shown: &Value) -> u64 {
    let text = shown.as_str().expect("a size is text");
    text.strip_suffix('G')
        .and_then(|number| number.parse().ok())
        .unwrap_or_else(|| panic!("{text} is not a whole number with G"))
}

/// Tokens in a shown count: a whole number, or thousands with `k`.
fn tokens(shown: &Value) -> u64 {
    match shown {
        Value::Number(number) => number.as_u64().expect("a whole number"),
        Value::String(text) => match text.strip_suffix('k') {
            Some(thousands) => thousands.parse::<u64>().expect("a whole number with k") * 1000,
            None => text.parse().expect("a whole number"),
        },
        other => panic!("{other} is not a count of tokens"),
    }
}

fn path_text(path: &Path) -> String {
    path.to_str().expect("a UTF-8 path").to_owned()
}

// ---------------------------------------------------------------------------------------------
// Acceptance 1: `config validate` passes on the story's example, and on an empty file is told
// the version is missing.
// ---------------------------------------------------------------------------------------------

#[test]
fn validate_passes_on_the_story_example() {
    let case = Case::new("validate-example");
    let file = fixture("example.yaml");
    let output = case.with_file(&file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert!(
        stderr(&output).is_empty(),
        "a valid file has no problem: {}",
        describe(&output)
    );
}

#[test]
fn validate_on_an_empty_file_is_told_the_version_is_missing() {
    let case = Case::new("validate-empty");
    let output = case.with_file(&fixture("empty.yaml"), &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let problems = stderr(&output);
    assert!(
        problems
            .lines()
            .any(|line| line.contains("version: ") && line.contains("missing")),
        "the problem names the YAML path `version` and that it is missing: {}",
        describe(&output)
    );
}

#[test]
fn the_config_flag_is_taken_before_and_after_the_subcommand() {
    let case = Case::new("validate-flag-after");
    let file = fixture("example.yaml");
    let after = case.conductor(
        &[
            OsStr::new("config"),
            OsStr::new("validate"),
            OsStr::new("--config"),
            file.as_os_str(),
        ],
        &[],
    );
    assert_eq!(after.status.code(), Some(0), "{}", describe(&after));
    let empty = fixture("empty.yaml");
    let refused = case.conductor(
        &[
            OsStr::new("config"),
            OsStr::new("validate"),
            OsStr::new("--config"),
            empty.as_os_str(),
        ],
        &[],
    );
    assert_eq!(refused.status.code(), Some(1), "{}", describe(&refused));
}

/// Decision 2: `--config` is parsed on every command, before the group and after the leaf, and
/// nothing but the `config` leaves reads it yet.
#[test]
fn the_config_flag_parses_on_every_command() {
    for argv in [
        &["conductor", "--config", "some.yaml", "goal", "goals"][..],
        &["conductor", "goal", "goals", "--config", "some.yaml"][..],
        &[
            "conductor",
            "snapshot",
            "snapshots",
            "--config",
            "some.yaml",
        ][..],
        &["conductor", "--config", "some.yaml", "watch", "run"][..],
        &["conductor", "--config", "some.yaml", "config", "show"][..],
    ] {
        if let Err(error) = Cli::try_parse_from(argv) {
            panic!("{argv:?} does not parse: {error}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Acceptance 2: `config show --format json` with no file prints the defaults, each equal to
// today's constant.
// ---------------------------------------------------------------------------------------------

#[test]
fn show_without_a_file_prints_todays_constants() {
    let case = Case::new("show-defaults");
    let output = case.run(&["config", "show", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let shown = json(&output);
    let instance = instance(&shown);
    let home = case.home();
    let work = case.work();

    // src/config.rs, `ORGANIZATION`: the organization and its GitHub owner.
    assert_eq!(instance["name"], ORGANIZATION, "{shown:#}");
    assert_eq!(
        instance["sources"],
        serde_json::json!([{"github": ORGANIZATION}]),
        "{shown:#}"
    );
    // src/guard/rules.rs:63, `CHECKOUTS`, and src/guard/rules.rs:66, `WORKTREES`, both under
    // the home directory.
    assert_eq!(
        instance["checkouts"]["root"],
        path_text(&home.join("example-org")),
        "{shown:#}"
    );
    assert_eq!(
        instance["checkouts"]["trees"],
        path_text(&home.join(".local/state/worktree/trees/example-org")),
        "{shown:#}"
    );
    // Coordinator decision 1: the records are the working directory; src/state.rs:21,
    // `DEFAULT`: the state is `state/` under it.
    assert_eq!(instance["records"], path_text(&work), "{shown:#}");
    assert_eq!(
        instance["state"],
        path_text(&work.join("state")),
        "{shown:#}"
    );
    // src/dashboard.rs:135: the watch cache the dashboard reads.
    assert_eq!(
        instance["cache"],
        path_text(&home.join(".cache/conductor-watch")),
        "{shown:#}"
    );
    // Taskfile.yml:20 (conductor), Taskfile.yml:48 (conductor-dev) and
    // .agents/conductor.md:177 (a controller): `claude … --model opus`; no role names an agent or
    // a settings file or a profile.
    let role = |name: &str| {
        serde_json::json!({
            "role": name, "harness": "claude", "model": "opus", "agent": null, "settings": null,
            "profile": null
        })
    };
    assert_eq!(
        instance["roles"],
        serde_json::json!([role("conductor"), role("conductor-dev"), role("controller")]),
        "{shown:#}"
    );
    // No repository rule: rules are written only by a user, and today a repository's activity is
    // its mark or the newest snapshot's (src/repository.rs), which no rule overrides
    // (coordinator ruling on wave 08 U2).
    assert_eq!(instance["repositories"], serde_json::json!([]), "{shown:#}");
    // .agents/conductor.md:17, `17 */2 * * *`: a cycle every 2 hours; .agents/conductor.md:19,
    // `7 8 * * *`: the daily cycle at 08:07.
    let cadence = &instance["cadence"];
    assert_eq!(seconds(&cadence["cycle"]), 2 * 3600, "{shown:#}");
    assert_eq!(cadence["daily"], "08:07", "{shown:#}");
    // src/config.rs, `WATCH_EVERY` and `CI_EVERY`: the watch's intervals.
    assert_eq!(seconds(&cadence["watch"]), config::WATCH_EVERY, "{shown:#}");
    assert_eq!(seconds(&cadence["ci"]), config::CI_EVERY, "{shown:#}");
    let thresholds = &instance["thresholds"];
    // src/config.rs, `CONTEXT_HANDOVER`, `DISK_LOW_GIB` and `DISK_CLEAR_GIB`; the disk at 100G
    // and 110G since wave 08 W4, which keeps 100G free.
    assert_eq!(
        tokens(&thresholds["context_handover"]),
        config::CONTEXT_HANDOVER,
        "{shown:#}"
    );
    assert_eq!(
        gib(&thresholds["disk_low"]),
        config::DISK_LOW_GIB,
        "{shown:#}"
    );
    assert_eq!(
        gib(&thresholds["disk_clear"]),
        config::DISK_CLEAR_GIB,
        "{shown:#}"
    );
    // docs/design/conductor.md:636: no session starts under 20G free.
    assert_eq!(gib(&thresholds["disk_admit"]), 20, "{shown:#}");
    // .agents/conductor.md, Dispatch: a build slot under 60G free, the line conductor grants
    // slots by.
    assert_eq!(gib(&thresholds["build_slot"]), 60, "{shown:#}");
    // The disk thresholds read `/`; a build expected to write more than 10G asks for a slot; the
    // full-gate watchdog stops the gate under 15G, resumes it from 17G and reads every 10s.
    assert_eq!(thresholds["disk_path"], "/", "{shown:#}");
    assert_eq!(gib(&thresholds["build_size"]), 10, "{shown:#}");
    assert_eq!(gib(&thresholds["gate_stop"]), 15, "{shown:#}");
    assert_eq!(gib(&thresholds["gate_resume"]), 17, "{shown:#}");
    assert_eq!(seconds(&thresholds["watchdog_every"]), 10, "{shown:#}");
    // No report channel is configured anywhere today.
    assert_eq!(instance["reports"], serde_json::json!([]), "{shown:#}");
    // spec/domains/decision.yaml:4-6: conductor answers class C, the operator class O.
    assert_eq!(
        instance["authority"],
        serde_json::json!({"class_c": "conductor", "class_o": "operator"}),
        "{shown:#}"
    );
    // The coordinator's addition to the brief: by default at most 5 controllers work at once. No constant caps them today
    // (docs/design/conductor.md:636, "no cap on the number of controllers"). One controller runs
    // at most 4 sub-agents at once.
    assert_eq!(
        instance["controllers"],
        serde_json::json!({"max_working": 5, "max_subagents": 4}),
        "{shown:#}"
    );
    // No instance names its operator unless the file does.
    assert_eq!(instance["operator"], Value::Null, "{shown:#}");
    // story:observation-retention: the newest 12 complete snapshots keep their observations, a
    // day at the 2-hour cadence.
    assert_eq!(
        instance["retention"],
        serde_json::json!({"snapshots": 12}),
        "{shown:#}"
    );
    let keys: Vec<&str> = instance
        .as_object()
        .expect("an instance is a mapping")
        .keys()
        .map(String::as_str)
        .collect();
    // story:catalog-source: the built-in instance names no catalog.
    assert_eq!(instance["catalog"], Value::Null, "{shown:#}");
    let mut expected = vec![
        "authority",
        "cache",
        "cadence",
        "catalog",
        "checkouts",
        "controllers",
        "name",
        "operator",
        "records",
        "repositories",
        "reports",
        "retention",
        "roles",
        "sources",
        "state",
        "thresholds",
    ];
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    expected.sort_unstable();
    assert_eq!(sorted, expected, "every key of an instance is shown");
}

/// The coordinator's addition: `controllers.max_working`, the most controllers working at once,
/// is 5 when absent (the key or the mapping), an integer of 1 or more when written, and 0 is
/// refused by its YAML path.
#[test]
fn the_most_controllers_working_is_five_when_absent_and_zero_is_refused() {
    let case = Case::new("controllers");
    let written = |controllers: &str| {
        format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: one\n\
             \x20   sources: [{{github: one}}]\n\
             \x20   checkouts: {{root: ~/one, trees: ~/trees}}\n\
             {controllers}"
        )
    };
    for (name, text, expected) in [
        ("absent", written(""), 5),
        ("empty", written("    controllers: {}\n"), 5),
        ("three", written("    controllers: {max_working: 3}\n"), 3),
    ] {
        let file = case.write(&format!("{name}.yaml"), &text);
        let shown = case.show(&file, &[]);
        assert_eq!(
            instance(&shown)["controllers"]["max_working"],
            expected,
            "{name}: {shown:#}"
        );
    }
    let example = case.show(&fixture("example.yaml"), &[]);
    assert_eq!(instance(&example)["controllers"]["max_working"], 5);

    for (name, value) in [
        ("zero", "0"),
        ("negative", "-2"),
        ("fraction", "2.5"),
        ("text", "five"),
    ] {
        let file = case.write(
            &format!("{name}.yaml"),
            &written(&format!("    controllers: {{max_working: {value}}}\n")),
        );
        let problems = refused(&case, &file);
        assert!(
            problems.contains("instances[0].controllers.max_working: "),
            "{name}: {problems}"
        );
    }
}

/// A config file of one instance `one`, with `extra` lines appended to it.
fn one_instance(extra: &str) -> String {
    format!(
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   sources: [{{github: one}}]\n\
         \x20   checkouts: {{root: ~/one, trees: ~/trees}}\n\
         {extra}"
    )
}

/// The keys the generic session profiles read: a role's `agent` and `settings`, the instance's
/// `operator`, `controllers.max_subagents`, and the thresholds `disk_path`, `build_size`,
/// `gate_stop`, `gate_resume` and `watchdog_every`. Written, each validates and is shown as
/// written, a path made absolute.
#[test]
fn the_session_profile_keys_validate_and_show_as_written() {
    let case = Case::new("session-profile-keys");
    let file = case.write(
        "profile.yaml",
        &one_instance(
            "\x20   roles:\n\
             \x20     - {role: controller, harness: claude, model: opus, agent: repo-controller, \
             settings: ~/profiles/controller.json}\n\
             \x20     - {role: conductor, harness: codex, model: gpt-fixture, settings: \
             /srv/conductor.json}\n\
             \x20   controllers: {max_working: 3, max_subagents: 2}\n\
             \x20   thresholds: {disk_path: /srv/data, build_slot: 45G, build_size: 12G, \
             gate_stop: 20G, gate_resume: 25G, watchdog_every: 30s}\n\
             \x20   operator: the decider\n",
        ),
    );
    let validated = case.with_file(&file, &["config", "validate"]);
    assert_eq!(validated.status.code(), Some(0), "{}", describe(&validated));
    assert!(stderr(&validated).is_empty(), "{}", describe(&validated));

    let shown = case.show(&file, &[]);
    let one = instance(&shown);
    let home = case.home();
    assert_eq!(
        one["roles"],
        serde_json::json!([
            {
                "role": "controller", "harness": "claude", "model": "opus",
                "agent": "repo-controller",
                "settings": path_text(&home.join("profiles/controller.json")),
                "profile": null,
            },
            {
                "role": "conductor", "harness": "codex", "model": "gpt-fixture",
                "agent": null, "settings": "/srv/conductor.json", "profile": null,
            },
        ]),
        "{shown:#}"
    );
    assert_eq!(
        one["controllers"],
        serde_json::json!({"max_working": 3, "max_subagents": 2}),
        "{shown:#}"
    );
    let thresholds = &one["thresholds"];
    assert_eq!(thresholds["disk_path"], "/srv/data", "{shown:#}");
    assert_eq!(gib(&thresholds["build_slot"]), 45, "{shown:#}");
    assert_eq!(gib(&thresholds["build_size"]), 12, "{shown:#}");
    assert_eq!(gib(&thresholds["gate_stop"]), 20, "{shown:#}");
    assert_eq!(gib(&thresholds["gate_resume"]), 25, "{shown:#}");
    assert_eq!(seconds(&thresholds["watchdog_every"]), 30, "{shown:#}");
    assert_eq!(one["operator"], "the decider", "{shown:#}");

    let text = case.with_file(&file, &["config", "show"]);
    assert_eq!(text.status.code(), Some(0), "{}", describe(&text));
    let text = String::from_utf8_lossy(&text.stdout);
    for line in [
        "instances[0].roles[0].agent: repo-controller".to_owned(),
        format!(
            "instances[0].roles[0].settings: {}",
            path_text(&home.join("profiles/controller.json"))
        ),
        "instances[0].controllers.max_subagents: 2".to_owned(),
        "instances[0].thresholds.disk_path: /srv/data".to_owned(),
        "instances[0].thresholds.build_size: 12G".to_owned(),
        "instances[0].thresholds.gate_stop: 20G".to_owned(),
        "instances[0].thresholds.gate_resume: 25G".to_owned(),
        "instances[0].thresholds.watchdog_every: 30s".to_owned(),
        "instances[0].operator: the decider".to_owned(),
    ] {
        assert!(
            text.lines().any(|shown| shown == line),
            "no line `{line}`: {text}"
        );
    }
}

/// Without them, a file shows the defaults: no agent, settings file or operator, 4 sub-agents,
/// the disk read on `/`, a build slot under 60G for a build over 10G, the gate stopped under 15G
/// and resumed from 17G, read every 10s. An empty `controllers` or `thresholds` is the same.
#[test]
fn without_the_session_profile_keys_a_file_shows_their_defaults() {
    let case = Case::new("session-profile-defaults");
    for (name, extra) in [
        (
            "absent",
            "\x20   roles: [{role: controller, harness: claude, model: opus}]\n",
        ),
        (
            "empty",
            "\x20   roles: [{role: controller, harness: claude, model: opus}]\n\
             \x20   controllers: {}\n\
             \x20   thresholds: {}\n",
        ),
    ] {
        let file = case.write(&format!("{name}.yaml"), &one_instance(extra));
        let shown = case.show(&file, &[]);
        let one = instance(&shown);
        assert_eq!(
            one["roles"],
            serde_json::json!([{
                "role": "controller", "harness": "claude", "model": "opus",
                "agent": null, "settings": null, "profile": null,
            }]),
            "{name}: {shown:#}"
        );
        assert_eq!(one["controllers"]["max_subagents"], 4, "{name}: {shown:#}");
        let thresholds = &one["thresholds"];
        assert_eq!(thresholds["disk_path"], "/", "{name}: {shown:#}");
        assert_eq!(gib(&thresholds["build_slot"]), 60, "{name}: {shown:#}");
        assert_eq!(gib(&thresholds["build_size"]), 10, "{name}: {shown:#}");
        assert_eq!(gib(&thresholds["gate_stop"]), 15, "{name}: {shown:#}");
        assert_eq!(gib(&thresholds["gate_resume"]), 17, "{name}: {shown:#}");
        assert_eq!(
            seconds(&thresholds["watchdog_every"]),
            10,
            "{name}: {shown:#}"
        );
        assert_eq!(one["operator"], Value::Null, "{name}: {shown:#}");
    }
}

/// `max_subagents` is a whole number of 1 or more (`Controllers`'s invariant), and `gate_resume`
/// lies above `gate_stop` (`Thresholds`'s), each refused by its YAML path; so is a `disk_path` or
/// a role's `settings` that is not a path.
#[test]
fn the_session_profile_keys_are_refused_by_their_path() {
    let case = Case::new("session-profile-refused");
    for (name, extra, path) in [
        (
            "subagents-zero",
            "\x20   controllers: {max_subagents: 0}\n",
            "instances[0].controllers.max_subagents: ",
        ),
        (
            "subagents-negative",
            "\x20   controllers: {max_subagents: -1}\n",
            "instances[0].controllers.max_subagents: ",
        ),
        (
            "subagents-text",
            "\x20   controllers: {max_subagents: four}\n",
            "instances[0].controllers.max_subagents: ",
        ),
        (
            "resume-equal",
            "\x20   thresholds: {gate_stop: 20G, gate_resume: 20G}\n",
            "instances[0].thresholds.gate_resume: ",
        ),
        (
            "resume-below",
            "\x20   thresholds: {gate_stop: 20G, gate_resume: 18G}\n",
            "instances[0].thresholds.gate_resume: ",
        ),
        (
            "stop-above-default-resume",
            "\x20   thresholds: {gate_stop: 17G}\n",
            "instances[0].thresholds.gate_resume: ",
        ),
        (
            "gate-stop-not-a-size",
            "\x20   thresholds: {gate_stop: 15}\n",
            "instances[0].thresholds.gate_stop: ",
        ),
        (
            "watchdog-not-a-duration",
            "\x20   thresholds: {watchdog_every: soon}\n",
            "instances[0].thresholds.watchdog_every: ",
        ),
        (
            "disk-path-relative",
            "\x20   thresholds: {disk_path: data}\n",
            "instances[0].thresholds.disk_path: ",
        ),
        (
            "settings-relative",
            "\x20   roles: [{role: controller, harness: claude, model: opus, settings: x.json}]\n",
            "instances[0].roles[0].settings: ",
        ),
        (
            "operator-empty",
            "\x20   operator: \"\"\n",
            "instances[0].operator: ",
        ),
    ] {
        let file = case.write(&format!("{name}.yaml"), &one_instance(extra));
        let problems = refused(&case, &file);
        assert!(problems.contains(path), "{name}: {problems}");
    }
}

/// The characters a role's `model`, `agent` and `settings` may hold: a start command passes each
/// to the harness as one shell word, unquoted.
const COMMAND_WORD: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._:/-";

/// A role's `model`, `agent` and `settings` that hold a character outside [`COMMAND_WORD`] are
/// refused by their path, the refusal naming the field; the harness is one of a closed set. The
/// specification declares the same: each of those fields is a `conductor.config.CommandWord`, a
/// `String` newtype whose alphabet is [`COMMAND_WORD`].
#[test]
fn a_role_s_command_words_hold_only_shell_safe_characters() {
    let case = Case::new("role-command-words");
    for (name, role, path) in [
        (
            "model-quote",
            "{role: conductor, harness: claude, model: \"opus' ; touch /x\"}",
            "instances[0].roles[0].model: ",
        ),
        (
            "model-space",
            "{role: conductor, harness: claude, model: \"opus 4\"}",
            "instances[0].roles[0].model: ",
        ),
        (
            "model-substitution",
            "{role: conductor, harness: claude, model: \"$(id)\"}",
            "instances[0].roles[0].model: ",
        ),
        (
            "agent-semicolon",
            "{role: controller, harness: claude, model: opus, agent: \"x; rm -rf ~\"}",
            "instances[0].roles[0].agent: ",
        ),
        (
            "agent-backtick",
            "{role: controller, harness: claude, model: opus, agent: \"a`id`\"}",
            "instances[0].roles[0].agent: ",
        ),
        (
            "settings-space",
            "{role: controller, harness: claude, model: opus, settings: \"~/a b.json\"}",
            "instances[0].roles[0].settings: ",
        ),
        (
            "settings-dollar",
            "{role: controller, harness: claude, model: opus, settings: \"/srv/$HOME.json\"}",
            "instances[0].roles[0].settings: ",
        ),
        (
            "harness-other",
            "{role: controller, harness: \"claude;id\", model: opus}",
            "instances[0].roles[0].harness: ",
        ),
    ] {
        let file = case.write(
            &format!("{name}.yaml"),
            &one_instance(&format!("\x20   roles: [{role}]\n")),
        );
        let problems = refused(&case, &file);
        let line = problems
            .lines()
            .find(|line| line.contains(path))
            .unwrap_or_else(|| panic!("{name}: no problem names `{path}`: {problems}"));
        if path.ends_with("model: ") || path.ends_with("agent: ") || path.ends_with("settings: ") {
            assert!(
                line.contains("A-Z a-z 0-9 . _ : / -"),
                "{name}: the refusal names the characters allowed: {line}"
            );
        }
    }
    let file = case.write(
        "allowed.yaml",
        &one_instance(
            "\x20   roles: [{role: controller, harness: claude, model: claude-opus-4.1:beta/x_y, \
             agent: plugin:repo-controller, settings: ~/p/controller-settings.json}]\n",
        ),
    );
    let validated = case.with_file(&file, &["config", "validate"]);
    assert_eq!(validated.status.code(), Some(0), "{}", describe(&validated));

    let types = &model()["types"];
    let word = &types["conductor.config.CommandWord"]["body"];
    assert_eq!(word["kind"], "newtype", "{word:#}");
    assert_eq!(word["alphabet"], COMMAND_WORD, "{word:#}");
    let fields = types["conductor.config.Role"]["body"]["fields"]
        .as_array()
        .expect("Role's fields");
    for name in ["model", "agent", "settings"] {
        let field = fields
            .iter()
            .find(|field| field["name"] == name)
            .unwrap_or_else(|| panic!("Role declares {name}"));
        let shown = field["type_ref"].to_string();
        assert!(
            shown.contains("\"conductor.config.CommandWord\""),
            "Role.{name} is a CommandWord: {shown}"
        );
    }
}

/// A role's `profile` is the profile file its session starts with through
/// `--append-system-prompt-file`: a path, absolute or under `~`, that a start command passes as
/// one shell word. Written, it validates and is shown absolute; absent, it is shown as null; a
/// relative path or one outside [`COMMAND_WORD`] is refused by its YAML path. The specification
/// declares it as an optional `conductor.config.CommandWord`.
#[test]
fn a_role_s_profile_is_a_path_shown_absolute_and_refused_when_relative() {
    let case = Case::new("role-profile");
    let file = case.write(
        "profile.yaml",
        &one_instance(
            "\x20   roles:\n\
             \x20     - {role: controller, harness: claude, model: opus, \
             profile: ~/profiles/repo-controller.md}\n\
             \x20     - {role: conductor, harness: claude, model: opus, \
             profile: /srv/conductor.md}\n\
             \x20     - {role: conductor-dev, harness: claude, model: opus}\n",
        ),
    );
    let validated = case.with_file(&file, &["config", "validate"]);
    assert_eq!(validated.status.code(), Some(0), "{}", describe(&validated));

    let shown = case.show(&file, &[]);
    let roles = &instance(&shown)["roles"];
    let home = case.home();
    let controller = path_text(&home.join("profiles/repo-controller.md"));
    assert_eq!(roles[0]["profile"], controller.as_str(), "{shown:#}");
    assert_eq!(roles[1]["profile"], "/srv/conductor.md", "{shown:#}");
    assert_eq!(roles[2]["profile"], Value::Null, "{shown:#}");

    let text = case.with_file(&file, &["config", "show"]);
    assert_eq!(text.status.code(), Some(0), "{}", describe(&text));
    let text = String::from_utf8_lossy(&text.stdout);
    let line = format!("instances[0].roles[0].profile: {controller}");
    assert!(
        text.lines().any(|shown| shown == line),
        "no line `{line}`: {text}"
    );

    for (name, profile) in [
        ("profile-relative", "profile.md"),
        ("profile-dot-relative", "./profile.md"),
        ("profile-space", "\"~/a b.md\""),
        ("profile-dollar", "\"/srv/$HOME.md\""),
        ("profile-empty", "\"\""),
    ] {
        let file = case.write(
            &format!("{name}.yaml"),
            &one_instance(&format!(
                "\x20   roles: [{{role: controller, harness: claude, model: opus, \
                 profile: {profile}}}]\n"
            )),
        );
        let problems = refused(&case, &file);
        assert!(
            problems.contains("instances[0].roles[0].profile: "),
            "{name}: {problems}"
        );
    }

    let fields = model()["types"]["conductor.config.Role"]["body"]["fields"]
        .as_array()
        .expect("Role's fields");
    let field = fields
        .iter()
        .find(|field| field["name"] == "profile")
        .expect("Role declares profile");
    assert_eq!(
        field["type_ref"],
        serde_json::json!({
            "kind": "optional",
            "of": {"kind": "declared", "name": "conductor.config.CommandWord"},
        }),
        "Role.profile is an optional CommandWord: {field:#}"
    );
}

#[test]
fn show_without_a_file_answers_the_built_in_instance_by_name_only() {
    let case = Case::new("show-defaults-instance");
    let named = case.run(&[
        "config",
        "show",
        "--format",
        "json",
        "--instance",
        ORGANIZATION,
    ]);
    assert_eq!(named.status.code(), Some(0), "{}", describe(&named));
    assert_eq!(instance(&json(&named))["name"], ORGANIZATION);
    let other = case.run(&["config", "show", "--instance", "other-org"]);
    assert_eq!(other.status.code(), Some(1), "{}", describe(&other));
    assert!(
        stderr(&other).contains("other-org") && stderr(&other).contains(ORGANIZATION),
        "the refusal names the instance asked for and the one there is: {}",
        describe(&other)
    );
}

#[test]
fn validate_without_a_file_says_the_defaults_apply() {
    let case = Case::new("validate-no-file");
    let output = case.run(&["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        said.contains(".b10x/conductor/conductor.yaml") && said.contains("default"),
        "validate names the file it looked for and that the defaults apply: {}",
        describe(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// Acceptance 3: two instances and no `--instance` is an error naming both; `--instance` picks
// one.
// ---------------------------------------------------------------------------------------------

#[test]
fn two_instances_and_no_instance_is_an_error_naming_both() {
    let case = Case::new("two-instances");
    let output = case.with_file(&fixture("two-instances.yaml"), &["config", "show"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = stderr(&output);
    assert!(
        said.contains("first-org") && said.contains("second-org"),
        "the error names both instances: {}",
        describe(&output)
    );
    assert!(output.stdout.is_empty(), "{}", describe(&output));
    let validated = case.with_file(&fixture("two-instances.yaml"), &["config", "validate"]);
    assert_eq!(
        validated.status.code(),
        Some(0),
        "two instances are a valid file: {}",
        describe(&validated)
    );
}

#[test]
fn the_instance_flag_picks_one_and_fills_its_defaults() {
    let case = Case::new("instance-flag");
    let home = case.home();
    let shown = case.show(
        &fixture("two-instances.yaml"),
        &["--instance", "second-org"],
    );
    let second = instance(&shown);
    assert_eq!(second["name"], "second-org", "{shown:#}");
    assert_eq!(
        second["sources"],
        serde_json::json!([{"local": path_text(&home.join("src/second"))}]),
        "{shown:#}"
    );
    assert_eq!(
        second["checkouts"],
        serde_json::json!({"root": "/srv/second", "trees": "/srv/second-trees"}),
        "{shown:#}"
    );
    assert_eq!(second["records"], "/srv/second-records", "{shown:#}");
    assert_eq!(
        second["state"],
        path_text(&home.join(".b10x/conductor/second-org/state")),
        "{shown:#}"
    );
    assert_eq!(
        second["cache"],
        path_text(&home.join(".cache/b10x/conductor/second-org")),
        "{shown:#}"
    );
    assert_eq!(
        second["roles"],
        serde_json::json!([{
            "role": "conductor", "harness": "codex", "model": "gpt-fixture",
            "agent": null, "settings": null, "profile": null,
        }]),
        "{shown:#}"
    );
    assert_eq!(second["repositories"], serde_json::json!([]), "{shown:#}");
    assert_eq!(seconds(&second["cadence"]["cycle"]), 86_400, "{shown:#}");
    assert_eq!(seconds(&second["cadence"]["watch"]), 60, "{shown:#}");
    assert_eq!(second["cadence"]["daily"], "08:07", "{shown:#}");
    assert_eq!(
        seconds(&second["cadence"]["ci"]),
        config::CI_EVERY,
        "{shown:#}"
    );
    assert_eq!(
        tokens(&second["thresholds"]["context_handover"]),
        150_000,
        "{shown:#}"
    );
    assert_eq!(gib(&second["thresholds"]["disk_low"]), 5, "{shown:#}");
    assert_eq!(
        gib(&second["thresholds"]["disk_clear"]),
        config::DISK_CLEAR_GIB,
        "{shown:#}"
    );
    assert_eq!(
        second["authority"],
        serde_json::json!({"class_c": "operator", "class_o": "operator"}),
        "{shown:#}"
    );

    let first = case.show(&fixture("two-instances.yaml"), &["--instance", "first-org"]);
    assert_eq!(instance(&first)["name"], "first-org", "{first:#}");
    assert_eq!(
        instance(&first)["checkouts"]["root"],
        path_text(&home.join("first-org")),
        "{first:#}"
    );
}

#[test]
fn an_instance_the_file_does_not_name_is_an_error_naming_the_instances() {
    let case = Case::new("instance-unknown");
    let output = case.with_file(
        &fixture("two-instances.yaml"),
        &["config", "show", "--instance", "third-org"],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = stderr(&output);
    assert!(
        said.contains("third-org") && said.contains("first-org") && said.contains("second-org"),
        "{}",
        describe(&output)
    );
}

#[test]
fn the_environment_names_the_file_and_the_instance() {
    let case = Case::new("environment");
    let (config, instance_variable) = variables();
    let file = fixture("two-instances.yaml");
    let output = case.conductor(
        &[
            OsStr::new("config"),
            OsStr::new("show"),
            OsStr::new("--format"),
            OsStr::new("json"),
        ],
        &[
            (config.as_str(), file.as_os_str()),
            (instance_variable.as_str(), OsStr::new("second-org")),
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(instance(&json(&output))["name"], "second-org");

    // The flags win over the environment.
    let empty = fixture("empty.yaml");
    let flags = case.conductor(
        &[
            OsStr::new("--config"),
            file.as_os_str(),
            OsStr::new("config"),
            OsStr::new("show"),
            OsStr::new("--format"),
            OsStr::new("json"),
            OsStr::new("--instance"),
            OsStr::new("first-org"),
        ],
        &[
            (config.as_str(), empty.as_os_str()),
            (instance_variable.as_str(), OsStr::new("second-org")),
        ],
    );
    assert_eq!(flags.status.code(), Some(0), "{}", describe(&flags));
    assert_eq!(instance(&json(&flags))["name"], "first-org");
}

#[test]
fn the_default_file_is_read_from_the_home_directory() {
    let case = Case::new("default-file");
    let text = fs::read_to_string(fixture("example.yaml")).expect("read the example");
    case.write("home/.b10x/conductor/conductor.yaml", &text);
    let output = case.run(&["config", "show", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let shown = json(&output);
    let home = case.home();
    assert_eq!(
        instance(&shown)["records"],
        path_text(&home.join(".b10x/conductor/example-org/records")),
        "the example's records, `~` expanded: {shown:#}"
    );
    assert_eq!(
        instance(&shown)["repositories"],
        serde_json::json!([{"match": "ess", "activity": "active"}]),
        "{shown:#}"
    );
}

#[test]
fn a_named_file_that_is_not_there_is_an_error() {
    let case = Case::new("named-missing");
    let missing = case.dir.join("nowhere.yaml");
    let output = case.with_file(&missing, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        stderr(&output).contains("nowhere.yaml"),
        "{}",
        describe(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// Acceptance 4: an unknown key, a duplicate name and a bad duration are each refused with their
// YAML path.
// ---------------------------------------------------------------------------------------------

/// Validates `file` and answers its problem lines, asserting exit 1.
fn refused(case: &Case, file: &Path) -> String {
    let output = case.with_file(file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let shown = case.with_file(file, &["config", "show", "--format", "json"]);
    assert_eq!(
        shown.status.code(),
        Some(1),
        "show refuses an invalid file too: {}",
        describe(&shown)
    );
    assert!(shown.stdout.is_empty(), "{}", describe(&shown));
    stderr(&output)
}

#[test]
fn an_unknown_key_is_refused_with_its_yaml_path() {
    let case = Case::new("unknown-key");
    let problems = refused(&case, &fixture("unknown-key.yaml"));
    assert!(
        problems.contains("instances[0].cadence.cycel: "),
        "{problems}"
    );
}

/// Every mapping of a shown config, with its YAML path, depth first; the root's path is empty.
fn mappings<'v>(value: &'v Value, path: &str, out: &mut Vec<(String, &'v Value)>) {
    match value {
        Value::Object(map) => {
            out.push((path.to_owned(), value));
            for (key, item) in map {
                let at = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                mappings(item, &at, out);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                mappings(item, &format!("{path}[{index}]"), out);
            }
        }
        _ => {}
    }
}

/// The value at a YAML path `mappings` answers, to be changed.
fn at_path<'v>(document: &'v mut Value, path: &str) -> &'v mut Value {
    let mut here = document;
    for part in path.split('.').filter(|part| !part.is_empty()) {
        let (key, indices) = part
            .split_once('[')
            .map_or((part, ""), |(key, rest)| (key, rest));
        here = &mut here[key];
        for index in indices.split('[') {
            if let Some(index) = index.strip_suffix(']') {
                here = &mut here[index.parse::<usize>().expect("an index")];
            }
        }
    }
    here
}

/// The rule holds for every mapping the file has, not only the one a fixture names: the story's
/// example, as `config show` writes it back, takes a key `bogus` in each of its mappings in
/// turn, and each time that key alone is refused, by its path.
#[test]
fn an_unknown_key_is_refused_in_every_mapping() {
    let case = Case::new("unknown-key-everywhere");
    let shown = case.show(&fixture("example.yaml"), &[]);
    let mut found = Vec::new();
    mappings(&shown, "", &mut found);
    let paths: Vec<String> = found.into_iter().map(|(path, _)| path).collect();
    assert!(paths.len() >= 10, "the example has its mappings: {paths:?}");
    for path in &paths {
        let mut document = shown.clone();
        at_path(&mut document, path)["bogus"] = Value::from(1);
        let file = case.write("bogus.yaml", &document.to_string());
        let output = case.with_file(&file, &["config", "validate"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{path}: {}",
            describe(&output)
        );
        let expected = if path.is_empty() {
            "bogus: ".to_owned()
        } else {
            format!("{path}.bogus: ")
        };
        let problems: Vec<String> = stderr(&output).lines().map(str::to_owned).collect();
        assert_eq!(problems.len(), 1, "{path}: {problems:#?}");
        assert!(
            problems[0].contains(&format!(": {expected}")),
            "{path}: {problems:#?}"
        );
    }
}

/// Each mapping `config show` writes holds exactly the fields its type declares in the
/// specification, following `conductor.config.Config` down through its fields; a union is
/// written as one key, its variant in lower case.
#[test]
fn every_mapping_holds_the_fields_the_specification_declares() {
    fn check(shown: &Value, type_ref: &Value, path: &str, problems: &mut Vec<String>) {
        match type_ref["kind"].as_str() {
            Some("list") => {
                for (index, item) in shown.as_array().into_iter().flatten().enumerate() {
                    check(item, &type_ref["of"], &format!("{path}[{index}]"), problems);
                }
            }
            Some("declared") => {
                let name = type_ref["name"].as_str().expect("a declared name");
                let body = &model()["types"][name]["body"];
                match body["kind"].as_str() {
                    Some("struct") => {
                        let fields = body["fields"].as_array().expect("a struct's fields");
                        let declared: Vec<&str> = fields
                            .iter()
                            .map(|field| field["name"].as_str().expect("a field's name"))
                            .collect();
                        let Some(map) = shown.as_object() else {
                            problems.push(format!("{path}: {name} is not shown as a mapping"));
                            return;
                        };
                        let mut written: Vec<&str> = map.keys().map(String::as_str).collect();
                        let mut expected = declared.clone();
                        written.sort_unstable();
                        expected.sort_unstable();
                        if written != expected {
                            problems.push(format!(
                                "{path}: {name} declares {expected:?}, shown {written:?}"
                            ));
                        }
                        for field in fields {
                            let key = field["name"].as_str().expect("a field's name");
                            let at = if path.is_empty() {
                                key.to_owned()
                            } else {
                                format!("{path}.{key}")
                            };
                            check(&shown[key], &field["type_ref"], &at, problems);
                        }
                    }
                    Some("union") => {
                        let variants: Vec<String> = body["variants"]
                            .as_object()
                            .expect("a union's variants")
                            .keys()
                            .map(|variant| variant.to_ascii_lowercase())
                            .collect();
                        let keys: Vec<&String> = shown
                            .as_object()
                            .into_iter()
                            .flat_map(|map| map.keys())
                            .collect();
                        if keys.len() != 1 || !variants.contains(keys[0]) {
                            problems.push(format!(
                                "{path}: {name} is one key of {variants:?}, shown {keys:?}"
                            ));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    let case = Case::new("fields-of-the-specification");
    for shown in [
        case.show(&fixture("example.yaml"), &[]),
        json(&case.run(&["config", "show", "--format", "json"])),
    ] {
        let mut problems = Vec::new();
        let root = serde_json::json!({"kind": "declared", "name": "conductor.config.Config"});
        check(&shown, &root, "", &mut problems);
        assert!(problems.is_empty(), "{problems:#?}");
    }
}

#[test]
fn a_duplicate_name_is_refused_with_its_yaml_path() {
    let case = Case::new("duplicate-name");
    let problems = refused(&case, &fixture("duplicate-name.yaml"));
    let named: Vec<&str> = problems
        .lines()
        .filter(|line| line.contains("instances[2].name: "))
        .collect();
    assert_eq!(named.len(), 1, "{problems}");
    assert!(named[0].contains("first-org"), "{problems}");
    assert!(
        !problems.contains("instances[1].name: "),
        "a distinct name is not refused: {problems}"
    );
}

/// A duplicate is named whatever else is wrong with either instance.
#[test]
fn a_duplicate_name_is_named_beside_other_problems() {
    let case = Case::new("duplicate-name-and-more");
    let file = case.write(
        "duplicate.yaml",
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: first-org\n\
         \x20   sources: [{github: first-org}]\n\
         \x20   checkouts: {root: ~/first-org, trees: ~/trees}\n\
         \x20   cadence: {watch: soon}\n\
         \x20 - name: first-org\n\
         \x20   checkouts: {root: ~/again}\n",
    );
    let problems = refused(&case, &file);
    for expected in [
        "instances[0].cadence.watch: ",
        "instances[1].sources: missing",
        "instances[1].checkouts.trees: missing",
        "instances[1].name: \"first-org\" is the name of instances[0] too",
    ] {
        assert!(
            problems.contains(expected),
            "no problem `{expected}`: {problems}"
        );
    }
}

#[test]
fn a_bad_duration_is_refused_with_its_yaml_path() {
    let case = Case::new("bad-duration");
    let problems = refused(&case, &fixture("bad-duration.yaml"));
    assert!(
        problems.contains("instances[0].cadence.watch: "),
        "{problems}"
    );
    assert!(
        !problems.contains("instances[0].cadence.cycle"),
        "2h is a duration: {problems}"
    );
}

#[test]
fn every_problem_of_a_file_is_named() {
    let case = Case::new("many-problems");
    let file = case.write(
        "many.yaml",
        "version: conductor.config/2\n\
         extra: 1\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   sources: [{github: one}, {gitea: one}]\n\
         \x20   checkouts: {root: relative/dir}\n\
         \x20   roles: [{role: conductor, harness: gpt, model: x}]\n\
         \x20   cadence: {cycle: 0h, daily: \"25:00\", ci: 15}\n\
         \x20   thresholds: {disk_low: 15, disk_clear: 18GB, context_handover: lots}\n\
         \x20   repositories: [{match: ess, activity: dormant}]\n\
         \x20   authority: {class_c: everyone}\n",
    );
    let problems = refused(&case, &file);
    for path in [
        "version: ",
        "extra: ",
        "instances[0].sources[1].gitea: ",
        "instances[0].checkouts.root: ",
        "instances[0].checkouts.trees: ",
        "instances[0].roles[0].harness: ",
        "instances[0].cadence.cycle: ",
        "instances[0].cadence.daily: ",
        "instances[0].cadence.ci: ",
        "instances[0].thresholds.disk_low: ",
        "instances[0].thresholds.disk_clear: ",
        "instances[0].thresholds.context_handover: ",
        "instances[0].repositories[0].activity: ",
        "instances[0].authority.class_c: ",
    ] {
        assert!(
            problems.contains(path),
            "no problem names `{path}`: {problems}"
        );
    }
}

/// Coordinator decision 4: no field takes a secret, and a value that looks like a token is
/// refused by its path, its value never printed: not by the refusal, and not by any other
/// problem that quotes a value (a duplicate name, a value of the wrong kind, an unknown key).
/// The token-like values are assembled here so that no committed file holds one.
#[test]
fn a_value_that_looks_like_a_token_is_refused_by_path_not_value() {
    let case = Case::new("token");
    let tokens = [
        ["gh", "p_fixture0123456789"].concat(),
        ["github", "_pat_fixture0123"].concat(),
        ["gl", "pat-fixture0123"].concat(),
        ["xo", "xb-0000-fixture"].concat(),
        ["xo", "xp-1111-fixture"].concat(),
        ["gl", "pat-fixturekey"].concat(),
    ];
    let text = format!(
        "version: conductor.config/1\n\
         {}: 1\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   sources: [{{github: {}}}]\n\
         \x20   checkouts: {{root: ~/one, trees: ~/trees}}\n\
         \x20   roles: [{{role: conductor, harness: claude, model: \"{}\"}}]\n\
         \x20   reports: [{{channel: \"slack:{}\", as: bot, when: [daily]}}]\n\
         \x20   records: \"~/{}\"\n\
         \x20   cadence: {{watch: {}}}\n\
         \x20 - name: {}\n\
         \x20   sources: [{{github: two}}]\n\
         \x20   checkouts: {{root: ~/two, trees: ~/trees}}\n\
         \x20 - name: {}\n\
         \x20   sources: [{{github: three}}]\n\
         \x20   checkouts: {{root: ~/three, trees: ~/trees}}\n",
        tokens[5], tokens[0], tokens[1], tokens[3], tokens[2], tokens[4], tokens[0], tokens[0]
    );
    let file = case.write("token.yaml", &text);
    let output = case.with_file(&file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let problems = stderr(&output);
    for path in [
        "instances[0].sources[0].github: ",
        "instances[0].roles[0].model: ",
        "instances[0].reports[0].channel: ",
        "instances[0].records: ",
        "instances[0].cadence.watch: ",
        "instances[1].name: ",
        "instances[2].name: ",
    ] {
        assert!(
            problems.contains(path),
            "no problem names `{path}`: {problems}"
        );
    }
    let shown = case.with_file(&file, &["config", "show", "--format", "json"]);
    for token in &tokens {
        for said in [&output, &shown] {
            assert!(
                !String::from_utf8_lossy(&said.stdout).contains(token.as_str())
                    && !String::from_utf8_lossy(&said.stderr).contains(token.as_str()),
                "a token-like value was printed: {}",
                describe(said)
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// `config show` renders the same document as text, JSON and YAML, and its YAML is a config file.
// ---------------------------------------------------------------------------------------------

#[test]
fn show_yaml_is_a_config_file_that_shows_the_same() {
    let case = Case::new("show-yaml");
    let file = fixture("example.yaml");
    let yaml = case.with_file(&file, &["config", "show", "--format", "yaml"]);
    assert_eq!(yaml.status.code(), Some(0), "{}", describe(&yaml));
    let written = case.write("shown.yaml", &String::from_utf8_lossy(&yaml.stdout));
    let validated = case.with_file(&written, &["config", "validate"]);
    assert_eq!(validated.status.code(), Some(0), "{}", describe(&validated));
    assert_eq!(case.show(&written, &[]), case.show(&file, &[]));

    let defaults = case.run(&["config", "show", "--format", "yaml"]);
    assert_eq!(defaults.status.code(), Some(0), "{}", describe(&defaults));
    let written = case.write("defaults.yaml", &String::from_utf8_lossy(&defaults.stdout));
    let reread = case.show(&written, &[]);
    let built_in = json(&case.run(&["config", "show", "--format", "json"]));
    assert_eq!(
        reread, built_in,
        "the built-in defaults round-trip through a file"
    );
}

#[test]
fn show_text_prints_one_line_per_yaml_path() {
    let case = Case::new("show-text");
    let output = case.with_file(&fixture("example.yaml"), &["config", "show"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let text = String::from_utf8_lossy(&output.stdout);
    for line in [
        "version: conductor.config/1",
        "instances[0].name: example-org",
        "instances[0].sources[0].github: example-org",
        "instances[0].cadence.watch: 180s",
        "instances[0].thresholds.disk_low: 15G",
        "instances[0].reports[0].when[0]: daily",
    ] {
        assert!(
            text.lines().any(|shown| shown == line),
            "no line `{line}`: {text}"
        );
    }
}

#[test]
fn show_refuses_a_format_it_does_not_render() {
    let case = Case::new("show-format");
    let output = case.with_file(
        &fixture("example.yaml"),
        &["config", "show", "--format", "markdown"],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        stderr(&output).contains("markdown"),
        "{}",
        describe(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// story:catalog-source: an instance's catalog is an optional source the file names.
// ---------------------------------------------------------------------------------------------

/// A `catalog` names a repository under the checkouts root, a directory on its `origin/main` and
/// how an entry's file stem spells a name: `plain` or `hex`, `plain` when absent. Each validates
/// and is shown as written, in JSON, as text lines, and through a YAML round trip.
#[test]
fn a_catalog_validates_and_shows_as_written_with_each_names_value() {
    let case = Case::new("catalog-shown");
    for (name, catalog, names) in [
        (
            "plain",
            "{repository: registry, path: catalog/entries, names: plain}",
            "plain",
        ),
        (
            "hex",
            "{repository: group/registry, path: store/subjects/7265706f, names: hex}",
            "hex",
        ),
        (
            "absent-names",
            "{repository: registry, path: entries}",
            "plain",
        ),
    ] {
        let file = case.write(
            &format!("{name}.yaml"),
            &one_instance(&format!("\x20   catalog: {catalog}\n")),
        );
        let validated = case.with_file(&file, &["config", "validate"]);
        assert_eq!(
            validated.status.code(),
            Some(0),
            "{name}: {}",
            describe(&validated)
        );
        let shown = case.show(&file, &[]);
        let catalog = &instance(&shown)["catalog"];
        let repository = catalog["repository"].as_str().expect("a repository");
        let path = catalog["path"].as_str().expect("a path");
        assert_eq!(catalog["names"], names, "{name}: {shown:#}");
        assert_eq!(
            catalog
                .as_object()
                .map(|map| map.keys().cloned().collect::<Vec<_>>()),
            Some(vec![
                "names".to_owned(),
                "path".to_owned(),
                "repository".to_owned()
            ]),
            "{name}: {shown:#}"
        );

        let text = case.with_file(&file, &["config", "show"]);
        assert_eq!(text.status.code(), Some(0), "{}", describe(&text));
        let text = String::from_utf8_lossy(&text.stdout);
        for line in [
            format!("instances[0].catalog.repository: {repository}"),
            format!("instances[0].catalog.path: {path}"),
            format!("instances[0].catalog.names: {names}"),
        ] {
            assert!(
                text.lines().any(|shown| shown == line),
                "{name}: no line `{line}`: {text}"
            );
        }

        let yaml = case.with_file(&file, &["config", "show", "--format", "yaml"]);
        assert_eq!(yaml.status.code(), Some(0), "{}", describe(&yaml));
        let written = case.write(
            &format!("{name}-shown.yaml"),
            &String::from_utf8_lossy(&yaml.stdout),
        );
        assert_eq!(case.show(&written, &[]), shown, "{name}: the round trip");
    }
}

/// Without a `catalog` key an instance names no catalog, and neither does the built-in one:
/// `config show` writes it as null.
#[test]
fn without_a_catalog_key_an_instance_names_no_catalog() {
    let case = Case::new("catalog-absent");
    let file = case.write("none.yaml", &one_instance(""));
    let shown = case.show(&file, &[]);
    assert_eq!(instance(&shown)["catalog"], Value::Null, "{shown:#}");
    let null = case.write("null.yaml", &one_instance("\x20   catalog: null\n"));
    assert_eq!(case.show(&null, &[]), shown, "a null catalog is no catalog");

    let built_in = json(&case.run(&["config", "show", "--format", "json"]));
    assert_eq!(instance(&built_in)["catalog"], Value::Null, "{built_in:#}");
}

/// A catalog that is not a mapping, has an unknown key, misses `repository` or `path`, names a
/// path that is not relative to the repository's top, or spells `names` other than `plain` or
/// `hex`, is refused by its YAML path.
#[test]
fn a_catalog_is_refused_by_its_path() {
    let case = Case::new("catalog-refused");
    for (name, catalog, path) in [
        ("not-a-mapping", "registry", "instances[0].catalog: "),
        (
            "unknown-key",
            "{repository: registry, path: entries, entity: x}",
            "instances[0].catalog.entity: ",
        ),
        (
            "no-repository",
            "{path: entries}",
            "instances[0].catalog.repository: ",
        ),
        (
            "no-path",
            "{repository: registry}",
            "instances[0].catalog.path: ",
        ),
        (
            "bad-names",
            "{repository: registry, path: entries, names: base64}",
            "instances[0].catalog.names: ",
        ),
        (
            "names-upper",
            "{repository: registry, path: entries, names: Hex}",
            "instances[0].catalog.names: ",
        ),
        (
            "absolute-path",
            "{repository: registry, path: /entries}",
            "instances[0].catalog.path: ",
        ),
        (
            "dotted-path",
            "{repository: registry, path: entries/../other}",
            "instances[0].catalog.path: ",
        ),
        (
            "absolute-repository",
            "{repository: /srv/registry, path: entries}",
            "instances[0].catalog.repository: ",
        ),
        (
            "empty-repository",
            "{repository: \"\", path: entries}",
            "instances[0].catalog.repository: ",
        ),
    ] {
        let file = case.write(
            &format!("{name}.yaml"),
            &one_instance(&format!("\x20   catalog: {catalog}\n")),
        );
        let problems = refused(&case, &file);
        assert!(problems.contains(path), "{name}: {problems}");
    }
}
