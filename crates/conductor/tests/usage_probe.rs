//! `story:usage-probe`: conductor reads each account's usage headroom through the instance's
//! `usage.probe`, records each reading in the snapshot, forecasts when the pool runs out, prints
//! that line in the watch and in `conductor status`, and `conductor doctor` reports the rotation
//! watcher `usage.watcher` running or not.
//!
//! The probe's output is `tests/fixtures/usage/probe.json`, the shape the rotator on the
//! development host prints with every e-mail address, name and id replaced (`account-a`,
//! `account-b`, `account-c`); an earlier reading is written by [`probe`] in the same shape. The
//! snapshot case runs the built binary's `snapshot start-snapshot` with `CONDUCTOR_CONFIG` naming
//! a config file of the case, `HOME` the case's `home/`, and a `PATH` holding only the fake
//! programs the case writes (and `sh`, which runs the probe). The forecast, failure and watch
//! cases take snapshots through the library over the `usage` collector alone, its probe `cat` of
//! a file the case writes, at fixed instants. The status, config and doctor cases run the built
//! binary on the case's own directories. No case reaches a real account, the real session list
//! or the real `systemctl`.

mod active;
mod common;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::Duration;

use conductor_cli::collect::Collector;
use conductor_cli::collect::usage::{self, Forecast};
use conductor_cli::snapshot;
use conductor_cli::store;
use conductor_cli::watch::{Sources, Watch};
use conductor_model::behaviour::Generated;
use conductor_model::observation::obligations::{MemberFailuresQuery, UsageReadingsQuery};
use conductor_model::observation::{SnapshotState, StartSnapshot};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The probe's output the newest reading of every case answers with.
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/usage/probe.json"
);

/// What the fixture names an account's address with; nothing of it may be kept.
const ADDRESS: &str = "example.org";

/// The line the two readings of [`two_readings`] give: `seven_day` has 60 % left on the usable
/// `account-a` (`account-b` is exhausted) and used 10 % in two hours, so it runs out in 12 hours,
/// before `five_hour` (80 % left, 10 % in two hours: 16 hours); the next reset of a window in use
/// is `account-a`'s `five_hour`, at 13:00.
const TWO_READINGS: &str =
    "usage: pool runs out ~2026-10-15 00:00 UTC, next reset 2026-10-14 13:00 UTC";

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// One case's directory: `home/` with the checkouts root `home/src/` and the managed trees
/// `home/trees/`, the instance's `records/` and `state/`, `bin/` for the programs the binary
/// finds, and an empty `work/` it runs from.
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

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("usage_probe")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in [
            "home/src",
            "home/trees",
            "records/.engineering",
            "work",
            "bin",
        ] {
            fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
        }
        fs::write(
            dir.join("records/.engineering/workspace.yaml"),
            "version: aep.workspace/1\nmembers: []\n",
        )
        .expect("write the records' workspace file");
        let dir = dir.canonicalize().expect("the case's directory");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    /// The config file of one instance, `acme`, on GitHub, its checkouts under `home/`, its
    /// records and state in the case's directory, with `usage` lines appended.
    fn config(&self, usage: &str) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\ninstances:\n  - name: acme\n    sources: \
             [{{github: example-org}}]\n    checkouts:\n      root: {}\n      trees: {}\n    \
             records: {}\n    state: {}\n{usage}",
            self.home().join("src").display(),
            self.home().join("trees").display(),
            self.dir.join("records").display(),
            self.state().display(),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    /// The program `name` on the binary's `PATH`, running `body`.
    fn program(&self, name: &str, body: &str) {
        let file = self.bin().join(name);
        fs::write(&file, format!("#!/bin/sh\n{body}\n")).expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// The built binary run from `work/` with `CONDUCTOR_CONFIG` naming `config` when given,
    /// `HOME` naming `home/` and `PATH` only `bin/`.
    fn conductor(&self, config: Option<&Path>, args: &[&str]) -> Output {
        let mut command = common::conductor(self.home());
        command
            .current_dir(self.dir.join("work"))
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin());
        if let Some(config) = config {
            command.env("CONDUCTOR_CONFIG", config);
        }
        command
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    /// Takes a complete snapshot started at `at` into the store under `state/`, whose one
    /// collector is the `usage` collector over a probe that prints `printed`, read at `at`.
    fn take(&self, at: &str, printed: &str) {
        let file = self.dir.join(format!("probe-{at}.json"));
        fs::write(&file, printed).expect("write the probe's output");
        let probe = format!("cat '{}'", file.display());
        self.take_probe(at, &probe);
    }

    /// [`Case::take`] over the probe command `probe`.
    fn take_probe(&self, at: &str, probe: &str) {
        let probe = probe.to_owned();
        let read_at = instant(at);
        let collector = Collector::new(usage::COLLECTOR, move |recorder| {
            usage::collect_with(&probe, Some(read_at), recorder)
        });
        let store = store::open(&self.state()).expect("open the case's store");
        let start = StartSnapshot {
            started_at: Timestamp(at.to_owned()),
            disk_free_bytes: 1_000_000,
            budget: None,
        };
        let taken = snapshot::take_from(store, start, &[collector]).expect("the snapshot ends");
        assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    }

    /// Every file under `state/`, read as bytes and joined: what the store keeps.
    fn kept(&self) -> String {
        fn walk(dir: &Path, into: &mut Vec<u8>) {
            for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, into);
                } else if let Ok(bytes) = fs::read(&path) {
                    into.extend(bytes);
                }
            }
        }
        let mut bytes = Vec::new();
        walk(&self.state(), &mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

fn instant(text: &str) -> OffsetDateTime {
    OffsetDateTime::parse(text, &Rfc3339).expect("an RFC 3339 instant")
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The probe's output in the fixture's shape: `account-a` active and ready, `account-b`
/// exhausted, each with its `five_hour` and `seven_day` percents.
fn probe(a: (f64, f64), b: (f64, f64)) -> String {
    let account = |id: &str, active: bool, health: &str, (five, seven): (f64, f64)| {
        json!({
            "id": id,
            "email": format!("{id}@{ADDRESS}"),
            "provider": "claude",
            "active": active,
            "health": health,
            "usage": {"plan": "team", "windows": [
                {"name": "five_hour", "used_percent": five, "resets_at": "2026-10-14T13:00:00Z",
                 "model": null},
                {"name": "seven_day", "used_percent": seven, "resets_at": "2026-10-16T11:00:00Z",
                 "model": null},
            ]},
        })
    };
    json!({
        "version": 1,
        "accounts": [
            account("account-a", true, "ready", a),
            account("account-b", false, "exhausted", b),
        ],
    })
    .to_string()
}

fn fixture() -> String {
    fs::read_to_string(FIXTURE).expect("read the probe fixture")
}

/// The reading at 10:00 and the fixture's at 12:00 of 2026-10-14.
fn two_readings(case: &Case) {
    case.take("2026-10-14T10:00:00Z", &probe((10.0, 30.0), (5.0, 100.0)));
    case.take("2026-10-14T12:00:00Z", &fixture());
}

// ---------------------------------------------------------------------------------------------
// The snapshot records each reading
// ---------------------------------------------------------------------------------------------

/// `snapshot start-snapshot` runs the instance's `usage.probe` and records one row per window of
/// each account the probe prints with windows, with the instant it was read; an account is its
/// id, and no address the probe prints is kept anywhere in the store.
#[test]
fn the_snapshot_records_each_window_of_each_account_by_its_id_and_never_its_address() {
    let case = Case::new("snapshot-records");
    case.program("gh", "printf '%s' '[]'");
    case.program(
        "aep",
        "case \"$3\" in\n  members) printf '%s' '{\"members\": []}' ;;\n  *) printf '%s' \
         '{\"artifacts\": []}' ;;\nesac",
    );
    case.program("claude", "printf '%s' '[]'");
    case.program("usage-probe", &format!("printf '%s' '{}'", fixture()));
    std::os::unix::fs::symlink("/bin/sh", case.bin().join("sh")).expect("link sh");
    let config = case.config("    usage:\n      probe: usage-probe\n");
    let before = OffsetDateTime::now_utc() - time::Duration::seconds(1);

    let state = case.state();
    let state = state.to_str().expect("a UTF-8 path");
    let taken = case.conductor(
        Some(&config),
        &[
            "--state-dir",
            state,
            "snapshot",
            "start-snapshot",
            "--started-at",
            "2026-10-14T12:00:00Z",
            "--disk-free-bytes",
            "100000000000",
        ],
    );
    assert!(taken.status.success(), "{}", describe(&taken));

    let shown = case.conductor(
        Some(&config),
        &[
            "--state-dir",
            state,
            "snapshot",
            "usage-readings",
            "--format",
            "jsonl",
        ],
    );
    assert!(shown.status.success(), "{}", describe(&shown));
    let mut rows: Vec<Value> = String::from_utf8(shown.stdout.clone())
        .expect("UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("one JSON object a line"))
        .collect();
    for row in &mut rows {
        let fields = row.as_object_mut().expect("a row is an object");
        fields.remove("observation_id");
        fields.remove("snapshot_id");
        let read_at = fields
            .remove("read_at")
            .and_then(|at| at.as_str().map(instant))
            .expect("each row has the instant it was read");
        assert!(
            read_at >= before && read_at <= OffsetDateTime::now_utc(),
            "read while the snapshot ran: {read_at}"
        );
    }
    rows.sort_by_key(|row| (row["account"].to_string(), row["window"].to_string()));
    assert_eq!(
        rows,
        [
            json!({"account": "account-a", "active": true, "health": "ready",
                   "window": "five_hour", "used_percent": 20,
                   "resets_at": "2026-10-14T13:00:00Z"}),
            json!({"account": "account-a", "active": true, "health": "ready",
                   "window": "seven_day", "used_percent": 40,
                   "resets_at": "2026-10-16T11:00:00.254181Z"}),
            json!({"account": "account-b", "active": false, "health": "exhausted",
                   "window": "five_hour", "used_percent": 5,
                   "resets_at": "2026-10-14T15:59:59.953471Z"}),
            json!({"account": "account-b", "active": false, "health": "exhausted",
                   "window": "seven_day", "used_percent": 100,
                   "resets_at": "2026-10-15T01:59:59.953491Z"}),
        ],
        "{}",
        describe(&shown)
    );
    let kept = case.kept();
    assert!(kept.contains("account-a"), "the store holds the readings");
    assert!(
        !kept.contains(ADDRESS),
        "no address the probe printed is kept in the store"
    );
}

/// A probe that exits unsuccessfully, or prints an address where an id belongs, records no
/// reading: one member failure `usage` names what was wrong and never what the probe printed,
/// and the snapshot completes.
#[test]
fn a_probe_that_fails_or_prints_no_id_records_one_member_failure_and_no_reading() {
    let case = Case::new("probe-fails");
    case.take_probe(
        "2026-10-14T10:00:00Z",
        "echo account-a@example.org >&2; exit 3",
    );
    let printed = fixture().replace("\"id\": \"account-b\"", "\"id\": \"account-b@example.org\"");
    case.take("2026-10-14T11:00:00Z", &printed);

    let generated = Generated::new(store::open_existing(&case.state()).expect("the store"));
    let readings = generated.usage_readings().expect("the readings read");
    assert!(readings.is_empty(), "no reading is recorded: {readings:?}");
    let mut failures: Vec<(String, String, String)> = generated
        .member_failures()
        .expect("the failures read")
        .into_iter()
        .map(|row| (row.member.0, row.collector, row.error_line))
        .collect();
    failures.sort();
    assert_eq!(
        failures,
        [
            (
                "usage".to_owned(),
                "usage".to_owned(),
                "accounts[1].id is not a word of letters, digits, '.', '_', ':' and '-'".to_owned()
            ),
            (
                "usage".to_owned(),
                "usage".to_owned(),
                "the usage probe exited 3".to_owned()
            ),
        ]
    );
    assert!(!case.kept().contains(ADDRESS), "no address is kept");
}

// ---------------------------------------------------------------------------------------------
// The forecast, the watch and the status
// ---------------------------------------------------------------------------------------------

/// The headroom of a window is what the accounts that are not exhausted have left; the burn rate
/// what the accounts used between the last two readings; the pool runs out when its first window
/// does. One reading alone forecasts nothing.
#[test]
fn the_pool_runs_out_when_its_first_window_does_at_the_rate_of_the_last_two_readings() {
    let case = Case::new("forecast");
    let forecast = || Forecast::of(&store::open_existing(&case.state()).expect("the store"));

    case.take("2026-10-14T10:00:00Z", &probe((10.0, 30.0), (5.0, 100.0)));
    assert_eq!(
        forecast().map(|forecast| forecast.line()).as_deref(),
        Some("usage: pool runs out ~unknown, next reset 2026-10-14 13:00 UTC"),
        "one reading has no burn rate"
    );

    case.take("2026-10-14T12:00:00Z", &fixture());
    assert_eq!(
        forecast().map(|forecast| forecast.line()).as_deref(),
        Some(TWO_READINGS)
    );
}

/// The newest reading has nothing left on any usable account: the pool has run out at that
/// reading, whatever the burn rate.
#[test]
fn a_pool_with_nothing_left_has_run_out_at_its_newest_reading() {
    let case = Case::new("forecast-empty");
    case.take("2026-10-14T12:20:00Z", &probe((100.0, 100.0), (5.0, 100.0)));
    let forecast =
        Forecast::of(&store::open_existing(&case.state()).expect("the store")).expect("a reading");
    assert_eq!(
        forecast.line(),
        "usage: pool runs out ~2026-10-14 12:00 UTC, next reset 2026-10-14 13:00 UTC"
    );
}

/// The watch's sources: an empty session list, no transcripts, no CI, plenty of free disk, and
/// the clock at 2026-10-14T14:00:00Z.
fn quiet(case: &Case) -> Sources {
    fn plenty(_: &Path) -> anyhow::Result<u64> {
        Ok(1 << 50)
    }
    fn clock() -> OffsetDateTime {
        instant("2026-10-14T14:00:00Z")
    }
    Sources {
        agents: ["sh", "-c", "printf '[]'"].map(OsString::from).to_vec(),
        root: case.dir.join("none/src"),
        trees: case.dir.join("none/trees"),
        records: None,
        projects: case.dir.join("none/projects"),
        dispatches: case.dir.join("none/dispatches"),
        organization: String::new(),
        store: None,
        repositories: Vec::new(),
        runs: Vec::new(),
        disk: case.dir.clone(),
        free_bytes: plenty,
        notify: vec![OsString::from("true")],
        clock,
        bound: Duration::from_secs(10),
    }
}

/// One pass with CI; its lines, and nothing on standard error.
fn passed(watch: &mut Watch) -> Vec<String> {
    let mut notes = Vec::new();
    let lines = watch.pass(true, &mut notes).expect("the pass ends");
    assert_eq!(String::from_utf8_lossy(&notes), "", "no source failed");
    lines
}

/// The watch prints the usage line once per change of the forecast hour: the first reading, the
/// forecast the second one gives, nothing for a third that keeps the hour, and the line again
/// for a fourth that moves it.
#[test]
fn the_watch_prints_the_usage_line_once_per_change_of_the_forecast_hour() {
    let case = Case::new("watch");
    let mut watch = Watch::new(quiet(&case), case.state().join("watch")).with_usage(case.state());
    assert_eq!(passed(&mut watch), Vec::<String>::new(), "no reading yet");

    case.take("2026-10-14T10:00:00Z", &probe((10.0, 30.0), (5.0, 100.0)));
    assert_eq!(
        passed(&mut watch),
        ["usage: pool runs out ~unknown, next reset 2026-10-14 13:00 UTC"]
    );
    assert_eq!(passed(&mut watch), Vec::<String>::new(), "once");

    case.take("2026-10-14T12:00:00Z", &fixture());
    assert_eq!(passed(&mut watch), [TWO_READINGS]);
    assert_eq!(passed(&mut watch), Vec::<String>::new(), "once");

    // 5 % in the hour since, 55 % left: 11 hours, the same hour.
    case.take("2026-10-14T13:00:00Z", &probe((25.0, 45.0), (5.0, 100.0)));
    assert_eq!(
        passed(&mut watch),
        Vec::<String>::new(),
        "the forecast hour did not change"
    );

    // 10 % in the hour since, 45 % left: 4.5 hours.
    case.take("2026-10-14T14:00:00Z", &probe((30.0, 55.0), (5.0, 100.0)));
    assert_eq!(
        passed(&mut watch),
        ["usage: pool runs out ~2026-10-14 18:00 UTC, next reset 2026-10-16 11:00 UTC"]
    );
    assert_eq!(passed(&mut watch), Vec::<String>::new(), "once");
}

/// `conductor status` shows the line the watch prints, as its own part in text and as `usage` in
/// JSON; without a reading it says so.
#[test]
fn conductor_status_shows_the_usage_line() {
    let case = Case::new("status");
    case.take("2026-10-14T09:00:00Z", r#"{"accounts": []}"#);
    let state = case.state();
    let state = state.to_str().expect("a UTF-8 path");
    let text = case.conductor(None, &["--state-dir", state, "status"]);
    assert!(text.status.success(), "{}", describe(&text));
    assert!(
        String::from_utf8_lossy(&text.stdout).ends_with("## Usage\n\nNone: no usage reading.\n"),
        "{}",
        describe(&text)
    );

    two_readings(&case);
    let text = case.conductor(None, &["--state-dir", state, "status"]);
    assert!(text.status.success(), "{}", describe(&text));
    assert!(
        String::from_utf8_lossy(&text.stdout).ends_with(&format!("## Usage\n\n{TWO_READINGS}\n")),
        "{}",
        describe(&text)
    );
    let json = case.conductor(None, &["--state-dir", state, "status", "--format", "json"]);
    assert!(json.status.success(), "{}", describe(&json));
    let report: Value = serde_json::from_slice(&json.stdout).expect("one JSON object");
    assert_eq!(report["usage"], json!(TWO_READINGS), "{}", describe(&json));
}

// ---------------------------------------------------------------------------------------------
// The config and doctor
// ---------------------------------------------------------------------------------------------

/// `usage.probe` and `usage.watcher` are config keys `config show` gives back, null when absent;
/// a watcher that is no unit name, and a key `usage` does not declare, are refused by their path.
#[test]
fn the_config_names_the_probe_and_the_watcher_and_refuses_a_watcher_that_is_no_unit() {
    let case = Case::new("config");
    let show = |config: &Path| {
        let output = case.conductor(Some(config), &["config", "show", "--format", "json"]);
        assert!(output.status.success(), "{}", describe(&output));
        let shown: Value = serde_json::from_slice(&output.stdout).expect("one JSON object");
        shown["instances"][0]["usage"].clone()
    };
    assert_eq!(
        show(&case.config("")),
        json!({"probe": null, "watcher": null})
    );
    assert_eq!(
        show(&case.config(
            "    usage:\n      probe: usage-probe --json\n      watcher: rotator.service\n"
        )),
        json!({"probe": "usage-probe --json", "watcher": "rotator.service"})
    );

    let refused = |usage: &str, path: &str| {
        let output = case.conductor(Some(&case.config(usage)), &["config", "validate"]);
        assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(path),
            "names {path}: {}",
            describe(&output)
        );
    };
    refused(
        "    usage:\n      watcher: rotator service\n",
        "instances[0].usage.watcher",
    );
    refused(
        "    usage:\n      probe: \"\"\n",
        "instances[0].usage.probe",
    );
    refused("    usage:\n      every: 5m\n", "instances[0].usage.every");
}

/// `conductor doctor` reports the configured watcher running when `systemctl --user is-active`
/// answers `active`, and not running otherwise; without a watcher it prints no such line.
#[test]
fn doctor_reports_the_usage_watcher_running_or_not() {
    let case = Case::new("doctor");
    let calls = case.dir.join("systemctl.log");
    let systemctl = |answer: &str, code: i32| {
        case.program(
            "systemctl",
            &format!(
                "printf '%s\\n' \"$*\" >> '{}'\nprintf '%s\\n' '{answer}'\nexit {code}",
                calls.display()
            ),
        );
    };
    let doctor = |config: &Path| {
        let output = case.conductor(Some(config), &["doctor"]);
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| line.starts_with("usage watcher:"))
            .map(str::to_owned)
            .collect::<Vec<String>>()
    };
    let config = case.config("    usage:\n      watcher: rotator.service\n");

    systemctl("active", 0);
    assert_eq!(doctor(&config), ["usage watcher: rotator.service running"]);
    assert_eq!(
        fs::read_to_string(&calls).expect("systemctl was asked"),
        "--user is-active rotator.service\n"
    );

    systemctl("inactive", 3);
    assert_eq!(
        doctor(&config),
        ["usage watcher: rotator.service not running (inactive)"]
    );

    assert_eq!(doctor(&case.config("")), Vec::<String>::new());
}
