//! `conductor daemon serve` (`story:daemon-reconcile`, design architecture-next § 4): the daemon
//! reconciles the instance runtimes from the config file, records one `Reconciliation` per action
//! (and none for a pass with nothing to do), reconciles on a config change, on a lost runtime and
//! every `daemon.cadence.reconcile`, serves `GET /v1/instances`, and installs as
//! `conductord.service`. Every case runs against a fake service manager ([`Fake`]); none touches
//! systemd.

mod active;
mod common;

use std::collections::BTreeSet;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use conductor_cli::daemon::reconcile::{BINARIES_FILE, Binary, Daemon, RUNTIMES_FILE, unit_name};
use conductor_cli::daemon::units::Units;
use conductor_cli::daemon::{api, install};
use conductor_cli::systemd::Service;
use conductor_model::behaviour::ReconciliationStorage;
use conductor_model::instance::{ReconcileAction, ReconcileReason, ReconcileResult};
use serde_json::Value;

/// The service manager of a case: what runs, every call made, and the units it refuses to start.
#[derive(Debug, Default, Clone)]
struct Fake {
    running: BTreeSet<String>,
    calls: Vec<String>,
    refuse: BTreeSet<String>,
    envs: Vec<Vec<String>>,
}

impl Units for Fake {
    fn start(&mut self, service: &Service) -> Result<()> {
        if self.refuse.contains(&service.unit) {
            bail!("refused {}", service.unit);
        }
        self.calls.push(format!(
            "start {} {}",
            service.unit,
            service.command.join(" ")
        ));
        self.envs.push(service.env.clone());
        self.running.insert(service.unit.clone());
        Ok(())
    }

    fn stop(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("stop {unit}"));
        self.running.remove(unit);
        Ok(())
    }

    fn active(&mut self, unit: &str) -> Result<bool> {
        Ok(self.running.contains(unit))
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

/// One case's directory: its home, its config file and the daemon's directory.
struct Case {
    dir: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("daemon")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// The case's shared database: a temporary SQLite file, the `daemon.store` of [`Case::config`].
    fn url(&self) -> String {
        format!("sqlite:{}", self.dir.join("shared.sqlite").display())
    }

    /// Writes the config file: `daemon` as given with [`Case::url`] as its `store`, and one
    /// instance per `(name, placement)`.
    fn config(&self, daemon: &str, instances: &[(&str, &str)]) {
        let daemon = daemon.replacen('{', &format!("{{store: {:?}, ", self.url()), 1);
        self.config_as(&daemon, instances);
    }

    /// [`Case::config`] with `daemon` written as given, no `store` added.
    fn config_as(&self, daemon: &str, instances: &[(&str, &str)]) {
        let mut text = format!("version: conductor.config/2\ndaemon: {daemon}\ninstances:\n");
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
    }

    fn daemon(&self, units: Fake) -> Daemon<Fake> {
        Daemon::open(
            self.file(),
            self.home(),
            self.dir.join("daemon"),
            binary("1.0.0"),
            units,
        )
        .expect("open the daemon")
    }
}

fn binary(version: &str) -> Binary {
    Binary {
        path: PathBuf::from("/opt/conductor/bin/conductor"),
        version: version.to_owned(),
    }
}

/// The reconciliations in the daemon's store.
fn recorded(daemon: &Daemon<Fake>) -> usize {
    ReconciliationStorage::list(daemon.store()).len()
}

fn actions(done: &[conductor_cli::daemon::reconcile::Done]) -> Vec<(String, ReconcileAction)> {
    done.iter()
        .map(|done| (done.instance.clone(), done.action))
        .collect()
}

#[test]
fn a_pass_starts_every_absent_instance_and_a_second_pass_records_nothing() {
    let case = Case::new("start");
    case.config("{port: 7300}", &[("alpha", "box-a"), ("beta", "box-b")]);
    let mut daemon = case.daemon(Fake::default());
    let now = Instant::now();
    let done = daemon.pass(ReconcileReason::Cadence, now).expect("pass");
    assert_eq!(
        actions(&done),
        [
            ("alpha".to_owned(), ReconcileAction::Start),
            ("beta".to_owned(), ReconcileAction::Start)
        ]
    );
    assert!(
        done.iter()
            .all(|done| done.result == ReconcileResult::Succeeded)
    );
    assert_eq!(recorded(&daemon), 2);
    for name in ["alpha", "beta"] {
        let unit = unit_name(name);
        assert!(daemon.units.running.contains(&unit), "{unit}");
        let started = format!(
            "start {unit} /opt/conductor/bin/conductor --config {} instance serve",
            case.file().display()
        );
        assert!(
            daemon.units.calls.contains(&started),
            "{:?}",
            daemon.units.calls
        );
    }
    let rows: Vec<_> = ReconciliationStorage::list(daemon.store())
        .into_iter()
        .map(|row| (row.data.action, row.data.reason, row.data.result))
        .collect();
    assert!(rows.iter().all(|row| *row
        == (
            ReconcileAction::Start,
            ReconcileReason::Cadence,
            ReconcileResult::Succeeded
        )));

    let calls = daemon.units.calls.len();
    let again = daemon.pass(ReconcileReason::Cadence, now).expect("pass");
    assert!(again.is_empty(), "{again:?}");
    assert_eq!(
        recorded(&daemon),
        2,
        "a pass with nothing changed records nothing"
    );
    assert_eq!(daemon.units.calls.len(), calls);
}

/// `story:instance-api`: each runtime is started with a token of its own and the socket it serves,
/// and the API's routes name each running runtime at that socket with that token; a restart mints
/// a new token.
#[test]
fn each_runtime_gets_its_own_token_and_socket_and_the_routes_name_them() {
    let case = Case::new("routes");
    case.config("{port: 7300}", &[("alpha", "box-a"), ("beta", "box-b")]);
    let mut daemon = case.daemon(Fake::default());
    daemon
        .pass(ReconcileReason::ConfigChange, Instant::now())
        .expect("pass");
    let routes = daemon.routes();
    assert_eq!(routes.len(), 2, "{routes:?}");
    let env = |index: usize, variable: &str| {
        daemon.units.envs[index]
            .iter()
            .find_map(|line| line.strip_prefix(&format!("{variable}=")))
            .unwrap_or_else(|| panic!("no {variable}: {:?}", daemon.units.envs[index]))
            .to_owned()
    };
    for (index, name) in ["alpha", "beta"].into_iter().enumerate() {
        let route = &routes[name];
        let state = case.dir.join(name).join("state");
        assert_eq!(route.socket, conductor_cli::instance::socket(&state));
        assert_eq!(route.token.len(), 64, "{route:?}");
        assert_eq!(
            env(index, conductor_cli::instance::TOKEN_VARIABLE),
            route.token
        );
        assert_eq!(
            env(index, conductor_cli::instance::SOCKET_VARIABLE),
            route.socket.display().to_string()
        );
    }
    assert_ne!(routes["alpha"].token, routes["beta"].token);

    case.config("{port: 7300}", &[("alpha", "box-c"), ("beta", "box-b")]);
    daemon
        .pass(ReconcileReason::ConfigChange, Instant::now())
        .expect("pass");
    assert_ne!(daemon.routes()["alpha"].token, routes["alpha"].token);
    daemon.units.running.clear();
    daemon.due(Instant::now()).expect("due");
    assert!(daemon.routes().is_empty(), "a stopped runtime is no route");
}

#[test]
fn an_instance_gone_from_the_config_is_stopped() {
    let case = Case::new("stop");
    case.config("{port: 7300}", &[("alpha", "box-a"), ("beta", "box-b")]);
    let mut daemon = case.daemon(Fake::default());
    let now = Instant::now();
    daemon.pass(ReconcileReason::Cadence, now).expect("pass");
    case.config("{port: 7300}", &[("alpha", "box-a")]);
    let done = daemon
        .pass(ReconcileReason::ConfigChange, now)
        .expect("pass");
    assert_eq!(actions(&done), [("beta".to_owned(), ReconcileAction::Stop)]);
    assert_eq!(done[0].reason, ReconcileReason::ConfigChange);
    assert!(!daemon.units.running.contains(&unit_name("beta")));
    assert!(daemon.units.running.contains(&unit_name("alpha")));
    assert_eq!(recorded(&daemon), 3);
    assert!(!daemon.started().contains_key("beta"));
    assert!(
        daemon
            .pass(ReconcileReason::Cadence, now)
            .expect("pass")
            .is_empty()
    );
    assert_eq!(recorded(&daemon), 3);
}

#[test]
fn a_changed_placement_or_desired_binary_restarts_the_runtime_and_the_daemons_binary_does_not() {
    let case = Case::new("restart");
    case.config("{port: 7300}", &[("alpha", "box-a"), ("beta", "box-b")]);
    let mut daemon = case.daemon(Fake::default());
    let now = Instant::now();
    daemon.pass(ReconcileReason::Cadence, now).expect("pass");

    case.config("{port: 7300}", &[("alpha", "box-c"), ("beta", "box-b")]);
    let done = daemon
        .pass(ReconcileReason::ConfigChange, now)
        .expect("pass");
    assert_eq!(
        actions(&done),
        [("alpha".to_owned(), ReconcileAction::Restart)]
    );
    assert_eq!(daemon.started()["alpha"].placement, "box-c");
    let unit = unit_name("alpha");
    let tail: Vec<&String> = daemon.units.calls.iter().rev().take(2).collect();
    assert!(tail[1] == &format!("stop {unit}") && tail[0].starts_with(&format!("start {unit} ")));

    // The binary a runtime is compared against is its instance's desired binary, kept in the
    // runtime record (`story:instance-upgrade`), not the daemon's own: a new daemon binary
    // restarts nothing.
    daemon.binary = binary("1.1.0");
    assert!(
        daemon
            .pass(ReconcileReason::Cadence, now)
            .expect("pass")
            .is_empty()
    );

    // alpha's desired binary changes in the record: alpha restarts on it, beta does not.
    let units = daemon.units.clone();
    drop(daemon);
    let file = case.dir.join("daemon").join(BINARIES_FILE);
    let mut pins: Value =
        serde_json::from_str(&fs::read_to_string(&file).expect("read the binaries")).expect("JSON");
    pins["alpha"]["desired"] = serde_json::json!({
        "path": "/opt/conductor/1.2.0/conductor",
        "version": "1.2.0"
    });
    fs::write(&file, pins.to_string()).expect("write the binaries");
    let mut daemon = case.daemon(units);
    let done = daemon.pass(ReconcileReason::Cadence, now).expect("pass");
    assert_eq!(
        actions(&done),
        [("alpha".to_owned(), ReconcileAction::Restart)]
    );
    assert_eq!(daemon.started()["alpha"].version, "1.2.0");
    assert_eq!(daemon.started()["beta"].version, "1.0.0");
    let last = daemon.units.calls.last().expect("a call");
    assert!(
        last.starts_with(&format!("start {unit} /opt/conductor/1.2.0/conductor ")),
        "{last}"
    );
    assert!(
        daemon
            .pass(ReconcileReason::Cadence, now)
            .expect("pass")
            .is_empty()
    );
    assert_eq!(recorded(&daemon), 2 + 1 + 1);
}

#[test]
fn a_restarted_daemon_finds_its_runtimes_running_and_does_nothing() {
    let case = Case::new("reopen");
    case.config("{port: 7300}", &[("alpha", "box-a")]);
    let mut first = case.daemon(Fake::default());
    first
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("pass");
    assert!(case.dir.join("daemon").join(RUNTIMES_FILE).is_file());
    let units = first.units.clone();
    drop(first);
    let mut second = case.daemon(units);
    assert!(
        second
            .pass(ReconcileReason::Cadence, Instant::now())
            .expect("pass")
            .is_empty()
    );
    assert_eq!(recorded(&second), 1);
}

#[test]
fn a_runtime_that_stopped_answering_is_started_again() {
    let case = Case::new("lost");
    case.config("{port: 7300}", &[("alpha", "box-a")]);
    let mut daemon = case.daemon(Fake::default());
    let now = Instant::now();
    daemon.pass(ReconcileReason::Cadence, now).expect("pass");
    assert_eq!(daemon.due(now).expect("due"), None);
    daemon.units.running.clear();
    // It ran past the start bound: a lost runtime, no failed start
    // (`story:box-crash-loop-reads-failed`).
    let now = now + conductor_cli::daemon::reconcile::RAN_BOUND + std::time::Duration::from_secs(2);
    assert_eq!(
        daemon.due(now).expect("due"),
        Some(ReconcileReason::LostRuntime)
    );
    let done = daemon
        .pass(ReconcileReason::LostRuntime, now)
        .expect("pass");
    assert_eq!(
        actions(&done),
        [("alpha".to_owned(), ReconcileAction::Start)]
    );
    assert_eq!(done[0].reason, ReconcileReason::LostRuntime);
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    assert_eq!(daemon.due(now).expect("due"), None);
}

#[test]
fn a_pass_is_due_at_start_on_a_config_change_and_every_cadence() {
    let case = Case::new("due");
    case.config("{port: 7300}", &[("alpha", "box-a")]);
    let mut daemon = case.daemon(Fake::default());
    let start = Instant::now();
    assert_eq!(
        daemon.due(start).expect("due"),
        Some(ReconcileReason::Cadence)
    );
    daemon.tick(start).expect("tick");
    assert_eq!(daemon.due(start).expect("due"), None);
    assert_eq!(
        daemon.due(start + Duration::from_secs(59)).expect("due"),
        None
    );
    assert_eq!(
        daemon.due(start + Duration::from_secs(60)).expect("due"),
        Some(ReconcileReason::Cadence),
        "daemon.cadence.reconcile is 60 s when the file writes none"
    );

    case.config(
        "{port: 7300, cadence: {reconcile: 10s}}",
        &[("alpha", "box-a")],
    );
    assert_eq!(
        daemon.due(start).expect("due"),
        Some(ReconcileReason::ConfigChange)
    );
    let done = daemon.tick(start).expect("tick");
    assert!(done.is_empty(), "the change touched no instance: {done:?}");
    assert_eq!(
        daemon.due(start + Duration::from_secs(9)).expect("due"),
        None
    );
    assert_eq!(
        daemon.due(start + Duration::from_secs(10)).expect("due"),
        Some(ReconcileReason::Cadence)
    );
}

#[test]
fn a_refused_start_is_recorded_refused_with_what_the_manager_said() {
    let case = Case::new("refused");
    case.config("{port: 7300}", &[("alpha", "box-a")]);
    let mut units = Fake::default();
    units.refuse.insert(unit_name("alpha"));
    let mut daemon = case.daemon(units);
    let done = daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("pass");
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].result, ReconcileResult::Refused);
    assert!(
        done[0]
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("refused")),
        "{done:?}"
    );
    assert_eq!(recorded(&daemon), 1);
    assert!(daemon.started().is_empty());
}

/// Sends `request` to `port` and answers the status line and the body.
fn http(port: u16, request: &str) -> (String, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.write_all(request.as_bytes()).expect("send");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("read");
    let (head, body) = answer.split_once("\r\n\r\n").expect("a head and a body");
    (
        head.lines().next().unwrap_or_default().to_owned(),
        body.to_owned(),
    )
}

#[test]
fn get_v1_instances_lists_every_instance_with_placement_state_version_and_last_seen() {
    let case = Case::new("api");
    case.config("{port: 7300}", &[("alpha", "box-a"), ("beta", "box-b")]);
    let mut units = Fake::default();
    units.refuse.insert(unit_name("beta"));
    let mut daemon = case.daemon(units);
    daemon.tick(Instant::now()).expect("tick");
    let rows = Arc::new(api::Api::new("0".repeat(64), "operator".to_owned(), None));
    rows.publish(daemon.rows(), daemon.routes(), Vec::new());
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    let served = Arc::clone(&rows);
    thread::spawn(move || api::serve(&listener, &served));

    let (status, body) = http(
        port,
        "GET /v1/instances HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert_eq!(status, "HTTP/1.1 200 OK");
    let list: Value = serde_json::from_str(&body).expect("a JSON body");
    let list = list.as_array().expect("a list");
    assert_eq!(list.len(), 2, "{list:?}");
    let alpha = &list[0];
    assert_eq!(alpha["name"], "alpha");
    assert_eq!(alpha["placement"], "box-a");
    assert_eq!(alpha["state"], "Running");
    assert_eq!(alpha["binary_version"], "1.0.0");
    assert!(alpha["last_seen"].is_string(), "{alpha}");
    assert_eq!(alpha["changed_by"]["kind"], "Daemon");
    let beta = &list[1];
    assert_eq!(beta["name"], "beta");
    assert_eq!(beta["placement"], "box-b");
    assert_eq!(beta["state"], "Stopped");
    assert!(
        beta["binary_version"].is_null() && beta["last_seen"].is_null(),
        "{beta}"
    );

    // The runtime stops: the next look marks it unreachable.
    daemon.units.running.clear();
    daemon.due(Instant::now()).expect("due");
    rows.publish(daemon.rows(), daemon.routes(), Vec::new());
    let (_, body) = http(port, "GET /v1/instances HTTP/1.1\r\n\r\n");
    let list: Value = serde_json::from_str(&body).expect("a JSON body");
    assert_eq!(list[0]["state"], "Unreachable");

    let (status, _) = http(port, "GET /v1/other HTTP/1.1\r\n\r\n");
    assert_eq!(status, "HTTP/1.1 404 Not Found");
    let (status, _) = http(port, "POST /v1/instances HTTP/1.1\r\n\r\n");
    assert_eq!(status, "HTTP/1.1 405 Method Not Allowed");
}

#[test]
fn install_writes_and_enables_conductord_and_uninstall_stops_and_removes_it() {
    let case = Case::new("install");
    let file = install::unit_file(&case.home(), None);
    assert_eq!(
        file,
        case.home().join(".config/systemd/user/conductord.service")
    );
    assert_eq!(
        install::unit_file(&case.home(), Some(Path::new("/srv/xdg"))),
        Path::new("/srv/xdg/systemd/user/conductord.service")
    );
    let text = install::unit_text(
        Path::new("/opt/conductor/bin/conductor"),
        Some(&case.file()),
        None,
        Some("/usr/bin"),
    );
    assert!(
        text.contains(&format!(
            "ExecStart=\"/opt/conductor/bin/conductor\" \"--config\" \"{}\" \"daemon\" \"serve\"\n",
            case.file().display()
        )),
        "{text}"
    );
    assert!(text.contains("Restart=on-failure\n") && text.contains("WantedBy=default.target\n"));

    let mut units = Fake::default();
    let line = install::install(&mut units, &file, &text).expect("install");
    assert!(line.contains("conductord.service"), "{line}");
    assert_eq!(fs::read_to_string(&file).expect("the unit file"), text);
    assert_eq!(units.calls, ["enable conductord.service"]);

    let line = install::uninstall(&mut units, &file).expect("uninstall");
    assert!(line.contains("stopped"), "{line}");
    assert!(!file.exists());
    assert_eq!(
        units.calls,
        ["enable conductord.service", "disable conductord.service"]
    );
    let line = install::uninstall(&mut units, &file).expect("uninstall");
    assert_eq!(line, "conductord.service: not installed");
    assert_eq!(units.calls.len(), 2);
}

#[test]
fn the_per_instance_run_unit_is_gone_and_install_and_uninstall_are_one_at_a_time() {
    let case = Case::new("cli");
    let run = common::conductor(case.home())
        .args(["run", "--install"])
        .output()
        .expect("run conductor");
    assert_eq!(run.status.code(), Some(2), "{run:?}");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("--install"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let both = common::conductor(case.home())
        .args(["daemon", "serve", "--install", "--uninstall"])
        .output()
        .expect("run conductor");
    assert_eq!(both.status.code(), Some(1), "{both:?}");
    assert!(
        String::from_utf8_lossy(&both.stderr).contains("give at most one"),
        "{}",
        String::from_utf8_lossy(&both.stderr)
    );
}

#[test]
fn the_daemon_refuses_to_start_without_daemon_store_naming_it() {
    let case = Case::new("no-store");
    case.config_as("{port: 7300}", &[("alpha", "box-a")]);
    let Err(error) = Daemon::open(
        case.file(),
        case.home(),
        case.dir.join("daemon"),
        binary("1.0.0"),
        Fake::default(),
    ) else {
        panic!("a config without daemon.store opened a daemon");
    };
    let said = format!("{error:#}");
    assert!(said.contains("daemon.store"), "{said}");
    assert_eq!(said.lines().count(), 1, "{said}");

    let mut child = common::conductor(case.home())
        .arg("--config")
        .arg(case.file())
        .args(["daemon", "serve"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run conductor");
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("daemon serve started without daemon.store");
        }
        thread::sleep(Duration::from_millis(50));
    };
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("stderr")
        .read_to_string(&mut stderr)
        .expect("read stderr");
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains("daemon.store"), "{stderr}");
}

#[test]
fn a_store_url_with_a_secret_password_is_refused_at_daemon_start() {
    let case = Case::new("secret-store");
    case.config_as(
        "{port: 7300, store: \"postgres://conductor:secret:db-password@db.example/conductor\"}",
        &[("alpha", "box-a")],
    );
    let Err(error) = Daemon::open(
        case.file(),
        case.home(),
        case.dir.join("daemon"),
        binary("1.0.0"),
        Fake::default(),
    ) else {
        panic!("a secret: store URL opened a daemon");
    };
    let said = format!("{error:#}");
    assert!(said.contains("daemon.store"), "{said}");
    assert!(said.contains("secret:"), "{said}");
    assert_eq!(said.lines().count(), 1, "{said}");
}

#[test]
fn reconciliations_land_in_the_shared_database_under_the_daemons_tenant() {
    let case = Case::new("shared");
    case.config("{port: 7300}", &[("alpha", "box-a")]);
    let mut daemon = case.daemon(Fake::default());
    daemon
        .pass(ReconcileReason::Cadence, Instant::now())
        .expect("pass");
    let handed = format!("CONDUCTOR_STORE={}", case.url());
    assert!(
        daemon.units.envs.iter().all(|env| env.contains(&handed)),
        "every runtime is handed daemon.store: {:?}",
        daemon.units.envs
    );
    assert!(
        !case.dir.join("daemon").join("store.sqlite").exists(),
        "the daemon opened a store of its own directory"
    );
    drop(daemon);

    // The runtime of alpha is handed this URL in CONDUCTOR_STORE and opens this one file.
    let url = case.url();
    let read = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
        .expect("the shared database opens");
    let rows = ReconciliationStorage::list(&read);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].data.instance.0, "alpha");
    assert_eq!(rows[0].data.action, ReconcileAction::Start);
    let runtime = conductor_cli::store::open_url(&url, "alpha").expect("the runtime's handle");
    assert!(
        ReconciliationStorage::list(&runtime).is_empty(),
        "the daemon's records are in its own tenant, not the instance's"
    );
}

// ---------------------------------------------------------------------------------------------
// Adversary pass 1 on story:box-crash-loop-reads-failed (uncommitted)
// ---------------------------------------------------------------------------------------------

mod adversary {
    use super::*;

    /// Mutant: the `boxed` branch of `failed_start` and the failed start of a runtime in a
    /// unit, which no case of the unit holds: it is recorded as `Refused` without a log's
    /// text, reads `Unreachable`, and is not started again before its wait is over.
    #[test]
    fn a_runtime_in_a_unit_that_stops_within_the_start_bound_is_a_failed_start() {
        let case = Case::new("adv-unit-failed");
        case.config("{port: 7300}", &[("alpha", "box-a")]);
        let mut daemon = case.daemon(Fake::default());
        let start = Instant::now();
        daemon.pass(ReconcileReason::Cadence, start).expect("pass");
        let starts = |daemon: &Daemon<Fake>| {
            daemon
                .units
                .calls
                .iter()
                .filter(|call| call.starts_with("start "))
                .count()
        };
        assert_eq!(starts(&daemon), 1);
        daemon.units.running.clear();
        let at = start + Duration::from_secs(2);
        assert_eq!(
            daemon.due(at).expect("due"),
            Some(ReconcileReason::LostRuntime)
        );
        let done = daemon.pass(ReconcileReason::LostRuntime, at).expect("pass");
        assert_eq!(done.len(), 1, "{done:?}");
        assert_eq!(done[0].result, ReconcileResult::Refused, "{done:?}");
        assert_eq!(
            done[0].detail.as_deref(),
            Some(
                "the runtime of alpha exited within 10 s of its start (failed start 1 in a row, \
                 the next start in 10 s)"
            )
        );
        assert_eq!(starts(&daemon), 1, "nothing is started in the same pass");
        assert_eq!(daemon.rows()[0]["state"], "Unreachable");
        let waiting = start + Duration::from_secs(10);
        assert_eq!(daemon.due(waiting).expect("due"), None);
        let retry = start + Duration::from_secs(12);
        assert_eq!(
            daemon.due(retry).expect("due"),
            Some(ReconcileReason::LostRuntime)
        );
        let done = daemon
            .pass(ReconcileReason::LostRuntime, retry)
            .expect("pass");
        assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
        assert_eq!(starts(&daemon), 2);
    }
}
