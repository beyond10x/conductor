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
//!
//! `story:placement-substrate-remote`: a placement names a remote substrate daemon by URL.
//!
//! - `{daemon: https://…, adapter, bearer: secret:<name>, trust_roots, server_identity}` reads as
//!   the instance's remote daemon; a missing bearer or trust roots, and the three keys on a
//!   local daemon, are refused by path.
//! - conductord opens and closes a remote instance's box on that daemon; one that does not
//!   answer makes the instance `Unreachable`, never fails a pass and starts nothing anew.
//! - the SDK's remote transport asks a fake remote daemon on loopback over TLS with the bearer
//!   the secrets source answered ([`FakeRemote`]); a silent one is unreachable within the bound.
//!
//! Every case runs against a fake substrate ([`Fake`]) or [`FakeRemote`]; no real daemon,
//! bubblewrap or unit runs.

mod active;
mod common;

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::units::Units;
use conductor_cli::placement::{
    self, Adapter, BoxRequest, Opened, Refused, Resolved, Substrate, Where,
};
use conductor_cli::systemd::Service;
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
    /// The daemon answers nothing: every question is an error (a silent remote).
    silent: bool,
    /// The remote daemons the box opener was built for, by URL.
    remotes: Vec<String>,
    /// The relay sessions it was asked to run.
    exchanges: Vec<placement::Exchange>,
    /// Each lease renewal, `(workspace, session)`.
    renewed: Vec<(String, String)>,
    /// Each runtime restarted in its box, `(workspace, request)`.
    restarted: Vec<(String, BoxRequest)>,
    /// For each box asked for, the mode of each writable root's directory at that moment;
    /// `None` for one that was not there.
    roots_at_open: Vec<Vec<Option<u32>>>,
    /// For each box asked for, the mode of the sessions directory under its state directory at
    /// that moment; `None` when it was not there.
    sessions_at_open: Vec<Option<u32>>,
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
        let roots = request
            .writable_roots
            .iter()
            .map(|root| {
                fs::metadata(&root.path)
                    .ok()
                    .filter(fs::Metadata::is_dir)
                    .map(|meta| meta.permissions().mode() & 0o777)
            })
            .collect();
        log.roots_at_open.push(roots);
        log.sessions_at_open.push(
            fs::metadata(Path::new(&request.cwd).join("sessions"))
                .ok()
                .filter(fs::Metadata::is_dir)
                .map(|meta| meta.permissions().mode() & 0o777),
        );
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
        let log = self.log.lock().expect("the log");
        if log.silent {
            anyhow::bail!("placement.substrate-unreachable: the daemon did not answer");
        }
        Ok(log.running.contains(session))
    }

    fn stop(&mut self, session: &str) -> Result<()> {
        let mut log = self.log.lock().expect("the log");
        if log.silent {
            anyhow::bail!("placement.substrate-unreachable: the daemon did not answer");
        }
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

    fn renew(&mut self, workspace: &str, session: &str, _ttl: std::time::Duration) -> Result<()> {
        self.log
            .lock()
            .expect("the log")
            .renewed
            .push((workspace.to_owned(), session.to_owned()));
        Ok(())
    }

    fn restart(&mut self, workspace: &str, request: &BoxRequest) -> Result<String, Refused> {
        let mut log = self.log.lock().expect("the log");
        log.restarted.push((workspace.to_owned(), request.clone()));
        let session = format!("ses_r{}", log.restarted.len());
        log.running.insert(session.clone());
        Ok(session)
    }

    /// A pipe session in the box: runs its program here, this build's `conductor` in place of
    /// the box's, with its arguments and environment, and answers its standard output.
    fn exchange(
        &mut self,
        exchange: &placement::Exchange,
        input: &[u8],
    ) -> Result<Vec<u8>, Refused> {
        use std::io::Write as _;
        self.log
            .lock()
            .expect("the log")
            .exchanges
            .push(exchange.clone());
        let home = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("placement-relay-home");
        fs::create_dir_all(&home).expect("the relay's home");
        let mut child = common::conductor(&home)
            .args(&exchange.argv[1..])
            .envs(exchange.env.iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the relay runs");
        child
            .stdin
            .take()
            .expect("its input")
            .write_all(input)
            .expect("write its input");
        let output = child.wait_with_output().expect("the relay ends");
        if !output.status.success() {
            return Err(Refused {
                name: "relay-failed".to_owned(),
                detail: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(output.stdout)
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
        // The store in a directory of its own: a box writes the whole directory of a store.
        for sub in ["home/work", "bin", "store"] {
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
        format!("sqlite:{}", self.dir.join("store/shared.sqlite").display())
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

/// The remote daemon `config show` prints for the case's only instance, null for a local one.
fn shown_remote(case: &Case, file: &Path) -> Value {
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
    shown["instances"][0]["remote"].clone()
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

    // alpha is gone from the config; beta, a systemd box, opens a box of its own.
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

    let file = case.config(&[("alpha", "box-a")]);
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
            .any(|(program, spec)| program == "substrate-daemon" && spec == "0.20.0"),
        "{:?}",
        conductor_cli::doctor::build_pins()
    );
}

// ---------------------------------------------------------------------------------------------
// story:placement-substrate-remote: a placement names a remote substrate daemon by URL
// ---------------------------------------------------------------------------------------------

/// The remote daemon every remote case names. No request reaches it: the cases run against
/// [`Fake`] or against a fake daemon on loopback ([`FakeRemote`]).
const REMOTE: &str = "https://substrate.example-org.example:8443/";

/// A bearer in the form substrate's SDK takes: `identity_access_v1_` and 43 characters.
fn bearer_value() -> String {
    format!("identity_access_v1_{}", "b".repeat(43))
}

impl Case {
    /// A remote placement of `url` with the bearer `secret:substrate-bearer` and the case's trust
    /// roots, as a YAML flow value.
    fn remote_placement(&self, url: &str) -> String {
        let roots = self.dir.join("roots.pem");
        if !roots.exists() {
            fs::write(&roots, "").expect("write the trust roots");
        }
        format!(
            "{{daemon: \"{url}\", adapter: linux, bearer: \"secret:substrate-bearer\", \
             trust_roots: {roots:?}}}"
        )
    }

    /// The daemon of [`Case::daemon`] whose remote boxes open on `remote`, a fake.
    fn remote_daemon(&self, local: Fake, remote: Fake) -> Daemon<Units0> {
        let mut daemon = self.daemon(local);
        daemon.remote = Arc::new(move |at: &placement::RemoteAt, _bearer| {
            remote
                .log
                .lock()
                .expect("the log")
                .remotes
                .push(at.url.clone());
            Box::new(remote.clone()) as Box<dyn Substrate + Send>
        });
        daemon
    }
}

#[test]
fn a_remote_placement_reads_its_url_bearer_and_trust_roots() {
    let case = Case::new("remote-reads");
    let file = case.config(&[("alpha", &case.remote_placement(REMOTE))]);
    let output = case.conductor(&[
        "--config",
        &file.display().to_string(),
        "config",
        "validate",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(shown_placement(&case, &file), format!("linux@{REMOTE}"));
    // `config show` prints the daemon with its bearer by name only, never a secret's value.
    assert_eq!(
        shown_remote(&case, &file),
        serde_json::json!({
            "url": REMOTE,
            "bearer": "secret:substrate-bearer",
            "trust_roots": case.dir.join("roots.pem").display().to_string(),
            "server_identity": "substrate.example-org.example",
        })
    );

    let config =
        conductor_cli::config::parse(&fs::read_to_string(&file).expect("the file"), &case.home())
            .expect("the file loads");
    let daemon = config.instances[0]
        .remote
        .clone()
        .expect("a remote placement carries its daemon");
    assert_eq!(daemon.url, REMOTE);
    assert_eq!(daemon.bearer.0, "substrate-bearer");
    assert_eq!(
        daemon.trust_roots,
        case.dir.join("roots.pem").display().to_string()
    );
    // The server identity defaults to the URL's host.
    assert_eq!(daemon.server_identity, "substrate.example-org.example");

    let file = case.config(&[("alpha", "{daemon: local, adapter: linux}")]);
    let config =
        conductor_cli::config::parse(&fs::read_to_string(&file).expect("the file"), &case.home())
            .expect("the file loads");
    assert_eq!(config.instances[0].remote, None);
    assert_eq!(shown_remote(&case, &file), Value::Null);
}

/// `remote` is derived from the placement: a file may write it null or as `config show` writes
/// it, so that the shown YAML reads back, and any other value is refused by its path.
#[test]
fn a_written_remote_is_held_to_the_placement_s_daemon() {
    let case = Case::new("remote-derived");
    let remote = case.remote_placement(REMOTE);
    let file = case.config(&[("alpha", &remote)]);
    let shown = shown_remote(&case, &file);
    let validate = |file: &Path| {
        case.conductor(&[
            "--config",
            &file.display().to_string(),
            "config",
            "validate",
        ])
    };
    for (placement, written, code) in [
        (remote.as_str(), shown.to_string(), 0),
        (remote.as_str(), "null".to_owned(), 0),
        ("{daemon: local, adapter: linux}", "null".to_owned(), 0),
        ("{daemon: local, adapter: linux}", shown.to_string(), 1),
    ] {
        let file = case.config(&[("alpha", &format!("{placement}\n    remote: {written}"))]);
        let output = validate(&file);
        assert_eq!(
            output.status.code(),
            Some(code),
            "{placement} with remote {written}: {}",
            describe(&output)
        );
        if code == 1 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr
                    .lines()
                    .any(|line| line.contains("instances[0].remote") && line.contains("derived")),
                "{}",
                describe(&output)
            );
        }
    }
}

#[test]
fn a_shown_remote_placement_loads_back_as_the_same_remote_daemon() {
    let case = Case::new("remote-roundtrip");
    let file = case.config(&[("alpha", &case.remote_placement(REMOTE))]);
    let show = |file: &Path, format: &str| {
        let output = case.conductor(&[
            "--config",
            &file.display().to_string(),
            "config",
            "show",
            "--format",
            format,
        ]);
        assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
        output.stdout
    };
    let json = |bytes: &[u8]| -> Value { serde_json::from_slice(bytes).expect("show prints JSON") };
    let first = json(&show(&file, "json"));
    assert!(first["instances"][0]["remote"].is_object(), "{first}");
    let printed = case.dir.join("shown.yaml");
    fs::write(&printed, show(&file, "yaml")).expect("write the shown document");
    let validated = case.conductor(&[
        "--config",
        &printed.display().to_string(),
        "config",
        "validate",
    ]);
    assert_eq!(validated.status.code(), Some(0), "{}", describe(&validated));
    assert_eq!(json(&show(&printed, "json")), first);
    assert_eq!(show(&printed, "yaml"), show(&file, "yaml"));
}

#[test]
fn a_remote_placement_without_its_bearer_or_roots_is_refused_by_path() {
    let case = Case::new("remote-refusals");
    let roots = case.dir.join("roots.pem");
    fs::write(&roots, "").expect("write the trust roots");
    for (placement, path, word) in [
        (
            format!("{{daemon: \"{REMOTE}\", adapter: linux, trust_roots: {roots:?}}}"),
            "instances[0].placement.bearer",
            "missing",
        ),
        (
            format!("{{daemon: \"{REMOTE}\", adapter: linux, bearer: \"secret:b\"}}"),
            "instances[0].placement.trust_roots",
            "missing",
        ),
        (
            format!(
                "{{daemon: \"{REMOTE}\", adapter: linux, bearer: plain, trust_roots: {roots:?}}}"
            ),
            "instances[0].placement.bearer",
            "secret:",
        ),
        (
            "{daemon: local, adapter: linux, bearer: \"secret:b\"}".to_owned(),
            "instances[0].placement.bearer",
            "remote",
        ),
    ] {
        let file = case.config(&[("alpha", &placement)]);
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
        assert!(
            !stderr.contains("plain") || stderr.contains("placement.bearer"),
            "{}",
            describe(&output)
        );
    }
}

#[test]
fn conductord_opens_a_remote_box_on_the_remote_daemon_and_closes_it_there() {
    let case = Case::new("remote-box");
    case.config(&[("alpha", &case.remote_placement(REMOTE))]);
    let local = Fake::default();
    let remote = Fake::default();
    let mut daemon = case.remote_daemon(local.clone(), remote.clone());
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    assert!(local.log.lock().expect("the log").opened.is_empty());
    {
        let log = remote.log.lock().expect("the log");
        assert_eq!(log.opened.len(), 1);
        assert_eq!(log.opened[0].placement.0, format!("linux@{REMOTE}"));
        assert!(
            log.remotes.iter().all(|url| url == REMOTE),
            "{:?}",
            log.remotes
        );
    }
    assert_eq!(BoxStorage::list(daemon.store()).len(), 1);
    let rows = daemon.rows();
    assert_eq!(rows[0]["state"], "Running", "{rows:?}");
    // No socket here reaches a runtime on a remote daemon: its route is a relay session there.
    let routes = daemon.routes();
    assert!(routes["alpha"].remote.is_some(), "{:?}", routes["alpha"]);

    // Gone from the config: the box is closed on the remote daemon, which the daemon kept.
    case.config(&[("beta", "systemd")]);
    let mut daemon = case.remote_daemon(local.clone(), remote.clone());
    daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert_eq!(
        remote.log.lock().expect("the log").closed,
        ["ws_1".to_owned()]
    );
    assert!(local.log.lock().expect("the log").closed.is_empty());
}

#[test]
fn a_silent_remote_makes_the_instance_unreachable_and_never_fails_a_pass() {
    let case = Case::new("remote-silent");
    case.config(&[("alpha", &case.remote_placement(REMOTE))]);
    let remote = Fake::default();
    let mut daemon = case.remote_daemon(Fake::default(), remote.clone());
    daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    remote.log.lock().expect("the log").silent = true;

    for reason in [
        conductor_model::instance::ReconcileReason::Cadence,
        conductor_model::instance::ReconcileReason::Cadence,
    ] {
        let done = daemon
            .pass(reason, std::time::Instant::now())
            .expect("a silent remote fails no pass");
        assert!(
            done.is_empty(),
            "a silent remote is not a lost runtime: no new box: {done:?}"
        );
    }
    let rows = daemon.rows();
    assert_eq!(rows[0]["state"], "Unreachable", "{rows:?}");
    assert!(rows[0]["last_seen"].is_string(), "{rows:?}");
    assert_eq!(remote.log.lock().expect("the log").opened.len(), 1);
    assert!(
        !daemon.routes().contains_key("alpha"),
        "the API proxies to no runtime it cannot reach"
    );

    // It answers again: Running, the same box.
    remote.log.lock().expect("the log").silent = false;
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::Cadence,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert!(done.is_empty(), "{done:?}");
    assert_eq!(daemon.rows()[0]["state"], "Running");
}

// ---------------------------------------------------------------------------------------------
// The SDK's remote transport against a fake remote daemon
// ---------------------------------------------------------------------------------------------

/// A fake remote substrate daemon on loopback: TLS 1.3 with a certificate for `name`; it keeps
/// the head of every request and answers each `503`, or, `silent`, accepts the connection and
/// never answers.
struct FakeRemote {
    port: u16,
    roots: PathBuf,
    heads: Arc<Mutex<Vec<String>>>,
}

impl FakeRemote {
    fn start(dir: &Path, name: &str, silent: bool) -> Self {
        use std::io::{Read as _, Write as _};
        let generated =
            rcgen::generate_simple_self_signed(vec![name.to_owned()]).expect("a certificate");
        let roots = dir.join(format!("{name}-roots.pem"));
        fs::write(&roots, generated.cert.pem()).expect("write the trust roots");
        let certificate = rustls::pki_types::CertificateDer::from(generated.cert.der().to_vec());
        let key = rustls::pki_types::PrivateKeyDer::try_from(generated.key_pair.serialize_der())
            .expect("a private key");
        let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("TLS 1.3")
        .with_no_client_auth()
        .with_single_cert(vec![certificate], key)
        .expect("the server's certificate");
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let config = Arc::new(config);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a listener");
        let port = listener.local_addr().expect("its address").port();
        let heads = Arc::new(Mutex::new(Vec::new()));
        let kept = heads.clone();
        std::thread::spawn(move || {
            let mut held = Vec::new();
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                if silent {
                    held.push(stream);
                    continue;
                }
                let Ok(connection) = rustls::ServerConnection::new(config.clone()) else {
                    continue;
                };
                let mut tls = rustls::StreamOwned::new(connection, stream);
                let mut head = Vec::new();
                let mut byte = [0_u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    match tls.read(&mut byte) {
                        Ok(1) => head.push(byte[0]),
                        _ => break,
                    }
                }
                kept.lock()
                    .expect("the heads")
                    .push(String::from_utf8_lossy(&head).into_owned());
                let _ = tls.write_all(
                    b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\n\
                      connection: close\r\n\r\n",
                );
                let _ = tls.flush();
                tls.conn.send_close_notify();
                let _ = tls.flush();
            }
        });
        Self { port, roots, heads }
    }

    fn at(&self, name: &str) -> placement::RemoteAt {
        placement::RemoteAt {
            url: format!("https://127.0.0.1:{}/", self.port),
            bearer: "substrate-bearer".to_owned(),
            trust_roots: self.roots.display().to_string(),
            server_identity: name.to_owned(),
        }
    }
}

/// The bearer source of a case: the value [`bearer_value`], as the secrets source would answer.
fn bearer() -> placement::Bearer {
    Arc::new(|| secrets_core::storage::SecretValue::new(bearer_value().into_bytes()).ok())
}

#[test]
fn the_sdk_asks_a_remote_daemon_over_tls_with_the_bearer_it_resolved() {
    let case = Case::new("sdk-remote");
    let name = "substrate.example-org.example";
    let fake = FakeRemote::start(&case.dir, name, false);
    let mut sdk = placement::Sdk::remote(fake.at(name), bearer());
    let request = placement::request(
        &linux_instance(&case),
        &case.file(),
        Path::new("/opt/conductor/bin/conductor"),
        Vec::new(),
    );
    let refused = sdk
        .open(&request)
        .expect_err("a daemon that answers 503 opens no box");
    let heads = fake.heads.lock().expect("the heads").clone();
    assert!(
        !heads.is_empty(),
        "the request reached the fake daemon: {refused:?}"
    );
    assert!(heads[0].starts_with("GET /v1/machine "), "{heads:?}");
    assert!(
        heads[0]
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {}", bearer_value())),
        "the bearer travels as the request's Authorization header: {heads:?}"
    );
    assert!(
        !format!("{refused:?}").contains(&bearer_value()),
        "the refusal never names the bearer: {refused:?}"
    );
}

#[test]
fn a_remote_daemon_that_never_answers_is_unreachable_within_the_bound() {
    let case = Case::new("sdk-silent");
    let name = "substrate.example-org.example";
    let fake = FakeRemote::start(&case.dir, name, true);
    let mut sdk = placement::Sdk::remote(fake.at(name), bearer());
    sdk.timeout = std::time::Duration::from_millis(500);
    let began = std::time::Instant::now();
    let refused = sdk
        .open(&placement::request(
            &linux_instance(&case),
            &case.file(),
            Path::new("/opt/conductor/bin/conductor"),
            Vec::new(),
        ))
        .expect_err("a silent daemon opens no box");
    assert_eq!(refused.name, placement::UNREACHABLE, "{refused:?}");
    assert!(began.elapsed() < std::time::Duration::from_secs(5));
    let running = sdk.running("ses_1");
    assert!(
        running.is_err(),
        "a silent daemon is asked nothing: {running:?}"
    );
}

#[test]
fn a_bearer_the_secrets_source_does_not_hold_is_refused_by_name() {
    let case = Case::new("sdk-no-bearer");
    let name = "substrate.example-org.example";
    let fake = FakeRemote::start(&case.dir, name, false);
    let mut sdk = placement::Sdk::remote(fake.at(name), Arc::new(|| None));
    let refused = sdk
        .open(&placement::request(
            &linux_instance(&case),
            &case.file(),
            Path::new("/opt/conductor/bin/conductor"),
            Vec::new(),
        ))
        .expect_err("no bearer, no box");
    assert_eq!(refused.name, placement::BEARER_UNAVAILABLE, "{refused:?}");
}

// ---------------------------------------------------------------------------------------------
// The relay: the instance API of a runtime in a remote box, through a substrate pipe session
// ---------------------------------------------------------------------------------------------

/// The answer [`FakeInstance`] gives every request.
const ANSWER: &[u8] =
    b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 34\r\n\
connection: close\r\n\r\n{\"rows\":[{\"session\":\"alpha-lead\"}]}";

/// An instance API on `socket`: it keeps each request, read to its end, and answers [`ANSWER`].
struct FakeInstance {
    requests: Arc<Mutex<Vec<String>>>,
}

impl FakeInstance {
    fn start(socket: &Path) -> Self {
        use std::io::{Read as _, Write as _};
        if let Some(parent) = socket.parent() {
            fs::create_dir_all(parent).expect("the socket's directory");
        }
        let _ = fs::remove_file(socket);
        let listener = std::os::unix::net::UnixListener::bind(socket).expect("bind the socket");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let kept = requests.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let mut request = Vec::new();
                let _ = stream.read_to_end(&mut request);
                kept.lock()
                    .expect("the requests")
                    .push(String::from_utf8_lossy(&request).into_owned());
                let _ = stream.write_all(ANSWER);
            }
        });
        Self { requests }
    }
}

#[test]
fn instance_relay_bridges_one_exchange_to_the_instance_socket() {
    use std::io::Write as _;
    let case = Case::new("relay-cli");
    let socket = conductor_cli::instance::socket(&case.dir);
    let instance = FakeInstance::start(&socket);
    let request = b"GET /v1/sessions HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let mut child = common::conductor(case.home())
        .args(["instance", "relay"])
        .env(conductor_cli::instance::SOCKET_VARIABLE, &socket)
        .current_dir(case.home().join("work"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("conductor runs");
    child
        .stdin
        .take()
        .expect("its input")
        .write_all(request)
        .expect("write the request");
    let output = child.wait_with_output().expect("the relay ends");
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(output.stdout, ANSWER, "{}", describe(&output));
    let requests = instance.requests.lock().expect("the requests").clone();
    assert_eq!(requests, [String::from_utf8_lossy(request).into_owned()]);

    // No instance answers: one line naming the socket, exit 1.
    let output = common::conductor(case.home())
        .args(["instance", "relay"])
        .env(
            conductor_cli::instance::SOCKET_VARIABLE,
            case.dir.join("none.sock"),
        )
        .current_dir(case.home().join("work"))
        .stdin(Stdio::null())
        .output()
        .expect("conductor runs");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("none.sock"),
        "{}",
        describe(&output)
    );
}

#[test]
fn conductord_reaches_a_remote_runtime_through_a_relay_session_in_its_box() {
    let case = Case::new("relay-remote");
    case.config(&[("alpha", &case.remote_placement(REMOTE))]);
    let remote = Fake::default();
    let mut daemon = case.remote_daemon(Fake::default(), remote.clone());
    daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    let route = daemon.routes()["alpha"].clone();
    // The runtime's socket in its box; the fake daemon runs the relay on this host.
    let socket = conductor_cli::instance::socket(&case.dir.join("alpha").join("state"));
    let instance = FakeInstance::start(&socket);

    // A proxied request goes through one relay session and comes back byte for byte.
    let request = b"GET /v1/decisions HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let answer = conductor_cli::daemon::api::send(&route, request)
        .expect("the remote runtime answers through the relay");
    assert_eq!(answer, ANSWER);

    // A federated read takes the same way.
    let rows = conductor_cli::daemon::federate::fetch(
        &route,
        "/v1/sessions",
        std::time::Duration::from_secs(30),
    )
    .expect("the federated read answers");
    assert_eq!(rows["rows"][0]["session"], "alpha-lead", "{rows}");

    let requests = instance.requests.lock().expect("the requests").clone();
    assert_eq!(requests.len(), 2, "{requests:?}");
    assert!(
        requests[0].starts_with("GET /v1/decisions "),
        "{requests:?}"
    );
    let log = remote.log.lock().expect("the log");
    assert_eq!(log.exchanges.len(), 2);
    let exchange = &log.exchanges[0];
    assert_eq!(exchange.workspace, "ws_1");
    assert_eq!(exchange.argv[0], "/opt/conductor/bin/conductor");
    assert_eq!(exchange.argv[3..], ["instance", "relay"]);
    assert!(
        exchange.env.iter().any(|(name, value)| {
            name == conductor_cli::instance::SOCKET_VARIABLE && Path::new(value) == socket.as_path()
        }),
        "{:?}",
        exchange.env
    );
    assert!(
        exchange
            .env
            .iter()
            .all(|(name, _)| name != conductor_cli::instance::TOKEN_VARIABLE),
        "no credential in the relay's environment: {:?}",
        exchange.env
    );
}

// ---------------------------------------------------------------------------------------------
// substrate 0.20.0: an adapter is the facts it requires; the model-token slot pool; the run bound
// ---------------------------------------------------------------------------------------------

use substrate_sdk::{ConfinementFamily as Family, ConfinementProperty as Property};

/// `linux`'s expected set of substrate's design 30 § 4.3, with the aperture conductor declares.
const LINUX_FAMILIES: [Family; 8] = [
    Family::ProcessRun,
    Family::ProcessWatch,
    Family::OutputRead,
    Family::Wait,
    Family::Workspace,
    Family::FilesystemView,
    Family::NetworkEgress,
    Family::Audit,
];

#[test]
fn an_adapter_is_the_confinement_facts_it_requires_never_its_name() {
    let linux = placement::requires(Adapter::Linux);
    assert_eq!(linux.families, LINUX_FAMILIES);
    assert_eq!(
        linux.facts,
        [Property::IpcSocketAllowlist, Property::IpcSignalsContained]
    );
    // systemd is linux inside a delegated scope: linux's set plus limits and whole-tree kill.
    let systemd = placement::requires(Adapter::Systemd);
    for family in &linux.families {
        assert!(systemd.families.contains(family), "{systemd:?}");
    }
    assert!(systemd.families.contains(&Family::Limits), "{systemd:?}");
    assert!(
        systemd.facts.contains(&Property::WholeTreeKill),
        "{systemd:?}"
    );

    // The first unmet requirement is refused by substrate's own name, families first.
    let served: Vec<Family> = LINUX_FAMILIES.to_vec();
    let proved = [Property::IpcSocketAllowlist, Property::IpcSignalsContained];
    assert_eq!(placement::unmet(&linux, &served, &proved), None);
    let refused = placement::unmet(&systemd, &served, &proved).expect("limits is not served");
    assert_eq!(refused.name, "confinement.family-unserved");
    assert!(refused.detail.contains("limits"), "{refused:?}");
    let mut served_limits = served.clone();
    served_limits.push(Family::Limits);
    let refused =
        placement::unmet(&systemd, &served_limits, &proved).expect("whole-tree-kill is not proved");
    assert_eq!(refused.name, "confinement.fact-unserved");
    assert!(refused.detail.contains("whole-tree-kill"), "{refused:?}");
}

/// The names of the writable roots the box of [`linux_instance`] asks for.
fn root_names(case: &Case) -> Vec<String> {
    placement::request(
        &linux_instance(case),
        &case.file(),
        Path::new("/opt/conductor/bin/conductor"),
        Vec::new(),
    )
    .writable_roots
    .into_iter()
    .map(|root| root.name)
    .collect()
}

#[test]
fn a_box_request_carries_its_requirement_and_its_named_writable_roots() {
    let case = Case::new("box-requires");
    let mut instance = linux_instance(&case);
    instance.placement = PlacementName("systemd".to_owned());
    instance.state = case.dir.join("i/state").display().to_string();
    instance.records = case.dir.join("i/records").display().to_string();
    let request = placement::request(
        &instance,
        &case.file(),
        Path::new("/opt/conductor/bin/conductor"),
        Vec::new(),
    );
    assert_eq!(request.requires, placement::requires(Adapter::Systemd));
    assert_eq!(
        request.writable_roots,
        [
            placement::HostRoot {
                name: format!("state-{}", instance.name.0),
                path: instance.state.clone(),
            },
            placement::HostRoot {
                name: format!("records-{}", instance.name.0),
                path: instance.records.clone(),
            },
            placement::HostRoot {
                name: format!("cache-{}", instance.name.0),
                path: instance.cache.clone(),
            },
        ]
    );
}

#[test]
fn a_writable_root_name_is_in_substrates_root_name_set() {
    for (instance, name) in [
        ("alpha", "state-alpha"),
        ("example-org_2", "state-example-org_2"),
        ("a.b c/d", "state-a_b_c_d"),
    ] {
        assert_eq!(placement::writable_root_name("state", instance), name);
    }
    assert_eq!(
        placement::writable_root_name("records", "a.b"),
        "records-a_b"
    );
}

/// A local substrate daemon on a Unix socket: it answers `GET /v1/machine` with `facts` beside
/// the vector's other facts, keeps the request line of every request, and answers every other
/// one 503; [`FakeLocal::opening`] also creates every workspace asked for and keeps the body of
/// every session start. It publishes `roots` as the writable host roots it serves
/// (`exec.writable-roots.host`), and the fact not at all when `roots` is empty.
struct FakeLocal {
    lines: Arc<Mutex<Vec<String>>>,
    starts: Arc<Mutex<Vec<Value>>>,
}

impl FakeLocal {
    fn start(socket: &Path, families: &[Family], properties: &[Property], roots: &[&str]) -> Self {
        Self::serve(socket, families, properties, roots, false)
    }

    fn opening(
        socket: &Path,
        families: &[Family],
        properties: &[Property],
        roots: &[&str],
    ) -> Self {
        Self::serve(socket, families, properties, roots, true)
    }

    fn serve(
        socket: &Path,
        families: &[Family],
        properties: &[Property],
        roots: &[&str],
        opens: bool,
    ) -> Self {
        use std::io::{Read as _, Write as _};
        let mut facts = serde_json::json!({
            "events.pull": true,
            "exec.argv-only": true,
            "exec.cgroup-kill": true,
            "exec.no-egress": true,
            "exec.output-limit-bytes": 65536,
            "leases.explicit": true,
            "operation.ledger-global-max-bytes": 4_294_967_296_u64,
            "operation.ledger-global-max-rows": 1_000_000,
            "operation.ledger-subject-max-bytes": 536_870_912,
            "operation.ledger-subject-max-rows": 100_000,
            "workspace.guarded-io": true,
        });
        for family in families {
            facts[family.fact()] = Value::Bool(true);
        }
        for property in properties {
            facts[property.fact()] = Value::Bool(true);
        }
        if !roots.is_empty() {
            facts["exec.writable-roots.host"] = serde_json::json!(roots);
        }
        let body = serde_json::to_vec(&serde_json::json!({
            "api_version": "v1",
            "request_id": "req_machine",
            "result": {
                "config_generation": 1,
                "driver": "host",
                "driver_version": "fixture-1",
                "facts": facts,
                "probed_at": "2026-11-08T12:00:00Z",
                "snapshot": format!("sha256:{}", "7".repeat(64)),
            }
        }))
        .expect("the machine");
        let _ = fs::remove_file(socket);
        let listener = std::os::unix::net::UnixListener::bind(socket).expect("bind the socket");
        let lines = Arc::new(Mutex::new(Vec::new()));
        let starts = Arc::new(Mutex::new(Vec::new()));
        let kept = lines.clone();
        let started = starts.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let mut head = Vec::new();
                let mut byte = [0_u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    match stream.read(&mut byte) {
                        Ok(1) => head.push(byte[0]),
                        _ => break,
                    }
                }
                let head = String::from_utf8_lossy(&head).into_owned();
                let line = head.lines().next().unwrap_or_default().to_owned();
                kept.lock().expect("the lines").push(line.clone());
                let length = head
                    .lines()
                    .find_map(|header| {
                        let (name, value) = header.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                let mut sent = vec![0_u8; length];
                let _ = stream.read_exact(&mut sent);
                let sent: Value = serde_json::from_slice(&sent).unwrap_or(Value::Null);
                let (status, answer) = if line.starts_with("GET /v1/machine ") {
                    ("200 OK", body.clone())
                } else if opens && line.starts_with("POST /v1/workspaces ") {
                    let workspace = serde_json::json!({
                        "api_version": "v1",
                        "request_id": "req_workspace",
                        "operation": sent["op"],
                        "result": {
                            "id": "ws_fake",
                            "kind": "workspace",
                            "labels": {},
                            "observed_at": "2026-11-08T12:00:00Z",
                            "state": "ready",
                            "origin": "created",
                            "root_name": "ws_fake"
                        }
                    });
                    (
                        "201 Created",
                        serde_json::to_vec(&workspace).expect("the workspace"),
                    )
                } else {
                    if opens && line.starts_with("POST /v1/sessions ") {
                        started.lock().expect("the starts").push(sent);
                    }
                    ("503 Service Unavailable", Vec::new())
                };
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 {status}\r\nx-b10x-contract: {}\r\n\
                         x-b10x-contract-bundle-sha256: {}\r\ncontent-type: application/json\r\n\
                         content-length: {}\r\nconnection: close\r\n\r\n",
                        substrate_sdk::CONTRACT,
                        substrate_sdk::CONTRACT_SHA256,
                        answer.len()
                    )
                    .as_bytes(),
                );
                let _ = stream.write_all(&answer);
            }
        });
        Self { lines, starts }
    }
}

#[test]
fn the_sdk_starts_the_runtime_with_each_writable_root_by_name_in_order() {
    let case = Case::new("roots");
    let socket = case.dir.join("s.sock");
    let mut served = root_names(&case);
    let own = served.len();
    served.push("second".to_owned());
    let served: Vec<&str> = served.iter().map(String::as_str).collect();
    let fake = FakeLocal::opening(
        &socket,
        &LINUX_FAMILIES,
        &[Property::IpcSocketAllowlist, Property::IpcSignalsContained],
        &served,
    );
    let mut sdk = placement::Sdk::default();
    sdk.socket = socket.clone();
    let mut request = placement::request(
        &linux_instance(&case),
        &case.file(),
        Path::new("/opt/conductor/bin/conductor"),
        Vec::new(),
    );
    request.writable_roots.push(placement::HostRoot {
        name: "second".to_owned(),
        path: "/srv/second".to_owned(),
    });
    let expected: Vec<Value> = request
        .writable_roots
        .iter()
        .map(|root| serde_json::json!({"host": root.name, "mount": root.path}))
        .collect();
    assert!(own >= 1, "the box writes its state directory");
    assert_eq!(expected.len(), own + 1);
    let refused = sdk
        .open(&request)
        .expect_err("the fake answers the session start 503");
    let starts = fake.starts.lock().expect("the starts").clone();
    let lines = fake.lines.lock().expect("the lines").clone();
    assert_eq!(
        starts.len(),
        1,
        "one session start: {starts:?} {refused:?} {lines:?}"
    );
    assert_eq!(
        starts[0].pointer("/input/exec/writable_roots"),
        Some(&Value::Array(expected)),
        "the start names each root, in order, at its own path: {}",
        starts[0]
    );
}

#[test]
fn the_sdk_refuses_a_box_whose_facts_the_local_daemon_does_not_publish_before_anything_starts() {
    let case = Case::new("facts");
    let socket = case.dir.join("s.sock");
    let served = root_names(&case);
    let served: Vec<&str> = served.iter().map(String::as_str).collect();
    let fake = FakeLocal::start(
        &socket,
        &LINUX_FAMILIES,
        &[Property::IpcSocketAllowlist, Property::IpcSignalsContained],
        &served,
    );
    let mut sdk = placement::Sdk::default();
    sdk.socket = socket.clone();
    let mut instance = linux_instance(&case);
    instance.placement = PlacementName("systemd".to_owned());
    let binary = Path::new("/opt/conductor/bin/conductor");
    let refused = sdk
        .open(&placement::request(
            &instance,
            &case.file(),
            binary,
            Vec::new(),
        ))
        .expect_err("a daemon without limits opens no systemd box");
    assert_eq!(refused.name, "confinement.family-unserved", "{refused:?}");
    let lines = fake.lines.lock().expect("the lines").clone();
    assert!(
        lines
            .iter()
            .all(|line| line.starts_with("GET /v1/machine ")),
        "nothing but the machine was asked: {lines:?}"
    );

    // linux's facts are all there: the box is asked for.
    let refused = sdk
        .open(&placement::request(
            &linux_instance(&case),
            &case.file(),
            binary,
            Vec::new(),
        ))
        .expect_err("the fake answers the workspace 503");
    assert_ne!(refused.name, "confinement.family-unserved", "{refused:?}");
    let lines = fake.lines.lock().expect("the lines").clone();
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("POST /v1/workspaces ")),
        "the workspace was asked for: {lines:?}"
    );
}

#[test]
fn the_sdk_refuses_a_box_whose_writable_root_the_local_daemon_does_not_serve_before_anything_starts()
 {
    let case = Case::new("unroot");
    let socket = case.dir.join("s.sock");
    let request = placement::request(
        &linux_instance(&case),
        &case.file(),
        Path::new("/opt/conductor/bin/conductor"),
        Vec::new(),
    );
    let root = request.writable_roots[0].name.clone();
    for roots in [&[][..], &["sessions-other"][..]] {
        let fake = FakeLocal::opening(
            &socket,
            &LINUX_FAMILIES,
            &[Property::IpcSocketAllowlist, Property::IpcSignalsContained],
            roots,
        );
        let mut sdk = placement::Sdk::default();
        sdk.socket = socket.clone();
        let refused = sdk
            .open(&request)
            .expect_err("a daemon that does not serve the root opens no box");
        assert_eq!(
            refused.name, "exec.writable-roots-host-unserved",
            "substrate's name: {refused:?}"
        );
        assert!(refused.detail.contains(&root), "{refused:?}");
        let lines = fake.lines.lock().expect("the lines").clone();
        assert!(
            lines
                .iter()
                .all(|line| line.starts_with("GET /v1/machine ")),
            "nothing but the machine was asked ({roots:?}): {lines:?}"
        );
    }
}

#[test]
fn the_box_s_runtime_logs_to_a_file_in_its_writable_root_not_to_its_session() {
    let case = Case::new("runtime-log");
    let instance = linux_instance(&case);
    let request = placement::request(
        &instance,
        &case.file(),
        Path::new("/opt/conductor/bin/conductor"),
        Vec::new(),
    );
    let log = request
        .env
        .iter()
        .find(|(name, _)| name == conductor_cli::instance::LOG_VARIABLE)
        .map(|(_, value)| PathBuf::from(value))
        .expect("the runtime is told where to log");
    assert_eq!(
        log,
        Path::new(&instance.state).join("sessions/runtime.log"),
        "{request:?}"
    );
    assert!(
        log.starts_with(&request.writable_roots[0].path),
        "the log is in the writable root: {request:?}"
    );
}

#[test]
fn conductord_creates_the_sessions_root_mode_0700_before_it_asks_for_the_box() {
    let case = Case::new("sessions-root");
    case.config(&[("alpha", "linux")]);
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    let log = fake.log.lock().expect("the log");
    assert_eq!(
        log.sessions_at_open,
        [Some(0o700)],
        "the sessions directory is a 0700 directory when the box is asked for: {:?}",
        log.opened
    );
    let state = case.dir.join("alpha/state");
    let holds_state = log.opened[0]
        .writable_roots
        .iter()
        .position(|root| state.starts_with(&root.path))
        .expect("a writable root holds the state directory");
    assert!(
        log.roots_at_open[0][holds_state].is_some(),
        "the root that holds the state directory is there when the box is asked for: {:?}",
        log.opened
    );
}

#[test]
fn conductord_starts_again_a_box_runtime_substrate_ended_at_its_output_bound() {
    let case = Case::new("output-bound");
    case.config(&[("alpha", "linux")]);
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    let now = std::time::Instant::now();
    daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            now,
        )
        .expect("a pass");
    assert_eq!(daemon.due(now).expect("due"), None);

    // Substrate ends the session with `session.output-limit` (state Cancelled): it no longer
    // runs. It ran past the start bound, so this is no failed start.
    fake.log.lock().expect("the log").running.remove("ses_1");
    let now = now + conductor_cli::daemon::reconcile::START_BOUND;
    assert_eq!(
        daemon.due(now).expect("due"),
        Some(conductor_model::instance::ReconcileReason::LostRuntime)
    );
    let done = daemon
        .pass(conductor_model::instance::ReconcileReason::LostRuntime, now)
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].action, ReconcileAction::Start);
    assert_eq!(
        done[0].reason,
        conductor_model::instance::ReconcileReason::LostRuntime
    );
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    {
        let log = fake.log.lock().expect("the log");
        assert_eq!(log.opened.len(), 2, "a new box: {:?}", log.opened);
        assert_eq!(log.closed, ["ws_1".to_owned()], "the old box is closed");
    }
    assert_eq!(daemon.started()["alpha"].unit, "substrate:ses_2");
    let boxes = BoxStorage::list(daemon.store());
    assert_eq!(
        boxes
            .iter()
            .filter(|row| row.state == BoxState::Opened)
            .count(),
        1,
        "one box open on record: {boxes:?}"
    );
    assert_eq!(daemon.due(now).expect("due"), None);
}

#[test]
fn conductord_opens_a_systemd_instance_in_a_substrate_box_never_on_its_own_units() {
    let case = Case::new("daemon-systemd");
    case.config(&[("alpha", "{daemon: local, adapter: systemd}")]);
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
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
    let log = fake.log.lock().expect("the log");
    assert_eq!(log.opened.len(), 1);
    assert_eq!(
        log.opened[0].requires,
        placement::requires(Adapter::Systemd)
    );
}

impl Case {
    /// [`Case::config`] with a slot pool of `slots` and each instance's model token
    /// `secret:<instance>-model`.
    fn config_with_tokens(&self, slots: usize, instances: &[(&str, &str)]) -> PathBuf {
        let file = self.config(instances);
        let mut text = fs::read_to_string(&file).expect("the config");
        text = text.replace(
            "daemon: {port: 7300,",
            &format!("daemon: {{port: 7300, substrate_slots: {slots},"),
        );
        for (name, _) in instances {
            let cache = format!("    cache: {:?}\n", self.dir.join(name).join("cache"));
            text = text.replace(
                &cache,
                &format!("{cache}    model_token: \"secret:{name}-model\"\n"),
            );
        }
        fs::write(&file, text).expect("write the config");
        file
    }
}

/// The value of the model token of `name` as the fake secrets source answers it.
fn token_of(name: &str) -> String {
    format!("model-token-of-{name}")
}

fn token_source() -> placement::TokenSource {
    Arc::new(|_source, name: &conductor_model::secret::SecretName| {
        let instance = name.0.strip_suffix("-model")?;
        secrets_core::storage::SecretValue::new(token_of(instance).into_bytes()).ok()
    })
}

#[test]
fn a_box_seals_its_model_token_through_a_free_slot_of_the_pool_and_frees_it_when_it_closes() {
    use std::os::unix::fs::PermissionsExt as _;
    let case = Case::new("slot-pool");
    let file = case.config_with_tokens(2, &[("alpha", "linux"), ("beta", "linux")]);
    let validated = case.conductor(&[
        "--config",
        &file.display().to_string(),
        "config",
        "validate",
    ]);
    assert_eq!(validated.status.code(), Some(0), "{}", describe(&validated));
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    daemon.tokens = token_source();
    let now = std::time::Instant::now();
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            now,
        )
        .expect("a pass");
    assert!(
        done.iter()
            .all(|done| done.result == ReconcileResult::Succeeded),
        "{done:?}"
    );
    let slots = case.dir.join("daemon").join(placement::SLOTS_DIR);
    let slot_of = |log: &Log, instance: &str| {
        log.opened
            .iter()
            .find(|request| request.instance.0 == instance)
            .map(|request| request.secret_slots.clone())
            .expect("its box was opened")
    };
    let (alpha_slot, beta_slot) = {
        let log = fake.log.lock().expect("the log");
        let alpha = slot_of(&log, "alpha");
        let beta = slot_of(&log, "beta");
        assert_eq!(alpha.len(), 1, "{alpha:?}");
        assert_eq!(beta.len(), 1, "{beta:?}");
        assert_ne!(alpha, beta, "each box has its own slot");
        for request in &log.opened {
            let shown = format!("{request:?}");
            assert!(
                !shown.contains(&token_of("alpha")) && !shown.contains(&token_of("beta")),
                "no token in the request: {shown}"
            );
        }
        (alpha[0].clone(), beta[0].clone())
    };
    for (slot, name) in [(&alpha_slot, "alpha"), (&beta_slot, "beta")] {
        assert!(slot.starts_with("model_token_"), "{slot}");
        let path = slots.join(slot);
        assert_eq!(
            fs::read_to_string(&path).expect("the slot's file"),
            token_of(name)
        );
        assert_eq!(
            fs::metadata(&path)
                .expect("its metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    // A third instance finds no free slot: its box is refused by name and nothing starts.
    case.config_with_tokens(
        2,
        &[("alpha", "linux"), ("beta", "linux"), ("gamma", "linux")],
    );
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            now,
        )
        .expect("a pass");
    let gamma = done
        .iter()
        .find(|done| done.instance == "gamma")
        .expect("gamma reconciled");
    assert_eq!(gamma.result, ReconcileResult::Refused, "{done:?}");
    assert!(
        gamma
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains(placement::SLOTS_EXHAUSTED)),
        "{gamma:?}"
    );
    assert!(
        BoxRefusalStorage::list(daemon.store())
            .iter()
            .any(|refusal| refusal.data.reason.0 == placement::SLOTS_EXHAUSTED)
    );

    // alpha leaves: its box closes at the end of the pass and frees its slot; gamma, refused
    // while it was held, takes it at the next pass.
    case.config_with_tokens(2, &[("beta", "linux"), ("gamma", "linux")]);
    daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            now,
        )
        .expect("a pass");
    daemon
        .pass(conductor_model::instance::ReconcileReason::Cadence, now)
        .expect("a pass");
    let gamma_slot = slot_of(&fake.log.lock().expect("the log"), "gamma");
    assert_eq!(
        gamma_slot,
        std::slice::from_ref(&alpha_slot),
        "the freed slot is reused"
    );
    assert_eq!(
        fs::read_to_string(slots.join(&alpha_slot)).expect("the slot's file"),
        token_of("gamma")
    );
    case.config_with_tokens(2, &[("beta", "linux")]);
    daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            now,
        )
        .expect("a pass");
    let freed = fs::read_to_string(slots.join(&alpha_slot)).expect("the slot's file");
    assert!(
        !freed.contains("model-token-of"),
        "a freed slot holds no token: {freed:?}"
    );
    assert!(
        !freed.is_empty(),
        "substrate refuses an empty slot file at its start"
    );
}

#[test]
fn conductord_renews_a_box_and_restarts_its_runtime_in_the_same_box_before_the_run_bound() {
    let case = Case::new("run-bound");
    case.config(&[("alpha", "linux")]);
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    let start = std::time::SystemTime::now();
    let hour = std::time::Duration::from_secs(3600);

    let done = daemon.keep_boxes(start + 2 * hour).expect("keep the boxes");
    assert!(done.is_empty(), "{done:?}");
    {
        let log = fake.log.lock().expect("the log");
        assert_eq!(log.renewed, [("ws_1".to_owned(), "ses_1".to_owned())]);
        assert!(log.restarted.is_empty());
    }

    let done = daemon
        .keep_boxes(start + 23 * hour + hour / 2)
        .expect("keep the boxes");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].action, ReconcileAction::Restart);
    assert_eq!(
        done[0].reason,
        conductor_model::instance::ReconcileReason::RunBound
    );
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    {
        let log = fake.log.lock().expect("the log");
        assert_eq!(log.restarted.len(), 1);
        assert_eq!(log.restarted[0].0, "ws_1", "the same box");
        assert_eq!(log.restarted[0].1.argv[3..], ["instance", "serve"]);
        assert!(log.stopped.contains(&"ses_1".to_owned()));
        assert!(log.closed.is_empty(), "the box stays open");
    }
    assert_eq!(daemon.started()["alpha"].unit, "substrate:ses_r1");
    assert!(
        ReconciliationStorage::list(daemon.store())
            .iter()
            .any(|row| row.data.reason == conductor_model::instance::ReconcileReason::RunBound)
    );
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::Cadence,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert!(done.is_empty(), "the restarted runtime runs: {done:?}");
}

// ---------------------------------------------------------------------------------------------
// `story:box-runtime-starts`: the box carries HOME and mounts what the runtime reads and writes
// ---------------------------------------------------------------------------------------------

/// Whether `path` is at or under a writable root of `request`.
fn writes(request: &BoxRequest, path: &Path) -> bool {
    request
        .writable_roots
        .iter()
        .any(|root| path.starts_with(&root.path))
}

#[test]
fn conductord_asks_for_a_systemd_box_with_home_and_only_its_own_directories_writable() {
    let case = Case::new("box-home");
    let file = case.config(&[("alpha", "systemd")]);
    let store = case.dir.join("store/shared.sqlite");
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    let log = fake.log.lock().expect("the log");
    let asked = &log.opened[0];

    let home = asked
        .env
        .iter()
        .find(|(name, _)| name == "HOME")
        .map(|(_, value)| value.clone());
    assert_eq!(
        home,
        Some(case.home().display().to_string()),
        "the box carries the daemon's HOME: {asked:?}"
    );

    let alpha = case.dir.join("alpha");
    for (what, path) in [
        ("the state directory", alpha.join("state")),
        ("the loop's lock", alpha.join("state/run/lock")),
        ("the API socket", alpha.join("state/api.sock")),
        ("the records", alpha.join("records")),
        ("the collectors' exports", alpha.join("cache/exports")),
        ("the decider's directory", alpha.join("cache/decide")),
        ("the store", store.clone()),
    ] {
        assert!(
            writes(asked, &path),
            "{what} ({}) is in a writable root: {asked:?}",
            path.display()
        );
    }
    assert_eq!(
        asked
            .writable_roots
            .iter()
            .map(|root| root.name.as_str())
            .collect::<Vec<_>>(),
        ["state-alpha", "records-alpha", "cache-alpha", "store"],
        "the instance's own directories and the store's, nothing else"
    );
    // The daemon was opened on its own directory, not the default one under HOME.
    for (what, path) in [
        ("the config file", file.clone()),
        (
            "the runtimes' tokens",
            case.dir.join("daemon/runtimes.json"),
        ),
        ("the pinned binaries", case.dir.join("daemon/binaries.json")),
    ] {
        assert!(
            !writes(asked, &path),
            "{what} ({}) is in no writable root: {asked:?}",
            path.display()
        );
    }
    assert!(
        asked
            .read_only_files
            .iter()
            .any(|known| known.mount == file.display().to_string()),
        "the config file is read: {asked:?}"
    );
    assert_substrate_admits(asked);
}

/// The store beside the config file and the daemon's directory, as `~/.b10x/conductor` holds
/// all three by default: a writable root is a whole directory, so no box can write that store.
#[test]
fn conductord_refuses_a_box_whose_store_shares_its_directory_before_substrate_is_asked() {
    let case = Case::new("box-store-shared");
    let file = case.config(&[("alpha", "systemd")]);
    let beside = format!("sqlite:{}", case.dir.join("shared.sqlite").display());
    let text = fs::read_to_string(&file)
        .expect("the config")
        .replace(&case.url(), &beside);
    fs::write(&file, text).expect("write the config file");
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    let done = daemon
        .pass(
            conductor_model::instance::ReconcileReason::ConfigChange,
            std::time::Instant::now(),
        )
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].result, ReconcileResult::Refused, "{done:?}");
    let detail = done[0].detail.clone().unwrap_or_default();
    assert!(
        detail.contains("PlacementRefused")
            && detail.contains("placement.writable-root-shared")
            && detail.contains(&format!(
                "daemon.store: sqlite:{}",
                case.dir.join("store/shared.sqlite").display()
            )),
        "the refusal names the store path to write: {detail}"
    );
    assert!(
        fake.log.lock().expect("the log").opened.is_empty(),
        "substrate is not asked"
    );
    assert_eq!(BoxRefusalStorage::list(daemon.store()).len(), 1);
}

/// An executable file at `path`.
fn program(path: &Path) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create the directory");
    fs::write(path, "#!/bin/sh\n").expect("write the program");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

fn real(path: &Path) -> String {
    fs::canonicalize(path)
        .expect("the path is there")
        .display()
        .to_string()
}

/// Substrate's lexical rules for one start (its `validate_read_only_roots`,
/// `validate_read_only_files` and `validate_writable_roots`): at most 4 read-only roots, 16
/// read-only files and 8 writable roots, each mount absolute with no trailing slash and no
/// empty, `.` or `..` component, and no mount at or under another.
fn assert_substrate_admits(request: &BoxRequest) {
    assert!(request.read_only_roots.len() <= 4, "{request:?}");
    assert!(request.read_only_files.len() <= 16, "{request:?}");
    assert!(request.writable_roots.len() <= 8, "{request:?}");
    let mounts: Vec<&str> = request
        .read_only_roots
        .iter()
        .map(String::as_str)
        .chain(
            request
                .read_only_files
                .iter()
                .flat_map(|file| [file.mount.as_str(), file.host_path.as_str()]),
        )
        .chain(request.writable_roots.iter().map(|root| root.path.as_str()))
        .collect();
    for mount in &mounts {
        assert!(
            mount.starts_with('/')
                && mount.len() > 1
                && mount[1..]
                    .split('/')
                    .all(|part| !part.is_empty() && part != "." && part != ".."),
            "{mount} is not canonical: {request:?}"
        );
    }
    let mounts: Vec<&str> = request
        .read_only_roots
        .iter()
        .map(String::as_str)
        .chain(
            request
                .read_only_files
                .iter()
                .map(|file| file.mount.as_str()),
        )
        .chain(request.writable_roots.iter().map(|root| root.path.as_str()))
        .collect();
    for (index, one) in mounts.iter().enumerate() {
        for other in &mounts[index + 1..] {
            assert!(
                !Path::new(one).starts_with(other) && !Path::new(other).starts_with(one),
                "{one} and {other} nest: {request:?}"
            );
        }
    }
}

#[test]
fn a_box_mounts_what_the_runtime_and_the_loop_read_and_write_each_named_in_one_list() {
    let case = Case::new("box-mounts");
    let file = case.config(&[("alpha", "systemd")]);
    let mut instance = linux_instance(&case);
    instance.placement = PlacementName("systemd".to_owned());
    let own = case.dir.join("i");
    instance.state = own.join("state").display().to_string();
    instance.records = own.join("records").display().to_string();
    instance.cache = own.join("cache").display().to_string();
    instance.checkouts.root = own.join("checkouts").display().to_string();
    instance.checkouts.trees = own.join("trees").display().to_string();

    // The harness as its installer leaves it: a link in a directory on PATH to a versioned file.
    let harness = case.dir.join("tools/share/claude/versions/2.1.0");
    program(&harness);
    fs::create_dir_all(case.dir.join("tools/bin")).expect("create the directory");
    std::os::unix::fs::symlink(&harness, case.dir.join("tools/bin/claude")).expect("link");
    // The forge CLI in a directory no mount holds, itself a link.
    let cli = case.dir.join("forge/cellar/gh");
    program(&cli);
    fs::create_dir_all(case.dir.join("forge/bin")).expect("create the directory");
    std::os::unix::fs::symlink(&cli, case.dir.join("forge/bin/gh")).expect("link");
    // Its login: one of its two files is there.
    let login = case.home().join(".config/gh/hosts.yml");
    fs::create_dir_all(login.parent().expect("a parent")).expect("create the directory");
    fs::write(&login, "github.com: {}\n").expect("write the login");

    let store = case.dir.join("store/conductor.sqlite");
    let env = vec![
        ("HOME".to_owned(), case.home().display().to_string()),
        (
            "PATH".to_owned(),
            format!(
                "{}:{}",
                case.dir.join("tools/bin").display(),
                case.dir.join("forge/bin").display()
            ),
        ),
        (
            conductor_cli::store::STORE_VARIABLE.to_owned(),
            format!("sqlite:{}", store.display()),
        ),
    ];
    let binary = Path::new("/opt/conductor/bin/conductor");

    let listed = placement::mounts(&instance, &file, binary, &placement::Host::of(&env));
    assert_eq!(
        listed.iter().map(|mount| mount.what).collect::<Vec<_>>(),
        [
            "state",
            "records",
            "cache",
            "store",
            "binary",
            "harness",
            "checkouts",
            "config",
            "harness",
            "forge-cli",
            "forge-login",
        ],
        "the one list: {listed:?}"
    );

    let request = placement::request(&instance, &file, binary, env);
    assert_eq!(request.refused, None, "a box can hold this layout");
    let mut store_of =
        conductor_cli::store::open_url(&case.url(), conductor_cli::daemon::TENANT).expect("store");
    let mut fake = Fake::default();
    placement::open_box(&mut store_of, &mut fake, &request).expect("the box opens");
    let log = fake.log.lock().expect("the log");
    let asked = &log.opened[0];

    assert!(
        asked
            .env
            .contains(&("HOME".to_owned(), case.home().display().to_string())),
        "the box carries HOME: {asked:?}"
    );
    let name = &instance.name.0;
    assert_eq!(
        asked.writable_roots,
        [
            placement::HostRoot {
                name: format!("state-{name}"),
                path: instance.state.clone(),
            },
            placement::HostRoot {
                name: format!("records-{name}"),
                path: instance.records.clone(),
            },
            placement::HostRoot {
                name: format!("cache-{name}"),
                path: instance.cache.clone(),
            },
            placement::HostRoot {
                name: "store".to_owned(),
                path: case.dir.join("store").display().to_string(),
            },
        ],
        "the state directory, the records, the cache and the store's directory are written"
    );
    assert_eq!(
        asked.read_only_roots,
        [
            "/opt/conductor/bin".to_owned(),
            case.dir.join("tools/bin").display().to_string(),
            instance.checkouts.root.clone(),
        ],
        "the binary's and the harness's directories and the checkouts root are read"
    );
    let at_own = |path: &Path| placement::HostFile {
        host_path: real(path),
        mount: real(path),
    };
    assert_eq!(
        asked.read_only_files,
        [
            // The config file, at the path `--config` names.
            placement::HostFile {
                host_path: real(&file),
                mount: file.display().to_string(),
            },
            // The file the harness's link names, at its own path: the link is in a root.
            at_own(&harness),
            // The forge CLI at the path PATH names it by, the file its link names.
            placement::HostFile {
                host_path: real(&cli),
                mount: case.dir.join("forge/bin/gh").display().to_string(),
            },
            // Its login, the one file that is there.
            placement::HostFile {
                host_path: real(&login),
                mount: login.display().to_string(),
            },
        ],
        "the config file, the harness binary, the forge CLI and its login are read"
    );
    assert_substrate_admits(asked);
}

/// The layout the getting-started page had: the config file, the store and conductord's
/// directory in one directory, each instance's state and records under it.
#[test]
fn a_box_writes_no_directory_that_holds_the_config_file_or_the_daemons_directory() {
    let case = Case::new("box-layout");
    let top = case.home().join(".b10x/conductor");
    fs::create_dir_all(&top).expect("create the directory");
    let file = top.join("conductor.yaml");
    fs::write(&file, "version: conductor.config/2\n").expect("write the config file");
    let mut instance = linux_instance(&case);
    instance.placement = PlacementName("systemd".to_owned());
    instance.state = top.join("home/state").display().to_string();
    instance.records = top.join("home/records").display().to_string();
    instance.cache = case.home().join(".cache/home").display().to_string();
    let env = |store: String| {
        vec![
            ("HOME".to_owned(), case.home().display().to_string()),
            (
                "PATH".to_owned(),
                case.dir.join("none").display().to_string(),
            ),
            (conductor_cli::store::STORE_VARIABLE.to_owned(), store),
        ]
    };
    let binary = Path::new("/usr/bin/conductor");
    let name = &instance.name.0;
    let own = [
        placement::HostRoot {
            name: format!("state-{name}"),
            path: instance.state.clone(),
        },
        placement::HostRoot {
            name: format!("records-{name}"),
            path: instance.records.clone(),
        },
        placement::HostRoot {
            name: format!("cache-{name}"),
            path: instance.cache.clone(),
        },
    ];

    // The store beside the config file: its directory is no root, and the box is refused.
    let request = placement::request(
        &instance,
        &file,
        binary,
        env(format!("sqlite:{}", top.join("conductor.sqlite").display())),
    );
    assert_eq!(
        request.writable_roots, own,
        "the store's directory is not written"
    );
    let refused = request
        .refused
        .clone()
        .expect("no box can write that store");
    assert_eq!(refused.name, placement::ROOT_SHARED);
    assert!(
        refused.detail.contains(&file.display().to_string())
            && refused.detail.contains(&format!(
                "daemon.store: sqlite:{}",
                top.join("store/conductor.sqlite").display()
            )),
        "the refusal names what the directory holds and the store path to write: {refused:?}"
    );
    assert_eq!(
        request.read_only_files,
        [placement::HostFile {
            host_path: real(&file),
            mount: file.display().to_string(),
        }],
        "the config file is read"
    );
    assert!(
        request
            .read_only_roots
            .iter()
            .all(|root| !Path::new(root).starts_with("/usr")),
        "substrate reserves the base system's mounts: {request:?}"
    );
    assert_substrate_admits(&request);

    // The store in a directory of its own, under `~/`: read under the box's HOME, and written.
    let request = placement::request(
        &instance,
        &file,
        binary,
        env("sqlite:~/.b10x/conductor/store/conductor.sqlite".to_owned()),
    );
    assert_eq!(request.refused, None);
    assert_eq!(
        request.writable_roots[3],
        placement::HostRoot {
            name: "store".to_owned(),
            path: top.join("store").display().to_string(),
        }
    );
    assert_substrate_admits(&request);

    // A store in the daemon's own directory, and a state directory that holds the config file.
    let request = placement::request(
        &instance,
        &file,
        binary,
        env("sqlite:~/.b10x/conductor/daemon/db/conductor.sqlite".to_owned()),
    );
    assert_eq!(
        request.refused.map(|refused| refused.name),
        Some(placement::ROOT_SHARED.to_owned())
    );
    instance.state = top.display().to_string();
    let listed = placement::mounts(
        &instance,
        &file,
        binary,
        &placement::Host {
            home: Some(case.home()),
            path: None,
            store: None,
            daemon: None,
        },
    );
    assert!(
        listed.iter().all(|mount| mount.what != "state"),
        "{listed:?}"
    );
    let refused = placement::unmountable(
        &instance,
        &file,
        binary,
        &placement::Host {
            home: Some(case.home()),
            path: None,
            store: None,
            daemon: None,
        },
    )
    .expect("the state directory holds the config file");
    assert_eq!(refused.name, placement::ROOT_SHARED);
    assert!(refused.detail.contains("state directory"), "{refused:?}");
}

/// The records under the checkouts root, as the configure-an-instance guide writes them:
/// substrate refuses a directory to read that holds one to write.
#[test]
fn a_directory_to_read_that_holds_one_to_write_is_named_by_its_other_entries() {
    let case = Case::new("box-around");
    let file = case.config(&[("alpha", "systemd")]);
    let mut instance = linux_instance(&case);
    instance.placement = PlacementName("systemd".to_owned());
    let own = case.dir.join("i");
    let root = own.join("checkouts");
    // Written with what substrate takes no mount with: a trailing slash, a `.` component.
    instance.state = format!("{}/./state/", own.display());
    instance.records = format!("{}/records/", root.display());
    instance.cache = own.join("cache").display().to_string();
    instance.checkouts.root = format!("{}/", root.display());
    instance.checkouts.trees = own.join("trees").display().to_string();
    for dir in ["records/decisions", "one/src", "two"] {
        fs::create_dir_all(root.join(dir)).expect("create the directory");
    }
    fs::write(root.join("notes.md"), "notes\n").expect("write the file");
    std::os::unix::fs::symlink(root.join("one"), root.join("link")).expect("link");
    let env = || {
        vec![
            ("HOME".to_owned(), case.home().display().to_string()),
            (
                "PATH".to_owned(),
                case.dir.join("none").display().to_string(),
            ),
        ]
    };
    let binary = Path::new("/usr/bin/conductor");

    let request = placement::request(&instance, &file, binary, env());
    assert_eq!(request.refused, None, "{request:?}");
    assert_eq!(
        request.cwd,
        own.join("state").display().to_string(),
        "the state directory as substrate takes it"
    );
    assert_eq!(
        request
            .writable_roots
            .iter()
            .map(|root| root.path.clone())
            .collect::<Vec<_>>(),
        [
            own.join("state").display().to_string(),
            root.join("records").display().to_string(),
            own.join("cache").display().to_string(),
        ]
    );
    assert_eq!(
        request.read_only_roots,
        [
            root.join("one").display().to_string(),
            root.join("two").display().to_string(),
        ],
        "each directory of the checkouts root but the records, no link to one"
    );
    assert!(
        request.read_only_files.contains(&placement::HostFile {
            host_path: real(&root.join("notes.md")),
            mount: root.join("notes.md").display().to_string(),
        }),
        "each file of the checkouts root: {request:?}"
    );
    assert_substrate_admits(&request);

    // More entries than one start names: refused by conductor, naming the directory.
    for dir in ["three", "four", "five"] {
        fs::create_dir_all(root.join(dir)).expect("create the directory");
    }
    let request = placement::request(&instance, &file, binary, env());
    let refused = request.refused.clone().expect("five directories to read");
    assert_eq!(refused.name, placement::MOUNTS_EXCEEDED);
    assert!(
        refused.detail.contains(&root.display().to_string())
            && refused.detail.contains("5 directories to read")
            && refused
                .detail
                .contains(&format!("put records outside {}", root.display())),
        "the refusal names the directory and the fix: {refused:?}"
    );
    let mut store =
        conductor_cli::store::open_url(&case.url(), conductor_cli::daemon::TENANT).expect("store");
    let mut fake = Fake::default();
    let answer = placement::open_box(&mut store, &mut fake, &request)
        .expect_err("conductor refuses the box");
    assert_eq!(answer.reason, placement::MOUNTS_EXCEEDED);
    assert!(
        fake.log.lock().expect("the log").opened.is_empty(),
        "substrate is not asked"
    );
}

/// The layout the configure-an-instance guide writes: the records outside the checkouts root,
/// which is then one directory to read however many repositories it holds.
#[test]
fn a_checkouts_root_of_six_repositories_with_the_records_outside_it_is_one_directory_to_read() {
    let case = Case::new("box-six");
    let file = case.config(&[("alpha", "systemd")]);
    let mut instance = linux_instance(&case);
    instance.placement = PlacementName("systemd".to_owned());
    let own = case.dir.join("i");
    let root = own.join("checkouts");
    instance.state = own.join("state").display().to_string();
    instance.records = own.join("records").display().to_string();
    instance.cache = own.join("cache").display().to_string();
    instance.checkouts.root = root.display().to_string();
    instance.checkouts.trees = own.join("trees").display().to_string();
    for repository in ["one", "two", "three", "four", "five", "six"] {
        fs::create_dir_all(root.join(repository).join("src")).expect("create the directory");
    }
    let env = vec![
        ("HOME".to_owned(), case.home().display().to_string()),
        (
            "PATH".to_owned(),
            case.dir.join("none").display().to_string(),
        ),
        (conductor_cli::store::STORE_VARIABLE.to_owned(), case.url()),
    ];
    let request = placement::request(&instance, &file, Path::new("/usr/bin/conductor"), env);
    assert_eq!(request.refused, None, "{request:?}");
    assert_eq!(
        request.read_only_roots,
        [root.display().to_string()],
        "the checkouts root, whole"
    );
    assert_eq!(request.writable_roots.len(), 4, "{request:?}");
    assert_substrate_admits(&request);
}

/// The bound of files to read (`story:box-runtime-starts`): the records under the checkouts
/// root, so each loose file beside them is named alone. Sixteen are one start's; the seventeenth
/// is refused by conductor's own name before substrate is asked. The config file is in a
/// directory of the root, read with it, so the loose files are every file the box names.
#[test]
fn sixteen_loose_files_beside_the_records_are_admitted_and_the_seventeenth_is_refused() {
    let case = Case::new("box-files");
    let mut instance = linux_instance(&case);
    instance.placement = PlacementName("systemd".to_owned());
    let own = case.dir.join("i");
    let root = own.join("checkouts");
    instance.state = own.join("state").display().to_string();
    instance.records = root.join("records").display().to_string();
    instance.cache = own.join("cache").display().to_string();
    instance.checkouts.root = root.display().to_string();
    instance.checkouts.trees = own.join("trees").display().to_string();
    fs::create_dir_all(root.join("records")).expect("create the records");
    fs::create_dir_all(root.join("etc")).expect("create the directory");
    let file = root.join("etc/conductor.yaml");
    fs::copy(case.config(&[("alpha", "systemd")]), &file).expect("copy the config file");
    let asked = || {
        let env = vec![
            ("HOME".to_owned(), case.home().display().to_string()),
            (
                "PATH".to_owned(),
                case.dir.join("none").display().to_string(),
            ),
            (conductor_cli::store::STORE_VARIABLE.to_owned(), case.url()),
        ];
        placement::request(&instance, &file, Path::new("/usr/bin/conductor"), env)
    };
    for number in 0..placement::READ_ONLY_FILES_BOUND {
        fs::write(root.join(format!("note-{number}.txt")), "").expect("write a loose file");
    }
    let request = asked();
    assert_eq!(request.refused, None, "{request:?}");
    assert_eq!(
        request.read_only_files.len(),
        placement::READ_ONLY_FILES_BOUND,
        "{:?}",
        request.read_only_files
    );
    assert_substrate_admits(&request);

    fs::write(root.join("note-more.txt"), "").expect("write a loose file");
    let request = asked();
    let refused = request.refused.clone().expect("seventeen files to read");
    assert_eq!(refused.name, placement::MOUNTS_EXCEEDED);
    assert!(
        refused.detail.contains("17 files to read")
            && refused
                .detail
                .contains(&format!("put records outside {}", root.display())),
        "{refused:?}"
    );
}

/// `config validate` names the refusal before conductord is started.
#[test]
fn config_validate_names_a_box_conductord_would_refuse() {
    let case = Case::new("validate-box");
    let file = case.config(&[("alpha", "systemd")]);
    let validate = |file: &Path| {
        case.conductor(&[
            "--config",
            &file.display().to_string(),
            "config",
            "validate",
        ])
    };
    let output = validate(&file);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));

    // The store beside the config file.
    let beside = format!("sqlite:{}", case.dir.join("shared.sqlite").display());
    let text = fs::read_to_string(&file)
        .expect("the config")
        .replace(&case.url(), &beside);
    fs::write(&file, text).expect("write the config file");
    let output = validate(&file);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        said.contains("instances[0].placement")
            && said.contains("placement.writable-root-shared")
            && said.contains(&format!(
                "daemon.store: sqlite:{}",
                case.dir.join("store/shared.sqlite").display()
            )),
        "the problem names its path, the refusal and the store to write: {said}"
    );
}

/// A sqlite store written without an absolute path: conductord would open it in its working
/// directory and the box could be handed no directory of it, so the box is refused by
/// conductor's own name before substrate is asked, and `config validate` notes it.
#[test]
fn a_box_whose_sqlite_store_has_no_absolute_path_is_refused_and_validate_notes_it() {
    let case = Case::new("box-relative-store");
    let file = case.config(&[("alpha", "systemd")]);
    let mut instance = linux_instance(&case);
    instance.placement = PlacementName("systemd".to_owned());
    let env = vec![
        ("HOME".to_owned(), case.home().display().to_string()),
        (
            "PATH".to_owned(),
            case.dir.join("none").display().to_string(),
        ),
        (
            conductor_cli::store::STORE_VARIABLE.to_owned(),
            "sqlite:shared.sqlite".to_owned(),
        ),
    ];
    let request = placement::request(&instance, &file, Path::new("/usr/bin/conductor"), env);
    let refused = request.refused.clone().expect("no directory to write");
    assert_eq!(refused.name, placement::STORE_NOT_ABSOLUTE);
    assert!(
        refused.detail.contains("shared.sqlite")
            && refused.detail.contains("write an absolute path")
            && refused
                .detail
                .contains("daemon.store: sqlite:~/.b10x/conductor/store/conductor.sqlite"),
        "{refused:?}"
    );
    assert!(
        request
            .writable_roots
            .iter()
            .all(|root| root.name != "store"),
        "{:?}",
        request.writable_roots
    );
    let mut store =
        conductor_cli::store::open_url(&case.url(), conductor_cli::daemon::TENANT).expect("store");
    let mut fake = Fake::default();
    let answer = placement::open_box(&mut store, &mut fake, &request)
        .expect_err("conductor refuses the box");
    assert_eq!(answer.reason, placement::STORE_NOT_ABSOLUTE);
    assert!(
        fake.log.lock().expect("the log").opened.is_empty(),
        "substrate is not asked"
    );

    let text = fs::read_to_string(&file)
        .expect("the config")
        .replace(&case.url(), "sqlite:shared.sqlite");
    fs::write(&file, text).expect("write the config file");
    let output = case.conductor(&[
        "--config",
        &file.display().to_string(),
        "config",
        "validate",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        said.contains("note: instances[0].placement: placement.store-not-absolute: "),
        "{said}"
    );
}

#[test]
fn the_sdk_starts_the_runtime_with_each_read_only_file_at_its_mount() {
    let case = Case::new("files");
    let socket = case.dir.join("s.sock");
    let file = case.config(&[("alpha", "linux")]);
    let request = placement::request(
        &linux_instance(&case),
        &file,
        Path::new("/opt/conductor/bin/conductor"),
        Vec::new(),
    );
    assert!(
        request
            .read_only_files
            .iter()
            .any(|known| known.mount == file.display().to_string()),
        "the config file is a read-only file: {request:?}"
    );
    let served: Vec<&str> = request
        .writable_roots
        .iter()
        .map(|root| root.name.as_str())
        .collect();
    let fake = FakeLocal::opening(
        &socket,
        &LINUX_FAMILIES,
        &[Property::IpcSocketAllowlist, Property::IpcSignalsContained],
        &served,
    );
    let mut sdk = placement::Sdk::default();
    sdk.socket = socket.clone();
    let expected: Vec<Value> = request
        .read_only_files
        .iter()
        .map(|file| serde_json::json!({"host_path": file.host_path, "mount": file.mount}))
        .collect();
    let refused = sdk
        .open(&request)
        .expect_err("the fake answers the session start 503");
    let starts = fake.starts.lock().expect("the starts").clone();
    assert_eq!(starts.len(), 1, "one session start: {starts:?} {refused:?}");
    assert_eq!(
        starts[0].pointer("/input/exec/read_only_files"),
        Some(&Value::Array(expected)),
        "the start names each file: {}",
        starts[0]
    );
}

// ---------------------------------------------------------------------------------------------
// story:box-crash-loop-reads-failed: a runtime that exits at once is a failed start
// ---------------------------------------------------------------------------------------------

/// The runtime log of the instance `name` of a case, where its box writes what it says.
fn runtime_log(case: &Case, name: &str) -> PathBuf {
    placement::sessions_dir(&case.dir.join(name).join("state")).join(placement::RUNTIME_LOG)
}

fn append(file: &Path, text: &str) {
    use std::io::Write as _;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .expect("append to the runtime log");
}

#[test]
fn a_boxed_runtime_that_exits_within_the_start_bound_is_a_failed_start_with_its_error() {
    use conductor_model::instance::ReconcileReason;
    use std::time::Duration;
    let case = Case::new("failed-start");
    case.config(&[("alpha", "systemd")]);
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    let start = std::time::Instant::now();
    let log = runtime_log(&case, "alpha");
    fs::create_dir_all(log.parent().expect("the sessions directory")).expect("create it");
    append(&log, "conductor: an older line of another start\n");
    let done = daemon
        .pass(ReconcileReason::ConfigChange, start)
        .expect("a pass");
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");

    // The runtime says why and exits; the daemon's next tick, 2 s after the start, finds no
    // session.
    append(
        &log,
        "conductor: instance serve: the store does not open: example failure\n",
    );
    fake.log.lock().expect("the log").running.clear();
    let at = start + Duration::from_secs(2);
    assert_eq!(
        daemon.due(at).expect("due"),
        Some(ReconcileReason::LostRuntime)
    );
    let done = daemon
        .pass(ReconcileReason::LostRuntime, at)
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(
        (done[0].action, done[0].reason, done[0].result),
        (
            ReconcileAction::Start,
            ReconcileReason::LostRuntime,
            ReconcileResult::Refused
        ),
        "a failed start, not a start that succeeded: {done:?}"
    );
    let detail = done[0].detail.clone().unwrap_or_default();
    assert!(
        detail.contains("the store does not open: example failure"),
        "the record carries the runtime's error text: {detail}"
    );
    assert!(
        !detail.contains("an older line"),
        "only what this start wrote: {detail}"
    );
    assert_eq!(
        fake.log.lock().expect("the log").opened.len(),
        1,
        "nothing is started in the same pass"
    );
    let rows = daemon.rows();
    assert_eq!(rows[0]["state"], "Unreachable", "{rows:?}");
    assert!(!daemon.routes().contains_key("alpha"));
    let recorded = ReconciliationStorage::list(daemon.store());
    assert!(
        recorded
            .iter()
            .any(|row| row.data.result == ReconcileResult::Refused
                && row
                    .data
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("example failure"))),
        "the failed start is in the store"
    );

    // While it waits no pass starts or records anything, and no pass is due for it.
    let waiting = start + Duration::from_secs(4);
    assert_eq!(daemon.due(waiting).expect("due"), None);
    let done = daemon
        .pass(ReconcileReason::Cadence, waiting)
        .expect("a pass");
    assert!(done.is_empty(), "{done:?}");
    assert_eq!(daemon.rows()[0]["state"], "Unreachable");

    // The wait over, a pass is due and starts it again; one that then runs is Running.
    let retry = start + Duration::from_secs(12);
    assert_eq!(
        daemon.due(retry).expect("due"),
        Some(ReconcileReason::LostRuntime)
    );
    let done = daemon
        .pass(ReconcileReason::LostRuntime, retry)
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    assert_eq!(fake.log.lock().expect("the log").opened.len(), 2);
    assert_eq!(daemon.rows()[0]["state"], "Running");

    // A runtime lost after the bound is no failed start: it is started again at once.
    fake.log.lock().expect("the log").running.clear();
    let later = retry + Duration::from_secs(60);
    let done = daemon
        .pass(ReconcileReason::LostRuntime, later)
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    assert_eq!(fake.log.lock().expect("the log").opened.len(), 3);
}

#[test]
fn a_crash_looping_boxed_runtime_is_started_ever_less_often() {
    use std::time::Duration;
    let case = Case::new("crash-loop");
    case.config(&[("alpha", "systemd")]);
    let fake = Fake::default();
    let mut daemon = case.daemon(fake.clone());
    let start = std::time::Instant::now();
    // conductord's loop: one tick every 2 s; every runtime exits before the next tick.
    let mut opened_at = Vec::new();
    for tick in 0..900_u64 {
        let before = fake.log.lock().expect("the log").opened.len();
        daemon
            .tick(start + Duration::from_secs(tick * 2))
            .expect("a tick");
        let mut log = fake.log.lock().expect("the log");
        let started = log.opened.len() > before;
        if started {
            opened_at.push(tick * 2);
        }
        log.running.clear();
        drop(log);
        // Between two starts the instance never reads as running.
        if !started {
            assert_ne!(
                daemon.rows()[0]["state"],
                "Running",
                "at {} s: {opened_at:?}",
                tick * 2
            );
        }
    }
    let first_four_minutes = opened_at.iter().filter(|at| **at < 240).count();
    assert!(
        (2..=6).contains(&first_four_minutes),
        "the starts back off: {opened_at:?}"
    );
    // The delay doubles to a cap, so it keeps being started.
    let last_ten_minutes = opened_at.iter().filter(|at| **at >= 1200).count();
    assert!(
        (1..=3).contains(&last_ten_minutes),
        "the delay is capped: {opened_at:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// story:box-crash-loop-reads-failed: doctor dry-runs the box
// ---------------------------------------------------------------------------------------------

/// What a `systemd` box requires its daemon to prove.
const SYSTEMD_PROPERTIES: [Property; 3] = [
    Property::WholeTreeKill,
    Property::IpcSocketAllowlist,
    Property::IpcSignalsContained,
];

fn systemd_families() -> Vec<Family> {
    let mut families = LINUX_FAMILIES.to_vec();
    families.push(Family::Limits);
    families
}

impl Case {
    /// `conductor doctor` for `file` with the local substrate daemon at `socket`.
    fn doctor_on(&self, file: &Path, socket: &Path) -> Output {
        common::conductor(self.home())
            .arg("doctor")
            .current_dir(self.home().join("work"))
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.dir.join("bin"))
            .env("CONDUCTOR_CONFIG", file)
            .env(placement::SOCKET_VARIABLE, socket)
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// The three directories of the instance `name` that its config names.
    fn directories(&self, name: &str) {
        for sub in ["state", "records", "cache"] {
            fs::create_dir_all(self.dir.join(name).join(sub)).expect("an instance directory");
        }
    }
}

/// Every file and directory under `dir`, sorted.
fn tree(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut left = vec![dir.to_path_buf()];
    while let Some(dir) = left.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() && !path.is_symlink() {
                left.push(path.clone());
            }
            found.push(path);
        }
    }
    found.sort();
    found
}

#[test]
fn doctor_is_not_ready_for_a_box_whose_substrate_daemon_does_not_answer() {
    let case = Case::new("doctor-box-silent");
    let file = case.config(&[("alpha", "systemd")]);
    case.directories("alpha");
    let output = case.doctor_on(&file, &case.dir.join("no.sock"));
    let lines = lines(&output);
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("box alpha: placement.substrate-unreachable")),
        "{}",
        describe(&output)
    );
    let verdict = lines.last().cloned().unwrap_or_default();
    assert!(
        verdict.starts_with("not ready: ")
            && verdict.contains("box alpha (placement.substrate-unreachable)"),
        "{}",
        describe(&output)
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
}

#[test]
fn doctor_dry_runs_a_box_against_the_substrate_daemon_and_starts_nothing() {
    let case = Case::new("dbox");
    let file = case.config(&[("alpha", "systemd"), ("beta", "box-b")]);
    case.directories("alpha");
    let before = tree(&case.dir);

    // The daemon serves the box's facts and three of its four roots: refused as it would be.
    let socket = case.dir.join("a.sock");
    let fake = FakeLocal::opening(
        &socket,
        &systemd_families(),
        &SYSTEMD_PROPERTIES,
        &["state-alpha", "records-alpha", "cache-alpha"],
    );
    let output = case.doctor_on(&file, &socket);
    let verdict = lines(&output).last().cloned().unwrap_or_default();
    assert!(
        verdict.contains("box alpha (exec.writable-roots-host-unserved)"),
        "{}",
        describe(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("--writable-root store="),
        "the row says what to declare: {}",
        describe(&output)
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let asked = fake.lines.lock().expect("the lines").clone();
    assert!(
        !asked.is_empty()
            && asked
                .iter()
                .all(|line| line.starts_with("GET /v1/machine ")),
        "only the machine is asked: {asked:?}"
    );
    fs::remove_file(&socket).expect("remove the socket");

    // It serves all four: the box is no failure, nothing was started and nothing written.
    let socket = case.dir.join("b.sock");
    let fake = FakeLocal::opening(
        &socket,
        &systemd_families(),
        &SYSTEMD_PROPERTIES,
        &["state-alpha", "records-alpha", "cache-alpha", "store"],
    );
    let output = case.doctor_on(&file, &socket);
    let shown = lines(&output);
    assert!(
        shown.iter().any(
            |line| line.starts_with("box alpha: systemd") && line.contains("4 writable roots")
        ),
        "{}",
        describe(&output)
    );
    assert!(
        !shown.last().is_some_and(|verdict| verdict.contains("box ")),
        "{}",
        describe(&output)
    );
    assert!(
        !shown.iter().any(|line| line.starts_with("box beta")),
        "an instance in no box has no box row: {}",
        describe(&output)
    );
    let asked = fake.lines.lock().expect("the lines").clone();
    assert!(
        !asked.is_empty()
            && asked
                .iter()
                .all(|line| line.starts_with("GET /v1/machine ")),
        "no workspace and no session is asked for: {asked:?}"
    );
    assert!(fake.starts.lock().expect("the starts").is_empty());
    fs::remove_file(&socket).expect("remove the socket");
    assert_eq!(tree(&case.dir), before, "doctor writes nothing");

    // A root the box writes is not there: named before substrate would refuse it.
    fs::remove_dir_all(case.dir.join("alpha/records")).expect("remove the records");
    let socket = case.dir.join("c.sock");
    let _fake = FakeLocal::opening(
        &socket,
        &systemd_families(),
        &SYSTEMD_PROPERTIES,
        &["state-alpha", "records-alpha", "cache-alpha", "store"],
    );
    let output = case.doctor_on(&file, &socket);
    let verdict = lines(&output).last().cloned().unwrap_or_default();
    assert!(
        verdict.contains("box alpha (placement.writable-root-missing)"),
        "{}",
        describe(&output)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&case.dir.join("alpha/records").display().to_string()),
        "the row names the directory: {}",
        describe(&output)
    );
}

#[test]
fn doctor_names_a_box_conductord_would_refuse_before_it_asks_substrate() {
    let case = Case::new("dbref");
    // The store beside the config file: the store's root would hold the config file.
    let text = format!(
        "version: conductor.config/2\ndaemon: {{port: 7300, store: {store:?}}}\ninstances:\n  \
         - name: alpha\n    placement: systemd\n    session_prefix: alpha\n    \
         sources: [{{github: example-org}}]\n    \
         checkouts: {{root: {root:?}, trees: {trees:?}}}\n    records: {records:?}\n    \
         state: {state:?}\n    cache: {cache:?}\n",
        store = format!("sqlite:{}", case.dir.join("shared.sqlite").display()),
        root = case.dir.join("alpha/checkouts"),
        trees = case.dir.join("alpha/trees"),
        records = case.dir.join("alpha/records"),
        state = case.dir.join("alpha/state"),
        cache = case.dir.join("alpha/cache"),
    );
    fs::write(case.file(), text).expect("write the config file");
    case.directories("alpha");
    let socket = case.dir.join("s.sock");
    let fake = FakeLocal::opening(
        &socket,
        &systemd_families(),
        &SYSTEMD_PROPERTIES,
        &["state-alpha", "records-alpha", "cache-alpha", "store"],
    );
    let output = case.doctor_on(&case.file(), &socket);
    let verdict = lines(&output).last().cloned().unwrap_or_default();
    assert!(
        verdict.contains("box alpha (placement.writable-root-shared)"),
        "{}",
        describe(&output)
    );
    assert!(
        fake.lines.lock().expect("the lines").is_empty(),
        "substrate is not asked for a box conductord refuses itself"
    );
}

// ---------------------------------------------------------------------------------------------
// Adversary pass 1 on story:box-crash-loop-reads-failed (uncommitted)
// ---------------------------------------------------------------------------------------------

mod adversary {
    use super::*;
    use conductor_cli::daemon::reconcile::{RETRY_CAP, START_BOUND, retry_after};
    use conductor_model::instance::ReconcileReason;
    use std::time::{Duration, Instant};

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    /// A daemon whose boxed `alpha` was started at `start` and whose start failed, found at
    /// `start + 2 s`: the pass that recorded it is answered.
    fn failed_once(
        case: &Case,
        fake: &Fake,
        start: Instant,
    ) -> (Daemon<Units0>, Vec<conductor_cli::daemon::reconcile::Done>) {
        let mut daemon = case.daemon(fake.clone());
        let done = daemon
            .pass(ReconcileReason::ConfigChange, start)
            .expect("a pass");
        assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(2))
            .expect("a pass");
        assert_eq!(done.len(), 1, "{done:?}");
        assert_eq!(done[0].result, ReconcileResult::Refused, "{done:?}");
        (daemon, done)
    }

    // ---- red: the bound ------------------------------------------------------------------

    /// The host page: "A runtime that exits within 10 s of its start is a failed start." The
    /// daemon looks every 2 s, so a runtime that exits 9 s after its start is found at 10 s.
    #[test]
    fn a_runtime_that_exits_nine_seconds_after_its_start_is_a_failed_start() {
        let case = Case::new("adv-nine");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let mut daemon = case.daemon(fake.clone());
        let start = Instant::now();
        for at in [0, 2, 4, 6, 8] {
            daemon.tick(start + secs(at)).expect("a tick");
        }
        assert_eq!(fake.log.lock().expect("the log").opened.len(), 1);
        // It exits 9 s after its start; the next tick is the one at 10 s.
        fake.log.lock().expect("the log").running.clear();
        let done = daemon.tick(start + secs(10)).expect("a tick");
        assert_eq!(done.len(), 1, "{done:?}");
        assert_eq!(
            done[0].result,
            ReconcileResult::Refused,
            "a runtime that exited 9 s after its start is a failed start, not a start that \
             succeeded: {done:?}"
        );
        assert_eq!(
            fake.log.lock().expect("the log").opened.len(),
            1,
            "and it is not started again at once"
        );
    }

    /// The acceptance: `/v1/instances` answers a runtime whose start failed as not Running.
    /// The start after a failed one has not held the start bound yet.
    #[test]
    fn the_start_after_a_failed_start_does_not_read_running_at_once() {
        let case = Case::new("adv-retry-state");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let start = Instant::now();
        let (mut daemon, _) = failed_once(&case, &fake, start);
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(12))
            .expect("a pass");
        assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
        // The runtime exits at once again, as it did before; nothing looked yet.
        fake.log.lock().expect("the log").running.clear();
        assert_ne!(
            daemon.rows()[0]["state"],
            "Running",
            "a crash-looping runtime reads Running after each of its starts"
        );
    }

    // ---- red: the log the box controls ---------------------------------------------------

    /// `<state>/sessions/runtime.log` is inside a root the box writes. A link there must not
    /// make the daemon, which runs in no box, copy another file of the host into the record.
    #[test]
    fn a_failed_start_does_not_record_a_host_file_its_log_links_to() {
        let case = Case::new("adv-link");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let mut daemon = case.daemon(fake.clone());
        let start = Instant::now();
        daemon
            .pass(ReconcileReason::ConfigChange, start)
            .expect("a pass");
        // The box sees none of this file: it is in no root of the box.
        let secret = case.dir.join("outside-the-box.txt");
        fs::write(&secret, "host-only-value-0123456789\n").expect("write the file");
        let log = runtime_log(&case, "alpha");
        fs::create_dir_all(log.parent().expect("the sessions directory")).expect("create it");
        let _ = fs::remove_file(&log);
        std::os::unix::fs::symlink(&secret, &log).expect("link the log");
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(2))
            .expect("a pass");
        assert_eq!(done[0].result, ReconcileResult::Refused, "{done:?}");
        let detail = done[0].detail.clone().unwrap_or_default();
        assert!(
            !detail.contains("host-only-value-0123456789"),
            "the record carries a file of the host the box cannot read: {detail}"
        );
    }

    /// A named pipe in place of the log must not stop conductord's loop.
    #[test]
    fn a_failed_start_whose_log_is_a_named_pipe_does_not_hang_the_pass() {
        let case = Case::new("adv-pipe");
        case.config(&[("alpha", "systemd")]);
        let log = runtime_log(&case, "alpha");
        let pipe = log.clone();
        let (sent, received) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let fake = Fake::default();
            let mut daemon = case.daemon(fake.clone());
            let start = Instant::now();
            daemon
                .pass(ReconcileReason::ConfigChange, start)
                .expect("a pass");
            fs::create_dir_all(pipe.parent().expect("the sessions directory")).expect("create it");
            let _ = fs::remove_file(&pipe);
            let made = std::process::Command::new("mkfifo")
                .arg(&pipe)
                .status()
                .expect("mkfifo runs");
            assert!(made.success(), "mkfifo");
            fake.log.lock().expect("the log").running.clear();
            let done = daemon.pass(ReconcileReason::LostRuntime, start + secs(2));
            let _ = sent.send(done.map(|done| done.len()).map_err(|error| format!("{error:#}")));
        });
        let answered = received.recv_timeout(secs(20));
        // Let the pass go, whatever it waits for: a writer that opens the pipe and closes it.
        let writer = log.clone();
        std::thread::spawn(move || {
            let _ = fs::OpenOptions::new().write(true).open(writer);
        });
        assert!(
            answered.is_ok(),
            "the pass did not return within 20 s: it waits to open the pipe {}",
            log.display()
        );
    }

    /// What the runtime wrote goes to conductord's standard output and the record on one line:
    /// no escape sequence of the runtime's output reaches a terminal through it.
    #[test]
    fn a_failed_start_carries_no_control_character_of_the_log() {
        let case = Case::new("adv-escape");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let mut daemon = case.daemon(fake.clone());
        let start = Instant::now();
        daemon
            .pass(ReconcileReason::ConfigChange, start)
            .expect("a pass");
        let log = runtime_log(&case, "alpha");
        fs::create_dir_all(log.parent().expect("the sessions directory")).expect("create it");
        append(
            &log,
            "conductor: instance serve: \x1b[2J\x1b]0;title\x07 the store\rdoes not open\n",
        );
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(2))
            .expect("a pass");
        let detail = done[0].detail.clone().unwrap_or_default();
        assert!(detail.contains("does not open"), "{detail:?}");
        assert!(
            !detail.chars().any(char::is_control),
            "a control character of the runtime's log is in the record: {detail:?}"
        );
    }

    // ---- red: the dead box held open -----------------------------------------------------

    /// The dead box stays open while its runtime waits for its next start. An instance taken
    /// out of the config in that wait is the operator's answer to a crash loop: its box closes.
    #[test]
    fn an_instance_removed_while_its_failed_start_waits_has_its_box_closed() {
        let case = Case::new("adv-removed");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let start = Instant::now();
        let (mut daemon, _) = failed_once(&case, &fake, start);
        assert!(fake.log.lock().expect("the log").closed.is_empty());
        case.config(&[("beta", "box-b")]);
        daemon
            .pass(ReconcileReason::ConfigChange, start + secs(4))
            .expect("a pass");
        // Every later pass and tick, far past the longest wait.
        for at in [6, 8, 400, 800] {
            daemon.tick(start + secs(at)).expect("a tick");
        }
        assert_eq!(
            fake.log.lock().expect("the log").closed,
            ["ws_1".to_owned()],
            "the box of the removed instance is still open on substrate"
        );
    }

    // ---- red: doctor ---------------------------------------------------------------------

    /// A substrate daemon whose process is stopped: the kernel takes the connection, nothing
    /// answers. Doctor says so; it does not wait for ever.
    #[test]
    fn doctor_ends_when_the_substrate_socket_takes_the_connection_and_never_answers() {
        let case = Case::new("dmu");
        let file = case.config(&[("alpha", "systemd")]);
        case.directories("alpha");
        let socket = case.dir.join("m.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).expect("bind the socket");
        let mut child = common::conductor(case.home())
            .arg("doctor")
            .current_dir(case.home().join("work"))
            .env_clear()
            .env("HOME", case.home())
            .env("PATH", case.dir.join("bin"))
            .env("CONDUCTOR_CONFIG", &file)
            .env(placement::SOCKET_VARIABLE, &socket)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("conductor runs");
        let until = Instant::now() + secs(45);
        let ended = loop {
            if let Some(status) = child.try_wait().expect("ask the child") {
                break Some(status);
            }
            if Instant::now() >= until {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        drop(listener);
        assert!(
            ended.is_some(),
            "conductor doctor did not end within 45 s on a substrate socket that never answers"
        );
        assert_eq!(ended.and_then(|status| status.code()), Some(1));
    }

    /// The roots of a box on a remote daemon are directories of that daemon's host. Doctor
    /// does not ask the remote daemon, and it does not look for them on this host either.
    #[test]
    fn doctor_does_not_look_on_this_host_for_the_roots_of_a_box_on_a_remote_daemon() {
        let case = Case::new("adv-remote-roots");
        let file = case.config(&[("alpha", &case.remote_placement(REMOTE))]);
        let output = case.doctor_on(&file, &case.dir.join("no.sock"));
        let shown = lines(&output);
        let row = shown
            .iter()
            .find(|line| line.starts_with("box alpha:"))
            .cloned()
            .unwrap_or_else(|| panic!("no box row: {}", describe(&output)));
        assert!(
            !row.contains("placement.writable-root-missing"),
            "a directory of the remote host is asked for on this one: {row}"
        );
    }

    // ---- green: cases for mutants the unit's suite does not catch ---------------------------

    /// Mutants of `retry_after`: no cap, another cap, a shift off by one. The unit's crash-loop
    /// case passes with the cap removed (it looks at 30 minutes; the sixth wait is the first
    /// the cap cuts).
    #[test]
    fn the_wait_doubles_from_the_start_bound_and_stops_at_the_cap() {
        assert_eq!(START_BOUND, secs(10));
        assert_eq!(RETRY_CAP, secs(300));
        let waits: Vec<u64> = [0, 1, 2, 3, 4, 5, 6, 7, 31, 32, 33, u32::MAX]
            .into_iter()
            .map(|failures| retry_after(failures).as_secs())
            .collect();
        assert_eq!(waits, [10, 10, 20, 40, 80, 160, 300, 300, 300, 300, 300, 300]);
    }

    /// The same through the daemon: over two hours of a crash loop no two starts are further
    /// apart than the cap and one tick.
    #[test]
    fn a_crash_loop_is_started_at_least_every_cap() {
        let case = Case::new("adv-cap");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let mut daemon = case.daemon(fake.clone());
        let start = Instant::now();
        let mut opened_at = Vec::new();
        for tick in 0..3600_u64 {
            let before = fake.log.lock().expect("the log").opened.len();
            daemon.tick(start + secs(tick * 2)).expect("a tick");
            let mut log = fake.log.lock().expect("the log");
            if log.opened.len() > before {
                opened_at.push(tick * 2);
            }
            log.running.clear();
        }
        let gaps: Vec<u64> = opened_at.windows(2).map(|pair| pair[1] - pair[0]).collect();
        assert_eq!(gaps[..6], [12, 22, 42, 82, 162, 302], "{opened_at:?}");
        assert!(gaps[6..].iter().all(|gap| *gap == 302), "{gaps:?}");
    }

    /// Mutant: `now < retry_at && !asked` without `!asked`. A changed config ends the wait.
    #[test]
    fn a_config_change_pass_starts_a_runtime_that_waits() {
        let case = Case::new("adv-asked");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let start = Instant::now();
        let (mut daemon, _) = failed_once(&case, &fake, start);
        let done = daemon
            .pass(ReconcileReason::Cadence, start + secs(4))
            .expect("a pass");
        assert!(done.is_empty(), "{done:?}");
        let done = daemon
            .pass(ReconcileReason::ConfigChange, start + secs(6))
            .expect("a pass");
        assert_eq!(done.len(), 1, "{done:?}");
        assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
        let log = fake.log.lock().expect("the log");
        assert_eq!(log.opened.len(), 2);
        assert_eq!(log.closed, ["ws_1".to_owned()], "the dead box is closed first");
    }

    /// Mutant: the wait of a failed start found by a config-change pass (`Duration::ZERO`).
    #[test]
    fn a_failed_start_found_by_a_config_change_pass_is_started_at_the_next_tick() {
        let case = Case::new("adv-asked-first");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let mut daemon = case.daemon(fake.clone());
        let start = Instant::now();
        daemon
            .pass(ReconcileReason::ConfigChange, start)
            .expect("a pass");
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::ConfigChange, start + secs(2))
            .expect("a pass");
        assert_eq!(done[0].result, ReconcileResult::Refused, "{done:?}");
        let detail = done[0].detail.clone().unwrap_or_default();
        assert!(detail.contains("the next start in 0 s"), "{detail}");
        let done = daemon.tick(start + secs(4)).expect("a tick");
        assert_eq!(done.len(), 1, "{done:?}");
        assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    }

    /// Mutant: `attempt.failures = 0` dropped. A runtime that ran past the bound ends the row.
    #[test]
    fn a_runtime_that_ran_past_the_bound_ends_the_row_of_failed_starts() {
        let case = Case::new("adv-row");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let start = Instant::now();
        let (mut daemon, done) = failed_once(&case, &fake, start);
        let detail = done[0].detail.clone().unwrap_or_default();
        assert!(
            detail.contains("failed start 1 in a row, the next start in 10 s"),
            "{detail}"
        );
        // The second start fails too: the second in a row, 20 s.
        daemon
            .pass(ReconcileReason::LostRuntime, start + secs(12))
            .expect("a pass");
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(14))
            .expect("a pass");
        let detail = done[0].detail.clone().unwrap_or_default();
        assert!(
            detail.contains("failed start 2 in a row, the next start in 20 s"),
            "{detail}"
        );
        // The third runs a minute and stops: started again at once.
        daemon
            .pass(ReconcileReason::LostRuntime, start + secs(34))
            .expect("a pass");
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(94))
            .expect("a pass");
        assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
        // The fourth fails: the first in a row again.
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(96))
            .expect("a pass");
        let detail = done[0].detail.clone().unwrap_or_default();
        assert!(
            detail.contains("failed start 1 in a row, the next start in 10 s"),
            "{detail}"
        );
    }

    /// Mutants of `runtime_said`: the line bound, the byte bound, and a log that was replaced
    /// by a shorter one since the start.
    #[test]
    fn the_record_carries_the_last_eight_lines_of_at_most_4096_bytes_and_a_replaced_log() {
        let case = Case::new("adv-said");
        case.config(&[("alpha", "systemd")]);
        let fake = Fake::default();
        let mut daemon = case.daemon(fake.clone());
        let start = Instant::now();
        let log = runtime_log(&case, "alpha");
        fs::create_dir_all(log.parent().expect("the sessions directory")).expect("create it");
        append(&log, &"an older start wrote this line\n".repeat(40));
        daemon
            .pass(ReconcileReason::ConfigChange, start)
            .expect("a pass");
        for n in 1..=20 {
            append(&log, &format!("line-{n:02}\n\n"));
        }
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(2))
            .expect("a pass");
        let detail = done[0].detail.clone().unwrap_or_default();
        assert!(
            detail.ends_with(
                ": line-13 | line-14 | line-15 | line-16 | line-17 | line-18 | line-19 | line-20"
            ),
            "{detail}"
        );

        // The second start: the log is replaced by a shorter one of one long line.
        daemon
            .pass(ReconcileReason::LostRuntime, start + secs(12))
            .expect("a pass");
        fs::write(&log, format!("{}tail-of-the-line\n", "x".repeat(10_000))).expect("replace it");
        assert!(fs::metadata(&log).expect("the log").len() > 4096);
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(14))
            .expect("a pass");
        let detail = done[0].detail.clone().unwrap_or_default();
        assert!(detail.ends_with("tail-of-the-line"), "{detail}");
        let said = detail.rsplit(": ").next().unwrap_or_default();
        assert_eq!(said.len(), 4095, "the last 4096 bytes, less the newline");

        // The third start: replaced by a log shorter than it was at the start.
        daemon
            .pass(ReconcileReason::LostRuntime, start + secs(34))
            .expect("a pass");
        fs::write(&log, "short\n").expect("replace it");
        fake.log.lock().expect("the log").running.clear();
        let done = daemon
            .pass(ReconcileReason::LostRuntime, start + secs(36))
            .expect("a pass");
        let detail = done[0].detail.clone().unwrap_or_default();
        assert!(detail.ends_with(": short"), "{detail}");
    }

    /// Mutant: `unmet` dropped from `Sdk::unserved`, seen through doctor; and `roots_missing`
    /// for a root that is a file and one that is a link to nothing.
    #[test]
    fn doctor_names_an_unserved_family_and_a_root_that_is_no_directory() {
        let case = Case::new("adv-doc");
        let file = case.config(&[("alpha", "systemd")]);
        case.directories("alpha");
        let all = ["state-alpha", "records-alpha", "cache-alpha", "store"];

        // No `limits` family: the systemd adapter requires it.
        let socket = case.dir.join("a.sock");
        let fake = FakeLocal::opening(&socket, &LINUX_FAMILIES, &SYSTEMD_PROPERTIES, &all);
        let output = case.doctor_on(&file, &socket);
        let verdict = lines(&output).last().cloned().unwrap_or_default();
        assert!(
            verdict.contains("box alpha (confinement.family-unserved)"),
            "{}",
            describe(&output)
        );
        assert!(fake.starts.lock().expect("the starts").is_empty());
        fs::remove_file(&socket).expect("remove the socket");

        // The cache is a file.
        let socket = case.dir.join("b.sock");
        let fake = FakeLocal::opening(&socket, &systemd_families(), &SYSTEMD_PROPERTIES, &all);
        let cache = case.dir.join("alpha/cache");
        fs::remove_dir_all(&cache).expect("remove the cache");
        fs::write(&cache, "").expect("a file");
        let output = case.doctor_on(&file, &socket);
        let verdict = lines(&output).last().cloned().unwrap_or_default();
        assert!(
            verdict.contains("box alpha (placement.writable-root-missing)"),
            "{}",
            describe(&output)
        );
        assert!(
            fake.lines.lock().expect("the lines").is_empty(),
            "substrate is not asked for a box whose root is missing"
        );

        // The cache is a link to nothing.
        fs::remove_file(&cache).expect("remove the file");
        std::os::unix::fs::symlink(case.dir.join("nowhere"), &cache).expect("a link");
        let before = tree(&case.dir);
        let output = case.doctor_on(&file, &socket);
        let verdict = lines(&output).last().cloned().unwrap_or_default();
        assert!(
            verdict.contains("box alpha (placement.writable-root-missing)"),
            "{}",
            describe(&output)
        );
        assert_eq!(tree(&case.dir), before, "doctor writes nothing");
    }
}
