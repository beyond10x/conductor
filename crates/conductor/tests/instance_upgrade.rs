//! `POST /v1/instances/{name}/upgrade` and `conductor instance upgrade` (`story:instance-upgrade`,
//! design architecture-next § 4, § 10): the daemon fetches a release, installs its binary for the
//! instance's placement, runs its `conductor doctor`, restarts the runtime on it, and rolls back to
//! the previous binary when the runtime does not answer within `daemon.upgrade_timeout` or stops
//! running; every step is one `Reconciliation`. `--all` goes one instance at a time and stops at
//! the first rollback. Every case runs against a fake service manager ([`Fake`]) and a fake
//! release ([`Release`]); none touches systemd or the network.

mod active;
mod common;

use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Result, bail};
use conductor_cli::confine::systemd::Service;
use conductor_cli::daemon::reconcile::{Binary, Daemon, unit_name};
use conductor_cli::daemon::units::Units;
use conductor_cli::daemon::upgrade::{self, Refusal, Releases, Upgraded};
use conductor_cli::daemon::{api, reconcile::Done};
use conductor_model::behaviour::ReconciliationStorage;
use conductor_model::instance::{ReconcileAction, ReconcileReason, ReconcileResult};
use conductor_model::operator::ActorKind;
use serde_json::{Value, json};

/// The service manager of a case: what runs, what of it answers, and every call made.
#[derive(Debug, Default, Clone)]
struct Fake {
    running: BTreeSet<String>,
    mute: BTreeSet<String>,
    calls: Vec<String>,
}

impl Units for Fake {
    fn start(&mut self, service: &Service) -> Result<()> {
        self.calls.push(format!(
            "start {} {}",
            service.unit,
            service.command.join(" ")
        ));
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

    fn answers(&mut self, unit: &str) -> Result<bool> {
        Ok(self.running.contains(unit) && !self.mute.contains(unit))
    }

    fn enable(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn disable(&mut self, _: &str) -> Result<()> {
        Ok(())
    }
}

/// The release of a case: what it was asked, whether the download fails, whether `doctor` fails.
#[derive(Debug, Default, Clone)]
struct Release {
    asked: Arc<Mutex<Vec<String>>>,
    unread: bool,
    doctor_fails: bool,
}

impl Releases for Release {
    fn fetch(&mut self, tag: &str, work: &Path) -> Result<PathBuf> {
        self.asked
            .lock()
            .expect("asked")
            .push(format!("fetch {tag}"));
        if self.unread {
            bail!("release unread: curl: 404");
        }
        let file = work.join("conductor");
        fs::write(&file, format!("conductor {tag}")).expect("write the fetched binary");
        Ok(file)
    }

    fn doctor(&mut self, binary: &Path, service: &Service) -> Result<()> {
        self.asked.lock().expect("asked").push(format!(
            "doctor {} in {}",
            binary.display(),
            service.cwd.display()
        ));
        if self.doctor_fails {
            bail!("doctor: substrate missing");
        }
        Ok(())
    }
}

struct Case {
    dir: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("instance-upgrade")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        Self { dir }
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// Writes the config file: `daemon` with this case's store added, one instance per name.
    fn config(&self, daemon: &str, instances: &[&str]) {
        let store = format!("sqlite:{}", self.dir.join("shared.sqlite").display());
        let daemon = daemon.replacen('{', &format!("{{store: {store:?}, "), 1);
        let mut text = format!("version: conductor.config/2\ndaemon: {daemon}\ninstances:\n");
        for name in instances {
            let root = self.dir.join(name);
            text.push_str(&format!(
                "  - name: {name}\n    placement: local\n    session_prefix: {name}\n    \
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

    /// The daemon of this case, every instance started on `1.0.0`.
    fn daemon(&self, release: Release) -> Daemon<Fake> {
        let mut daemon = Daemon::open(
            self.file(),
            self.dir.join("home"),
            self.dir.join("daemon"),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            Fake::default(),
        )
        .expect("open the daemon");
        daemon.releases = Box::new(release);
        daemon
            .pass(ReconcileReason::Cadence, Instant::now())
            .expect("the first pass");
        daemon
    }

    fn installed(&self, version: &str) -> PathBuf {
        self.dir
            .join("daemon")
            .join(upgrade::BINARIES_DIR)
            .join(version)
            .join("conductor")
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

/// The reconciliations of action `Upgrade`, in order: result and the detail's step.
fn steps(daemon: &Daemon<Fake>) -> Vec<(ReconcileResult, String)> {
    let mut rows = ReconciliationStorage::list(daemon.store());
    rows.sort_by(|left, right| left.data.at.0.cmp(&right.data.at.0));
    rows.into_iter()
        .filter(|row| row.data.action == ReconcileAction::Upgrade)
        .map(|row| {
            let detail = row.data.detail.clone().unwrap_or_default();
            let step = detail.split(':').next().unwrap_or_default().to_owned();
            (row.data.result, step)
        })
        .collect()
}

fn row<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["name"] == name)
        .expect("a row of the instance")
}

fn operator() -> conductor_model::operator::Actor {
    conductor_cli::actor::operator("operator-one")
}

#[test]
fn an_upgrade_fetches_installs_runs_doctor_restarts_and_the_runtime_comes_up_on_it() {
    let case = Case::new("up");
    case.config("{port: 7300}", &["alpha"]);
    let release = Release::default();
    let asked = Arc::clone(&release.asked);
    let mut daemon = case.daemon(release);
    let now = SystemTime::now();

    let upgraded = daemon
        .upgrade("alpha", "v1.1.0", &operator(), now)
        .expect("the upgrade");
    assert_eq!(
        upgraded,
        Upgraded::Upgrading {
            version: "1.1.0".to_owned()
        }
    );
    let installed = case.installed("1.1.0");
    assert_eq!(
        fs::read_to_string(&installed).expect("the installed binary"),
        "conductor v1.1.0"
    );
    let asked = asked.lock().expect("asked").clone();
    assert_eq!(asked[0], "fetch v1.1.0");
    assert!(
        asked[1].starts_with(&format!("doctor {}", installed.display())),
        "{asked:?}"
    );
    let unit = unit_name("alpha");
    let tail: Vec<&String> = daemon.units.calls.iter().rev().take(2).collect();
    assert_eq!(tail[1], &format!("stop {unit}"));
    assert!(
        tail[0].starts_with(&format!("start {unit} {} ", installed.display())),
        "{tail:?}"
    );
    assert_eq!(
        steps(&daemon),
        [
            (ReconcileResult::Succeeded, "fetch".to_owned()),
            (ReconcileResult::Succeeded, "install".to_owned()),
            (ReconcileResult::Succeeded, "doctor".to_owned()),
            (ReconcileResult::Succeeded, "restart".to_owned()),
        ]
    );
    for recorded in ReconciliationStorage::list(daemon.store())
        .into_iter()
        .filter(|row| row.data.action == ReconcileAction::Upgrade)
    {
        assert_eq!(recorded.data.reason, ReconcileReason::OperatorCommand);
        assert_eq!(recorded.data.actor.kind, ActorKind::Human);
    }
    let rows = daemon.rows();
    assert_eq!(row(&rows, "alpha")["state"], "Upgrading");
    assert_eq!(row(&rows, "alpha")["binary_version"], "1.1.0");

    let done = daemon.settle(now).expect("settle");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(
        (done[0].action, done[0].result),
        (ReconcileAction::Upgrade, ReconcileResult::Succeeded)
    );
    assert!(
        done[0]
            .detail
            .as_deref()
            .unwrap_or_default()
            .starts_with("up:")
    );
    let rows = daemon.rows();
    assert_eq!(row(&rows, "alpha")["state"], "Running");
    assert_eq!(row(&rows, "alpha")["binary_version"], "1.1.0");

    // The instance's desired binary is the upgraded one, not the daemon's own (1.0.0): a pass
    // leaves it alone, and so does a daemon opened again on the same directory.
    assert!(
        daemon
            .pass(ReconcileReason::Cadence, Instant::now())
            .expect("pass")
            .is_empty()
    );
    let units = daemon.units.clone();
    drop(daemon);
    let mut again = Daemon::open(
        case.file(),
        case.dir.join("home"),
        case.dir.join("daemon"),
        Binary {
            path: PathBuf::from("/opt/conductor/bin/conductor"),
            version: "1.0.0".to_owned(),
        },
        units,
    )
    .expect("open the daemon again");
    assert!(
        again
            .pass(ReconcileReason::Cadence, Instant::now())
            .expect("pass")
            .is_empty()
    );
    assert_eq!(again.started()["alpha"].version, "1.1.0");
}

#[test]
fn a_runtime_that_does_not_answer_within_upgrade_timeout_is_rolled_back() {
    let case = Case::new("timeout");
    case.config("{port: 7300, upgrade_timeout: 30s}", &["alpha"]);
    let mut daemon = case.daemon(Release::default());
    daemon.units.mute.insert(unit_name("alpha"));
    let now = SystemTime::now();
    daemon
        .upgrade("alpha", "v1.1.0", &operator(), now)
        .expect("the upgrade");

    assert!(
        daemon
            .settle(now + Duration::from_secs(29))
            .expect("settle")
            .is_empty(),
        "within daemon.upgrade_timeout nothing is done"
    );
    let done = daemon
        .settle(now + Duration::from_secs(31))
        .expect("settle");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(
        (done[0].action, done[0].reason, done[0].result),
        (
            ReconcileAction::Upgrade,
            ReconcileReason::Cadence,
            ReconcileResult::RolledBack
        )
    );
    let unit = unit_name("alpha");
    let last = daemon.units.calls.last().expect("a call");
    assert!(
        last.starts_with(&format!("start {unit} /opt/conductor/bin/conductor ")),
        "{last}"
    );
    assert_eq!(daemon.started()["alpha"].version, "1.0.0");
    assert_eq!(row(&daemon.rows(), "alpha")["binary_version"], "1.0.0");
    assert_eq!(steps(&daemon).last().expect("a step").1, "rollback");
}

#[test]
fn a_runtime_lost_while_upgrading_is_rolled_back_not_started_again_on_the_new_binary() {
    let case = Case::new("lost");
    case.config("{port: 7300}", &["alpha"]);
    let mut daemon = case.daemon(Release::default());
    let now = Instant::now();
    daemon
        .upgrade("alpha", "v1.1.0", &operator(), SystemTime::now())
        .expect("the upgrade");
    daemon.units.running.clear();
    let done: Vec<Done> = daemon.tick(now).expect("tick");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(
        (done[0].action, done[0].reason, done[0].result),
        (
            ReconcileAction::Upgrade,
            ReconcileReason::LostRuntime,
            ReconcileResult::RolledBack
        )
    );
    let last = daemon.units.calls.last().expect("a call");
    assert!(last.contains(" /opt/conductor/bin/conductor "), "{last}");
    assert_eq!(row(&daemon.rows(), "alpha")["state"], "Running");
}

#[test]
fn a_failing_doctor_rolls_back_before_the_switch_and_the_runtime_runs_on() {
    let case = Case::new("doctor");
    case.config("{port: 7300}", &["alpha"]);
    let mut daemon = case.daemon(Release {
        doctor_fails: true,
        ..Release::default()
    });
    let calls = daemon.units.calls.len();
    let upgraded = daemon
        .upgrade("alpha", "v1.1.0", &operator(), SystemTime::now())
        .expect("the upgrade answers");
    assert!(
        matches!(&upgraded, Upgraded::RolledBack { detail } if detail.contains("substrate missing")),
        "{upgraded:?}"
    );
    assert_eq!(
        daemon.units.calls.len(),
        calls,
        "the runtime was not touched"
    );
    assert_eq!(
        steps(&daemon).last().expect("a step"),
        &(ReconcileResult::RolledBack, "doctor".to_owned())
    );
    assert_eq!(row(&daemon.rows(), "alpha")["state"], "Running");
    assert_eq!(daemon.started()["alpha"].version, "1.0.0");
}

#[test]
fn an_unknown_or_stopped_instance_or_an_unread_release_is_refused_and_nothing_restarts() {
    let case = Case::new("refused");
    case.config("{port: 7300}", &["alpha", "beta"]);
    let mut daemon = case.daemon(Release {
        unread: true,
        ..Release::default()
    });
    let before = ReconciliationStorage::list(daemon.store()).len();
    assert!(matches!(
        daemon.upgrade("gamma", "v1.1.0", &operator(), SystemTime::now()),
        Err(Refusal::UnknownInstance(name)) if name == "gamma"
    ));
    daemon.units.running.remove(&unit_name("beta"));
    daemon.due(Instant::now()).expect("observe");
    assert!(matches!(
        daemon.upgrade("beta", "v1.1.0", &operator(), SystemTime::now()),
        Err(Refusal::InstanceStateConflict(name)) if name == "beta"
    ));
    assert_eq!(ReconciliationStorage::list(daemon.store()).len(), before);

    let calls = daemon.units.calls.len();
    let refused = daemon.upgrade("alpha", "v1.1.0", &operator(), SystemTime::now());
    assert!(
        matches!(&refused, Err(Refusal::Refused { detail }) if detail.contains("404")),
        "{refused:?}"
    );
    assert_eq!(daemon.units.calls.len(), calls);
    assert_eq!(
        steps(&daemon),
        [(ReconcileResult::Refused, "fetch".to_owned())]
    );
}

#[test]
fn upgrade_timeout_is_120s_unless_the_file_writes_one() {
    let case = Case::new("timeout-key");
    let shown = |daemon: &str| {
        case.config(daemon, &["alpha"]);
        let text = fs::read_to_string(case.file()).expect("read the config file");
        conductor_cli::config::parse(&text, &case.dir.join("home"))
            .map(|config| conductor_cli::config::seconds(&config.daemon.upgrade_timeout))
    };
    assert_eq!(shown("{port: 7300}"), Ok(Some(120)));
    assert_eq!(shown("{port: 7300, upgrade_timeout: 5m}"), Ok(Some(300)));
    let problems = shown("{port: 7300, upgrade_timeout: soon}").expect_err("refused");
    assert!(
        problems
            .iter()
            .any(|problem| problem.to_string().contains("daemon.upgrade_timeout")),
        "{problems:?}"
    );
}

/// Sends `request` to `port`; answers the status line and the body.
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
fn post_v1_instances_upgrade_hands_the_request_to_the_daemon_and_answers_its_outcome() {
    let case = Case::new("post");
    case.config("{port: 7300}", &["alpha"]);
    let mut daemon = case.daemon(Release::default());
    let requests = upgrade::listen();
    // The global API of `story:instance-api`: a local client writes with the daemon's token, as
    // the operator the Unix user names, who holds every grant while `daemon.operators` is empty.
    let token = "3333333333333333333333333333333333333333333333333333333333333333";
    let api = Arc::new(api::Api::new(token.to_owned(), "owner".to_owned(), None));
    api.publish(daemon.rows(), std::collections::BTreeMap::new(), Vec::new());
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    thread::spawn(move || api::serve(&listener, &api));

    let body = json!({"tag": "v1.1.0"}).to_string();
    let request = format!(
        "POST /v1/instances/alpha/upgrade HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost:{port}\r\nX-Conductor-Token: {token}\r\nContent-Type: \
         application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let client = thread::spawn(move || http(port, &request));
    let handed = requests
        .recv_timeout(Duration::from_secs(10))
        .expect("the request reaches the loop");
    daemon.answer(handed);
    let (status, body) = client.join().expect("the client");
    assert_eq!(status, "HTTP/1.1 202 Accepted", "{body}");
    let body: Value = serde_json::from_str(&body).expect("a JSON body");
    assert_eq!(body["state"], "Upgrading");
    assert_eq!(body["binary_version"], "1.1.0");

    let missing = format!(
        "POST /v1/instances/alpha/upgrade HTTP/1.1\r\nOrigin: http://127.0.0.1:{port}\r\nX-Conductor-Token: {token}\r\n\
         Content-Length: 2\r\n\r\n{{}}"
    );
    let (status, body) = http(port, &missing.replace(token, "0"));
    assert_eq!(status, "HTTP/1.1 403 Forbidden", "{body}");
    let (status, body) = http(port, &missing);
    assert_eq!(status, "HTTP/1.1 400 Bad Request", "{body}");
}

/// A daemon's API for the client cases: each `POST .../upgrade` is answered `202` and recorded;
/// `GET /v1/instances` answers each upgraded instance on the version `comes_up` gives it.
fn fake_daemon(comes_up: fn(&str) -> &'static str) -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    let posted = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = Arc::clone(&posted);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut line = String::new();
            reader.read_line(&mut line).expect("request line");
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).expect("header");
                if header.trim().is_empty() {
                    break;
                }
                if let Some((name, value)) = header.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).expect("body");
            let path = line
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_owned();
            let (status, answer) = if line.starts_with("POST ") {
                let name = path.split('/').nth(3).unwrap_or_default().to_owned();
                seen.lock().expect("posted").push(name.clone());
                (
                    "202 Accepted",
                    json!({"name": name, "state": "Upgrading", "binary_version": "1.1.0"}),
                )
            } else {
                let posted = seen.lock().expect("posted").clone();
                let rows: Vec<Value> = ["alpha", "beta", "gamma"]
                    .iter()
                    .map(|name| {
                        let version = if posted.iter().any(|done| done == name) {
                            comes_up(name)
                        } else {
                            "1.0.0"
                        };
                        json!({"name": name, "state": "Running", "binary_version": version})
                    })
                    .collect();
                ("200 OK", Value::Array(rows))
            };
            let answer = answer.to_string();
            let mut stream = stream;
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{answer}",
                answer.len()
            );
        }
    });
    (port, posted)
}

#[test]
fn instance_upgrade_all_goes_one_at_a_time_and_stops_at_the_first_rollback() {
    let case = Case::new("all");
    let (port, posted) = fake_daemon(|name| if name == "beta" { "1.0.0" } else { "1.1.0" });
    case.config(&format!("{{port: {port}}}"), &["alpha", "beta", "gamma"]);
    let output = common::conductor(case.dir.join("home"))
        .args(["--config"])
        .arg(case.file())
        .args(["instance", "upgrade", "--all", "--tag", "v1.1.0"])
        .output()
        .expect("run conductor");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stdout}\n{stderr}");
    assert_eq!(*posted.lock().expect("posted"), ["alpha", "beta"]);
    assert!(stdout.contains("alpha: upgraded to 1.1.0"), "{stdout}");
    assert!(stderr.contains("beta: rolled back"), "{stderr}");
    assert!(!stdout.contains("gamma") && !stderr.contains("gamma"));
}

#[test]
fn instance_upgrade_of_one_instance_reports_it_upgraded() {
    let case = Case::new("one");
    let (port, posted) = fake_daemon(|_| "1.1.0");
    case.config(&format!("{{port: {port}}}"), &["alpha", "beta", "gamma"]);
    let output = common::conductor(case.dir.join("home"))
        .args(["--config"])
        .arg(case.file())
        .args([
            "instance",
            "upgrade",
            "--instance",
            "beta",
            "--tag",
            "1.1.0",
        ])
        .output()
        .expect("run conductor");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(*posted.lock().expect("posted"), ["beta"]);
    assert!(stdout.contains("beta: upgraded to 1.1.0"), "{stdout}");
}
