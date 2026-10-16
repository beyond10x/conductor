//! `story:instance-isolation`: two instances of one config file never read or write each other's
//! records, state, leases or messages, and share one host ledger.
//!
//! Each case is a directory of its own in this test target's temporary directory, holding a
//! config file `conductor.yaml` of two fictional instances, `alpha` and `beta`, each with its
//! own `records/`, `state/`, `cache/`, checkouts and trees under `alpha/` and `beta/`, and a home
//! directory `home/`. The binary runs with `HOME` there and the config named by
//! `CONDUCTOR_CONFIG`; nothing reads the real home.

mod active;
mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::config::{CONFIG_VARIABLE, INSTANCE_VARIABLE};
use conductor_cli::gc::reclaim::{MARGIN, READINGS_VARIABLE, VAR_TMP_VARIABLE};
use conductor_model::behaviour::HostGrantStorage;
use conductor_model::config::InstanceName;
use conductor_model::host::{GrantId, GrantKind, HostGrantData, HostGrantSnapshot, HostGrantState};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::Timestamp;
use serde_json::Value;

const GIB: u64 = 1 << 30;

/// The instances of every case.
const INSTANCES: [&str; 2] = ["alpha", "beta"];

struct Case {
    dir: PathBuf,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("instance_isolation")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("work")).expect("create the case's work directory");
        fs::create_dir_all(dir.join("var-tmp")).expect("create the case's /var/tmp");
        let case = Self { dir };
        case.write_config("");
        case
    }

    /// Writes the config file of `alpha` and `beta`, `extra` added to each instance.
    fn write_config(&self, extra: &str) {
        let mut text = String::from("version: conductor.config/1\ninstances:\n");
        for name in INSTANCES {
            let at = self.dir.join(name);
            for sub in ["records", "state", "cache", "checkouts", "trees"] {
                fs::create_dir_all(at.join(sub)).expect("create an instance directory");
            }
            text.push_str(&format!(
                "  - name: {name}\n\
                 \x20   session_prefix: {name}\n\
                 \x20   sources: [{{local: {checkouts:?}}}]\n\
                 \x20   checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n\
                 \x20   records: {records:?}\n\
                 \x20   state: {state:?}\n\
                 \x20   cache: {cache:?}\n\
                 \x20   thresholds: {{watchdog_every: 1s}}\n{extra}",
                checkouts = at.join("checkouts"),
                trees = at.join("trees"),
                records = at.join("records"),
                state = at.join("state"),
                cache = at.join("cache"),
            ));
        }
        fs::write(self.config(), text).expect("write the config file");
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// The host directory: `host/` beside the config file.
    fn host(&self) -> PathBuf {
        self.dir.join("host")
    }

    fn state(&self, instance: &str) -> PathBuf {
        self.dir.join(instance).join("state")
    }

    /// The binary for `instance`, in `work/`, free disk read from `readings.txt`.
    fn conductor(&self, instance: &str) -> Command {
        let mut command = common::conductor(self.dir.join("home"));
        command
            .env(CONFIG_VARIABLE, self.config())
            .env(INSTANCE_VARIABLE, instance)
            .env(READINGS_VARIABLE, self.dir.join("readings.txt"))
            .env(VAR_TMP_VARIABLE, self.dir.join("var-tmp"))
            .current_dir(self.dir.join("work"));
        command
    }

    fn run(&self, instance: &str, args: &[&str]) -> Output {
        self.conductor(instance)
            .args(args)
            .output()
            .expect("conductor runs")
    }

    /// The rows of `host grants` as `instance` lists them.
    fn grants(&self, instance: &str) -> Vec<Value> {
        self.listed(instance)
            .unwrap_or_else(|output| panic!("host grants: {output:?}"))
    }

    /// The rows of `host grants` as `instance` lists them, or the output of a listing that failed.
    fn listed(&self, instance: &str) -> Result<Vec<Value>, Output> {
        let output = self.run(instance, &["host", "grants", "--format", "json"]);
        if !output.status.success() {
            return Err(output);
        }
        match serde_json::from_slice(&output.stdout).expect("json rows") {
            Value::Array(rows) => Ok(rows),
            other => panic!("rows: {other}"),
        }
    }
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        thread::sleep(Duration::from_millis(50));
    }
}

fn output_of(child: Child) -> Output {
    child.wait_with_output().expect("the run ends")
}

/// Item 4: `alpha`'s and `beta`'s `build run` reserve in one ledger, `host/` beside the config
/// file. Free disk is 10G over the floor; `alpha` runs holding 6G, so `beta`'s 6G does not fit
/// beside it and waits for it, then runs once `alpha` released. With one ledger per instance,
/// `beta` would not see `alpha`'s reservation and would run at once.
#[test]
fn two_instances_build_runs_queue_in_one_host_ledger() {
    let case = Case::new("one-ledger");
    // The floor is `disk_admit` (20G by default) and reclaim's margin.
    let free = 20 * GIB + MARGIN + 10 * GIB;
    fs::write(case.dir.join("readings.txt"), format!("{free}\n")).expect("write the readings");
    // A reclaim that ran a moment ago and reached nothing: `beta` reuses it instead of walking the
    // disk (`build::RECLAIM_REUSE`). It is the host's, so it lies in the host directory.
    fs::create_dir_all(case.host()).expect("create the host directory");
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    fs::write(
        case.host().join("host.reclaim"),
        serde_json::json!({
            "started": now, "finished": now, "need": u64::MAX / 2, "reached": false,
            "reclaimed": 0, "left": [],
        })
        .to_string(),
    )
    .expect("write the last reclaim");

    let started = case.dir.join("alpha.started");
    let go = case.dir.join("go");
    let script = format!(
        "touch {started:?}; while [ ! -e {go:?} ]; do sleep 0.05; done",
        started = started,
        go = go
    );
    let alpha = case
        .conductor("alpha")
        .args(["build", "run", "--reserve", "6G", "--", "sh", "-c", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("alpha's build run starts");
    wait_until("alpha's command runs", || started.exists());

    let beta = case
        .conductor("beta")
        .args(["build", "run", "--reserve", "6G", "--", "true"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("beta's build run starts");
    let mut waiting = false;
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        // A listing that fails (no ledger yet) lists nothing.
        let rows = case.listed("alpha").unwrap_or_default();
        if rows
            .iter()
            .any(|row| row["instance"] == "beta" && row["state"] == "Requested")
        {
            waiting = true;
            break;
        }
        if rows
            .iter()
            .any(|row| row["instance"] == "beta" && row["state"] != "Requested")
        {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    fs::write(&go, "").expect("let alpha's command end");
    let alpha = output_of(alpha);
    let beta = output_of(beta);
    assert!(alpha.status.success(), "alpha: {alpha:?}");
    assert!(beta.status.success(), "beta: {beta:?}");
    assert!(
        waiting,
        "beta's request waits in the ledger alpha's run holds: {:?}",
        case.grants("alpha")
    );
    assert!(
        String::from_utf8_lossy(&beta.stderr).contains("waiting"),
        "beta names its wait: {beta:?}"
    );
    for instance in INSTANCES {
        let rows = case.grants(instance);
        let owners: Vec<&Value> = rows.iter().map(|row| &row["instance"]).collect();
        assert!(
            owners.contains(&&Value::from("alpha")) && owners.contains(&&Value::from("beta")),
            "{instance} lists both instances' grants of the one ledger: {rows:?}"
        );
    }
    for instance in INSTANCES {
        for name in [
            "host.lock",
            "host-runs",
            "host.reclaim",
            "host.reclaim.lock",
        ] {
            assert!(
                !case.state(instance).join(name).exists(),
                "{instance}'s state holds no {name}"
            );
        }
    }
    assert!(
        case.host().join("host.lock").exists(),
        "the lock is the host's"
    );
}

/// A grant as an older binary kept it in an instance's store.
fn legacy_grant(id: &str, state: HostGrantState) -> HostGrantSnapshot {
    HostGrantSnapshot {
        state,
        data: HostGrantData {
            grant_id: GrantId(id.to_owned()),
            instance: InstanceName("alpha".to_owned()),
            repository: Some(RepositoryName("example-repo".to_owned())),
            kind: GrantKind::Disk,
            bytes: Some(i64::try_from(GIB).unwrap()),
            peak_bytes: None,
            pauses: None,
            command: Some("cargo test".to_owned()),
            process_group: None,
            session_name: None,
            reclaimed_bytes: None,
            shortfall_bytes: None,
            requested_at: Timestamp("2026-10-14T12:00:00Z".to_owned()),
            started_at: None,
            released_at: Some(Timestamp("2026-10-14T12:05:00Z".to_owned())),
            release_reason: None,
        },
    }
}

/// Item 4: what an instance kept of the host ledger in its `state` is moved, once, to the host
/// directory at its first host command: its grants, its run locks, the reclaim record and the
/// fall's marker. Its lock files are left behind no more; a second command moves nothing.
#[test]
fn an_instances_old_ledger_moves_to_the_host_directory_once() {
    let case = Case::new("migrate");
    let state = case.state("alpha");
    {
        let mut store = conductor_cli::store::open(&state).expect("alpha's store opens");
        HostGrantStorage::put(
            &mut store,
            legacy_grant(
                "0190a000-0000-7000-8000-000000000001",
                HostGrantState::Released,
            ),
        );
        store.check().expect("the store keeps the grant");
    }
    fs::write(state.join("host.lock"), "").unwrap();
    fs::write(state.join("host.reclaim.lock"), "").unwrap();
    fs::write(state.join("host.reclaim"), "{\"started\": 1}").unwrap();
    fs::write(state.join("host.fall"), "disk-example\n").unwrap();
    fs::create_dir_all(state.join("host-runs")).unwrap();
    fs::write(
        state.join("host-runs/0190a000-0000-7000-8000-000000000002.lock"),
        "",
    )
    .unwrap();

    let rows = case.grants("alpha");
    assert_eq!(
        rows.len(),
        1,
        "the old grant is in the host ledger: {rows:?}"
    );
    assert_eq!(rows[0]["grant_id"], "0190a000-0000-7000-8000-000000000001");
    assert_eq!(case.grants("beta").len(), 1, "beta reads the same ledger");
    for name in [
        "host.lock",
        "host.reclaim.lock",
        "host.reclaim",
        "host.fall",
        "host-runs",
    ] {
        assert!(!state.join(name).exists(), "{name} left alpha's state");
    }
    assert_eq!(
        fs::read_to_string(case.host().join("host.reclaim")).unwrap(),
        "{\"started\": 1}",
        "the reclaim record moved"
    );
    assert!(
        fs::read_to_string(case.host().join("host.fall"))
            .unwrap()
            .starts_with("disk-example\nalpha"),
        "the fall's marker moved, naming its instance"
    );
    assert!(
        case.host()
            .join("host-runs/0190a000-0000-7000-8000-000000000002.lock")
            .exists(),
        "the run lock moved"
    );
    {
        let store = conductor_cli::store::open(&state).expect("alpha's store opens");
        assert!(
            HostGrantStorage::list(&store).is_empty(),
            "alpha's store holds no grant any more"
        );
    }
    assert!(state.join("host.migrated").exists(), "the move is recorded");

    // Once: a grant written to the old place afterwards (an older binary still running) stays.
    {
        let mut store = conductor_cli::store::open(&state).expect("alpha's store opens");
        HostGrantStorage::put(
            &mut store,
            legacy_grant(
                "0190a000-0000-7000-8000-000000000003",
                HostGrantState::Released,
            ),
        );
        store.check().expect("the store keeps the grant");
    }
    assert_eq!(case.grants("alpha").len(), 1, "nothing is moved twice");
}

/// A settings file that pins `instance`: its `env` names it, and its hook command runs `conductor`
/// with it first.
fn pinned(instance: &str) -> String {
    serde_json::json!({
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Item 1: `config validate` names, by its `settings` path, a role whose settings file pins
/// another instance than the role's, and exits 1. A file that pins no instance (no
/// `env.CONDUCTOR_INSTANCE`, or a hook command that runs `conductor` without
/// `CONDUCTOR_INSTANCE=<name>` first), as the shipped ones do, is a note on standard error and
/// the file is valid: `session start` and `spawn` start the role with a pinned copy. A pinned
/// role is valid with no note.
#[test]
fn config_validate_names_a_role_whose_settings_pin_another_instance() {
    let case = Case::new("validate-pin");
    let settings = case.dir.join("settings.json");
    case.write_config(&format!(
        "    roles: [{{role: conductor, harness: claude, model: opus, settings: {settings:?}}}]\n"
    ));
    fs::write(&settings, pinned("alpha")).unwrap();
    // `alpha`'s role is pinned to alpha, `beta`'s to alpha as well: beta's is named.
    let output = case.run("alpha", &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        stderr(&output).contains("instances[1].roles[0].settings")
            && !stderr(&output).contains("instances[0].roles[0].settings"),
        "beta's role pins alpha: {output:?}"
    );
    // A hook pinned to beta in a file whose env pins alpha pins another instance too.
    let foreign_hook = serde_json::json!({
        "env": {"CONDUCTOR_INSTANCE": "alpha"},
        "hooks": {"SessionStart": [{"hooks": [{
            "type": "command",
            "command": "CONDUCTOR_INSTANCE=beta conductor session claim --from-session-start",
        }]}]},
    })
    .to_string();
    fs::write(&settings, foreign_hook).unwrap();
    let output = case.run("alpha", &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        stderr(&output).contains("instances[0].roles[0].settings")
            && stderr(&output).contains("\"beta\""),
        "alpha's role has a hook pinned to beta: {output:?}"
    );
    let unprefixed = serde_json::json!({
        "hooks": {"SessionStart": [{"hooks": [{
            "type": "command",
            "command": "conductor session claim --from-session-start || exit 2",
        }]}]},
    })
    .to_string();
    let shipped = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.claude/conductor-settings.json"),
    )
    .unwrap();
    for (what, text) in [
        ("no env", "{}".to_owned()),
        ("a hook without it", unprefixed),
        ("the shipped conductor settings", shipped),
    ] {
        fs::write(&settings, text).unwrap();
        let output = case.run("alpha", &["config", "validate"]);
        assert_eq!(output.status.code(), Some(0), "{what}: {output:?}");
        assert!(
            stderr(&output).contains("note: instances[0].roles[0].settings")
                && stderr(&output).contains("note: instances[1].roles[0].settings")
                && stderr(&output).contains(".settings.json"),
            "{what}: each role's unpinned file is a note naming its pinned copy: {output:?}"
        );
    }
    // Each instance its own pinned file: valid, with no note.
    let beta = case.dir.join("settings-beta.json");
    fs::write(&settings, pinned("alpha")).unwrap();
    fs::write(&beta, pinned("beta")).unwrap();
    let text = fs::read_to_string(case.config()).unwrap();
    let at = text.rfind(&format!("{settings:?}")).unwrap();
    let mut fixed = text.clone();
    fixed.replace_range(at..at + format!("{settings:?}").len(), &format!("{beta:?}"));
    fs::write(case.config(), fixed).unwrap();
    let output = case.run("alpha", &["config", "validate"]);
    assert!(output.status.success(), "{output:?}");
    assert!(!stderr(&output).contains("note:"), "{output:?}");
}

/// The raise of one decision request, which writes the store and the records' decision log.
const RAISE: [&str; 14] = [
    "decision",
    "raise-decision-request",
    "--request-id",
    "r-1",
    "--repository",
    "example-repo",
    "--question",
    "q?",
    "--options",
    "A: a; B: b",
    "--recommendation",
    "A",
    "--raised-at",
    "2026-10-15T09:00:00Z",
];

/// Item 2: without `CONDUCTOR_INSTANCE`, a command run in an instance's records, state,
/// checkouts root or managed trees acts for that instance, not for the file's `default`; one run
/// where two instances' trees meet is refused (`AmbiguousInstance`); one run elsewhere acts for
/// the default.
#[test]
fn without_conductor_instance_the_working_directory_selects_the_instance() {
    let case = Case::new("by-cwd");
    let text = fs::read_to_string(case.config()).unwrap();
    fs::write(
        case.config(),
        text.replace("instances:\n", "default: alpha\ninstances:\n"),
    )
    .unwrap();
    let run_in = |cwd: &PathBuf| {
        case.conductor("alpha")
            .env_remove(INSTANCE_VARIABLE)
            .current_dir(cwd)
            .args(RAISE)
            .output()
            .expect("conductor runs")
    };
    for sub in ["records", "checkouts", "trees", "state"] {
        let cwd = case.dir.join("beta").join(sub);
        let output = run_in(&cwd);
        assert!(output.status.success(), "in beta's {sub}: {output:?}");
        assert!(
            case.state("beta").join("tree").exists(),
            "in beta's {sub}, beta's store is written"
        );
        assert!(
            !case.state("alpha").join("tree").exists(),
            "in beta's {sub}, alpha's store is not written"
        );
        fs::remove_dir_all(case.state("beta").join("tree")).unwrap();
        let _ = fs::remove_dir_all(case.dir.join("beta/records/decisions"));
    }
    let output = run_in(&case.dir.join("work"));
    assert!(output.status.success(), "elsewhere: {output:?}");
    assert!(
        case.state("alpha").join("tree").exists(),
        "elsewhere, the default instance acts"
    );

    // Two instances whose checkouts root is one directory: neither is chosen there.
    let shared = case.dir.join("shared");
    fs::create_dir_all(shared.join("repo")).unwrap();
    let text = fs::read_to_string(case.config()).unwrap();
    let mut text = text;
    for name in INSTANCES {
        text = text.replace(
            &format!("root: {:?}", case.dir.join(name).join("checkouts")),
            &format!("root: {shared:?}"),
        );
    }
    fs::write(case.config(), text).unwrap();
    let output = run_in(&shared.join("repo"));
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        stderr(&output).contains("AmbiguousInstance"),
        "names the refusal: {output:?}"
    );
}

/// Item 3: a command that writes the instance's records refuses a `--state-dir` other than the
/// instance's `state` (`StateDirNotInstance`) and writes nothing there; the instance's own
/// `state` is taken, and a view may read another.
#[test]
fn a_record_writing_command_refuses_another_state_dir() {
    let case = Case::new("state-dir");
    let other = case.dir.join("elsewhere");
    let mut args = vec!["--state-dir", other.to_str().unwrap()];
    args.extend(RAISE);
    let output = case.run("alpha", &args);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        stderr(&output).contains("StateDirNotInstance"),
        "names the refusal: {output:?}"
    );
    assert!(!other.exists(), "nothing is written to the other directory");
    let publish = case.run(
        "alpha",
        &[
            "--state-dir",
            other.to_str().unwrap(),
            "snapshot",
            "publish-board",
        ],
    );
    assert_eq!(publish.status.code(), Some(1), "{publish:?}");
    assert!(
        stderr(&publish).contains("StateDirNotInstance"),
        "{publish:?}"
    );

    let own = case.state("alpha");
    let mut args = vec!["--state-dir", own.to_str().unwrap()];
    args.extend(RAISE);
    let output = case.run("alpha", &args);
    assert!(output.status.success(), "its own state: {output:?}");
    let view = case.run(
        "alpha",
        &[
            "--state-dir",
            own.to_str().unwrap(),
            "decision",
            "requests",
            "--format",
            "json",
        ],
    );
    assert!(view.status.success(), "a view: {view:?}");
}

/// Item 5: when the config file does not load, the guard denies a call it would allow, and names
/// why, rather than deciding by the built-in paths.
#[test]
fn the_guard_denies_when_the_config_does_not_load() {
    let case = Case::new("guard-config");
    let broken = case.dir.join("broken.yaml");
    fs::write(&broken, "version: conductor.config/1\ncolour: blue\n").unwrap();
    let payload = serde_json::json!({
        "session_id": "s-1",
        "cwd": case.dir.join("work"),
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": {"file_path": case.dir.join("work/x")},
    });
    let run = |config: &PathBuf| {
        let mut child = case
            .conductor("alpha")
            .env(CONFIG_VARIABLE, config)
            .args(["guard", "record-guard-decision", "--from-pre-tool-use"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the guard runs");
        std::io::Write::write_all(
            child.stdin.as_mut().unwrap(),
            payload.to_string().as_bytes(),
        )
        .unwrap();
        child.wait_with_output().expect("the guard answers")
    };
    let allowed = run(&case.config());
    assert_eq!(
        allowed.status.code(),
        Some(0),
        "a read is allowed: {allowed:?}"
    );
    let denied = run(&broken);
    assert_eq!(denied.status.code(), Some(2), "{denied:?}");
    assert!(
        stderr(&denied).contains("ConfigUnloaded") && stderr(&denied).contains("colour"),
        "the denial names the config's problem: {denied:?}"
    );
}

/// Item 6: `dashboard serve` without `--port` listens on the instance's `dashboard_port`.
#[test]
fn dashboard_serve_listens_on_the_instance_s_port() {
    let case = Case::new("dashboard-port");
    let port = std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let text = fs::read_to_string(case.config()).unwrap();
    let at = text.rfind("    thresholds:").unwrap();
    let mut text = text;
    text.insert_str(at, &format!("    dashboard_port: {port}\n"));
    fs::write(case.config(), text).unwrap();
    let mut child = case
        .conductor("beta")
        .args(["dashboard", "serve"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the dashboard starts");
    let mut line = String::new();
    let read = std::io::BufRead::read_line(
        &mut std::io::BufReader::new(child.stdout.take().unwrap()),
        &mut line,
    );
    let _ = child.kill();
    let output = child.wait_with_output().expect("the dashboard ends");
    read.expect("read its first line");
    assert_eq!(
        line.trim(),
        format!("listening on http://127.0.0.1:{port}"),
        "beta's port: {output:?}"
    );
}
