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
    /// The daemon answers nothing: every question is an error (a silent remote).
    silent: bool,
    /// The remote daemons the box opener was built for, by URL.
    remotes: Vec<String>,
    /// The relay sessions it was asked to run.
    exchanges: Vec<placement::Exchange>,
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
