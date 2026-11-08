//! `story:placement-substrate-local`: a placement is a substrate box on this host.
//!
//! - `placement` of a `conductor.config/2` instance is a map `{daemon, adapter}`, or a bare
//!   string read as `{daemon: local, adapter: <string>}` (decision P3); `oci` and `pod` on a
//!   local daemon are refused at validate, by YAML path (decision P5).
//! - `placement::open_box` asks the substrate daemon, through the [`Substrate`] seam, for one
//!   box per instance with the profile of design § 6 (argv and cwd of `conductor instance
//!   serve`, read-only roots, the six writable places, limits, egress, the model token as a
//!   sealed slot) and records `BoxOpened`; a refusal is recorded as `BoxRefused` with
//!   substrate's name, answered as `PlacementRefused`, and nothing falls back.
//! - conductord starts the runtime of an `adapter: linux` instance in its box, never through the
//!   service manager, records the refusal on the reconciliation, and closes the box of an
//!   instance gone from the config (`BoxClosed`).
//! - `conductor doctor` checks `substrate-daemon` against its pin when an instance's placement
//!   is a substrate box, and says `not needed` otherwise (decision P6).
//!
//! Every case runs against a fake substrate ([`Fake`]); no real daemon, bubblewrap or unit runs.

mod active;
mod common;

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use conductor_cli::confine::systemd::Service;
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::units::Units;
use conductor_cli::placement::{
    self, Adapter, BoxRequest, Opened, Refused, Resolved, Substrate, Where,
};
use conductor_model::behaviour::{BoxRefusalStorage, BoxStorage, ReconciliationStorage};
use conductor_model::config::PlacementName;
use conductor_model::instance::{ReconcileAction, ReconcileResult};
use conductor_model::placement::BoxState;
use serde_json::Value;

// ---------------------------------------------------------------------------------------------
// The fake substrate and the fake service manager
// ---------------------------------------------------------------------------------------------

/// What the fake substrate saw and how it answers.
#[derive(Debug, Default)]
struct Log {
    opened: Vec<BoxRequest>,
    stopped: Vec<String>,
    closed: Vec<String>,
    running: BTreeSet<String>,
    refuse: Option<String>,
}

/// A substrate daemon that opens every box it is asked for, unless told to refuse.
#[derive(Debug, Default, Clone)]
struct Fake {
    log: Arc<Mutex<Log>>,
}

impl Fake {
    fn refusing(name: &str) -> Self {
        let fake = Self::default();
        fake.log.lock().expect("the log").refuse = Some(name.to_owned());
        fake
    }
}

impl Substrate for Fake {
    fn open(&mut self, request: &BoxRequest) -> Result<Opened, Refused> {
        let mut log = self.log.lock().expect("the log");
        if let Some(name) = &log.refuse {
            return Err(Refused {
                name: name.clone(),
                detail: "the probe proved no sandbox".to_owned(),
            });
        }
        log.opened.push(request.clone());
        let n = log.opened.len();
        let session = format!("ses_{n}");
        log.running.insert(session.clone());
        Ok(Opened {
            workspace: format!("ws_{n}"),
            session,
        })
    }

    fn running(&mut self, session: &str) -> Result<bool> {
        Ok(self.log.lock().expect("the log").running.contains(session))
    }

    fn stop(&mut self, session: &str) -> Result<()> {
        let mut log = self.log.lock().expect("the log");
        log.running.remove(session);
        log.stopped.push(session.to_owned());
        Ok(())
    }

    fn close(&mut self, workspace: &str) -> Result<()> {
        self.log
            .lock()
            .expect("the log")
            .closed
            .push(workspace.to_owned());
        Ok(())
    }
}

/// A service manager that records every call; a boxed runtime makes none.
#[derive(Debug, Default)]
struct Units0 {
    calls: Vec<String>,
}

impl Units for Units0 {
    fn start(&mut self, service: &Service) -> Result<()> {
        self.calls.push(format!("start {}", service.unit));
        Ok(())
    }
    fn stop(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("stop {unit}"));
        Ok(())
    }
    fn active(&mut self, unit: &str) -> Result<bool> {
        self.calls.push(format!("active {unit}"));
        Ok(false)
    }
    fn enable(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("enable {unit}"));
        Ok(())
    }
    fn disable(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("disable {unit}"));
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// One case's directory
// ---------------------------------------------------------------------------------------------

struct Case {
    dir: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("placement")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        for sub in ["home/work", "bin"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn url(&self) -> String {
        format!("sqlite:{}", self.dir.join("shared.sqlite").display())
    }

    /// A `conductor.config/2` file of one instance per `(name, placement)`, each placement
    /// written as given (a YAML flow value).
    fn config(&self, instances: &[(&str, &str)]) -> PathBuf {
        let mut text = format!(
            "version: conductor.config/2\ndaemon: {{port: 7300, store: {:?}}}\ninstances:\n",
            self.url()
        );
        for (name, placement) in instances {
            let root = self.dir.join(name);
            text.push_str(&format!(
                "  - name: {name}\n    placement: {placement}\n    session_prefix: {name}\n    \
                 sources: [{{github: example-org}}]\n    \
                 checkouts: {{root: {root:?}, trees: {trees:?}}}\n    \
                 records: {records:?}\n    state: {state:?}\n    cache: {cache:?}\n",
                root = root.join("checkouts"),
                trees = root.join("trees"),
                records = root.join("records"),
                state = root.join("state"),
                cache = root.join("cache"),
            ));
        }
        fs::write(self.file(), text).expect("write the config file");
        self.file()
    }

    fn conductor(&self, args: &[&str]) -> Output {
        common::conductor(self.home())
            .args(args)
            .current_dir(self.home().join("work"))
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    fn daemon(&self, substrate: Fake) -> Daemon<Units0> {
        let mut daemon = Daemon::open(
            self.file(),
            self.home(),
            self.dir.join("daemon"),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            Units0::default(),
        )
        .expect("open the daemon");
        daemon.substrate = Box::new(substrate);
        daemon
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\n--- stdout\n{}\n--- stderr\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// ---------------------------------------------------------------------------------------------
// P3, P5: the placement in the config
// ---------------------------------------------------------------------------------------------

/// The placement `config show` prints for the case's one instance.
fn shown_placement(case: &Case, file: &Path) -> Value {
    let output = case.conductor(&[
        "--config",
        &file.display().to_string(),
        "config",
        "show",
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let shown: Value = serde_json::from_slice(&output.stdout).expect("show prints JSON");
    shown["instances"][0]["placement"].clone()
}

#[test]
fn a_placement_map_and_a_bare_adapter_read_as_the_same_local_box() {
    let case = Case::new("map-and-bare");
    let file = case.config(&[("alpha", "{daemon: local, adapter: linux}")]);
    let output = case.conductor(&[
        "--config",
        &file.display().to_string(),
        "config",
        "validate",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(shown_placement(&case, &file), "linux");
    let file = case.config(&[("alpha", "linux")]);
    assert_eq!(shown_placement(&case, &file), "linux");
    let file = case.config(&[("alpha", "{adapter: systemd}")]);
    assert_eq!(shown_placement(&case, &file), "systemd");
}

#[test]
fn oci_and_pod_on_a_local_daemon_are_refused_at_validate_by_path() {
    let case = Case::new("local-refusals");
    for (placement, path, word) in [
        (
            "{daemon: local, adapter: oci}",
            "instances[0].placement.adapter",
            "oci",
        ),
        ("pod", "instances[0].placement", "pod"),
        (
            "{daemon: local, adapter: lxc}",
            "instances[0].placement.adapter",
            "lxc",
        ),
        (
            "{daemon: local, adapter: linux, image: x}",
            "instances[0].placement.image",
            "image",
        ),
        (
            "{daemon: tcp://box, adapter: linux}",
            "instances[0].placement.daemon",
            "tcp://box",
        ),
    ] {
        let file = case.config(&[("alpha", placement)]);
        let output = case.conductor(&[
            "--config",
            &file.display().to_string(),
            "config",
            "validate",
        ]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{placement}: {}",
            describe(&output)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr
                .lines()
                .any(|line| line.contains(path) && line.contains(word)),
            "{placement}: the refusal names {path} and {word}: {}",
            describe(&output)
        );
    }
}

#[test]
fn a_placement_name_resolves_to_its_daemon_and_adapter() {
    let local = |adapter| {
        Some(Resolved {
            daemon: Where::Local,
            adapter,
        })
    };
    assert_eq!(placement::resolve("linux"), local(Adapter::Linux));
    assert_eq!(placement::resolve("systemd"), local(Adapter::Systemd));
    assert_eq!(
        placement::resolve("linux@https://substrate.example.org"),
        Some(Resolved {
            daemon: Where::Remote("https://substrate.example.org".to_owned()),
            adapter: Adapter::Linux,
        })
    );
    // A name that names no adapter keeps the path it had before substrate placements.
    assert_eq!(placement::resolve("box-a"), None);
}

// ---------------------------------------------------------------------------------------------
// The box: opened, refused, closed
// ---------------------------------------------------------------------------------------------

fn linux_instance(case: &Case) -> conductor_model::config::Instance {
    let mut instance = conductor_cli::config::built_in(&case.home(), &case.home().join("work"));
    instance.placement = PlacementName("linux".to_owned());
    instance
}

#[test]
fn a_box_is_opened_with_the_profile_of_design_6_and_recorded() {
    let case = Case::new("box-opened");
    let instance = linux_instance(&case);
    let binary = Path::new("/opt/conductor/bin/conductor");
    let request = placement::request(&instance, &case.file(), binary, Vec::new());
    let mut store =
        conductor_cli::store::open_url(&case.url(), conductor_cli::daemon::TENANT).expect("store");
    let mut fake = Fake::default();
    let opened = placement::open_box(&mut store, &mut fake, &request).expect("the box opens");
    assert_eq!(opened.session, "ses_1");
    assert_eq!(opened.workspace, "ws_1");

    let log = fake.log.lock().expect("the log");
    let asked = &log.opened[0];
    assert_eq!(
        asked.argv,
        [
            "/opt/conductor/bin/conductor".to_owned(),
            "--config".to_owned(),
            case.file().display().to_string(),
            "instance".to_owned(),
            "serve".to_owned(),
        ]
    );
    assert_eq!(asked.cwd, instance.state);
    assert!(
        asked
            .read_only_roots
            .contains(&"/opt/conductor/bin".to_owned()),
        "the binary's directory is a read-only root: {asked:?}"
    );
    assert_eq!(
        asked.places,
        conductor_cli::config::PLACES.map(str::to_owned)
    );
    assert!(asked.egress.contains(&"github.com".to_owned()), "{asked:?}");
    assert!(
        asked.egress.contains(&placement::MODEL_HOST.to_owned()),
        "{asked:?}"
    );
    assert_eq!(
        asked.egress.len(),
        2,
        "nothing else is reachable: {asked:?}"
    );
    assert_eq!(asked.secret_slots, ["model_token".to_owned()]);
    assert!(
        asked.env.iter().all(|(_, value)| !value.contains("token=")),
        "no credential in env: {asked:?}"
    );

    let boxes = BoxStorage::list(&store);
    assert_eq!(boxes.len(), 1, "one box on record: {boxes:?}");
    assert_eq!(boxes[0].state, BoxState::Opened);
    assert_eq!(boxes[0].data.placement.0, "linux");
    assert_eq!(boxes[0].data.instance, instance.name);
    assert_eq!(boxes[0].data.box_id.0.0, opened.box_id);
    assert!(BoxRefusalStorage::list(&store).is_empty());

    drop(log);
    placement::close_box(&mut store, &mut fake, &opened).expect("the box closes");
    let boxes = BoxStorage::list(&store);
    assert_eq!(boxes[0].state, BoxState::Closed, "{boxes:?}");
    let log = fake.log.lock().expect("the log");
    assert_eq!(log.stopped, ["ses_1".to_owned()]);
    assert_eq!(log.closed, ["ws_1".to_owned()]);
}

#[test]
fn a_refused_box_is_recorded_with_substrates_name_and_nothing_falls_back() {
    let case = Case::new("box-refused");
    let instance = linux_instance(&case);
    let request = placement::request(
        &instance,
        &case.file(),
        Path::new("/opt/conductor/bin/conductor"),
        Vec::new(),
    );
    let mut store =
        conductor_cli::store::open_url(&case.url(), conductor_cli::daemon::TENANT).expect("store");
    let mut fake = Fake::refusing("exec.sandbox-unavailable");
    let refused = placement::open_box(&mut store, &mut fake, &request)
        .expect_err("substrate refused the box");
    assert_eq!(refused.reason, "exec.sandbox-unavailable");
    let line = refused.to_string();
    assert!(
        line.starts_with("PlacementRefused: ") && line.contains("exec.sandbox-unavailable"),
        "{line}"
    );
    let refusals = BoxRefusalStorage::list(&store);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].data.reason.0, "exec.sandbox-unavailable");
    assert_eq!(refusals[0].data.placement.0, "linux");
    assert!(BoxStorage::list(&store).is_empty(), "no box opened");
    assert!(fake.log.lock().expect("the log").opened.is_empty());
}

// ---------------------------------------------------------------------------------------------
// conductord
// ---------------------------------------------------------------------------------------------

#[test]
fn conductord_starts_a_linux_instance_in_its_box_and_closes_it_when_gone() {
    let case = Case::new("daemon-box");
    case.config(&[("alpha", "{daemon: local, adapter: linux}")]);
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].action, ReconcileAction::Start);
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    assert!(
        daemon
            .units
            .calls
            .iter()
            .all(|call| !call.starts_with("start")),
        "the service manager started nothing: {:?}",
        daemon.units.calls
    );
    {
        let log = fake.log.lock().expect("the log");
        assert_eq!(log.opened.len(), 1);
        assert_eq!(log.opened[0].argv[3..], ["instance", "serve"]);
        assert!(
            log.opened[0]
                .env
                .iter()
                .all(|(name, _)| name != conductor_cli::instance::TOKEN_VARIABLE),
            "no credential is handed into the box in its environment: {:?}",
            log.opened[0].env
        );
    }
    assert_eq!(BoxStorage::list(daemon.store()).len(), 1);

    // A second pass finds the runtime running in its box and does nothing.
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::Cadence,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert!(done.is_empty(), "{done:?}");

    // alpha is gone from the config; beta, on conductor's own units, is not boxed.
    case.config(&[("beta", "systemd")]);
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert!(
        done.iter()
            .any(|done| done.instance == "alpha" && done.action == ReconcileAction::Stop),
        "{done:?}"
    );
    let boxes = BoxStorage::list(daemon.store());
    assert_eq!(boxes[0].state, BoxState::Closed, "{boxes:?}");
    let log = fake.log.lock().expect("the log");
    assert_eq!(log.closed, ["ws_1".to_owned()]);
}

#[test]
fn conductord_records_a_refused_box_and_starts_nothing_else() {
    let case = Case::new("daemon-refused");
    case.config(&[("alpha", "linux")]);
    let mut daemon = case.daemon(Fake::refusing("exec.sandbox-unavailable"));
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].result, ReconcileResult::Refused);
    let detail = done[0].detail.clone().unwrap_or_default();
    assert!(
        detail.contains("PlacementRefused") && detail.contains("exec.sandbox-unavailable"),
        "{detail}"
    );
    assert!(
        daemon
            .units
            .calls
            .iter()
            .all(|call| !call.starts_with("start"))
    );
    assert_eq!(BoxRefusalStorage::list(daemon.store()).len(), 1);
    assert_eq!(ReconciliationStorage::list(daemon.store()).len(), 1);
}

// ---------------------------------------------------------------------------------------------
// P6: doctor
// ---------------------------------------------------------------------------------------------

impl Case {
    fn program(&self, name: &str, line: &str) {
        let file = self.dir.join("bin").join(name);
        fs::write(&file, format!("#!/bin/sh\nprintf '%s\\n' '{line}'\n")).expect("write");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("runnable");
    }

    fn doctor(&self, file: &Path) -> Output {
        common::conductor(self.home())
            .arg("doctor")
            .current_dir(self.home().join("work"))
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.dir.join("bin"))
            .env("CONDUCTOR_CONFIG", file)
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }
}

fn lines(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn doctor_checks_substrate_daemon_against_its_pin_for_a_substrate_placement() {
    let case = Case::new("doctor-substrate");
    let file = case.config(&[("alpha", "linux")]);
    let output = case.doctor(&file);
    assert!(
        lines(&output).contains(&"substrate-daemon: missing".to_owned()),
        "{}",
        describe(&output)
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));

    case.program("substrate-daemon", "substrate-daemon 0.16.0");
    fs::write(
        case.home().join("work/b10x.toml"),
        "[pins]\nsubstrate-daemon = \"0.16.1\"\n",
    )
    .expect("write the pins");
    let output = case.doctor(&file);
    assert!(
        lines(&output)
            .contains(&"substrate-daemon: 0.16.0 is older than its pin 0.16.1".to_owned()),
        "{}",
        describe(&output)
    );

    let file = case.config(&[("alpha", "systemd")]);
    let output = case.doctor(&file);
    assert!(
        lines(&output).contains(&"substrate-daemon: not needed".to_owned()),
        "{}",
        describe(&output)
    );
}

#[test]
fn the_build_carries_the_substrate_pin_of_the_repository() {
    assert!(
        conductor_cli::doctor::build_pins()
            .iter()
            .any(|(program, spec)| program == "substrate-daemon" && spec == "0.16.1"),
        "{:?}",
        conductor_cli::doctor::build_pins()
    );
}
