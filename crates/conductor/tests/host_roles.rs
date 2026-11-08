//! `story:host-roles-by-daemon` (design architecture-next § 10): the daemon starts the host roles
//! `globals.roles` lists, the supervisor and the teacher, through `conductor session start --role
//! <role>` at the `host` placement, keeps them running and stops one the file no longer lists; a
//! host role's session runs in `conductor_host.slice`; `POST /v1/roles/{role}/start` asks the
//! daemon for one now; `POST /v1/ask` reaches the supervisor once it runs; `task
//! supervisor:start` and `task teacher:start` call the daemon, and no task runs `claude --bg` or a
//! `~/.claude/agents/` adapter. Every case runs against a fake service manager ([`Fake`]) and a
//! fake delivery; none touches systemd, tmux or a harness.

mod active;
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use conductor_cli::confine::systemd::{self, Service};
use conductor_cli::daemon::api::{self, Api};
use conductor_cli::daemon::host_roles::{self, HostStart, Supervisor};
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::units::Units;
use conductor_model::instance::{ReconcileAction, ReconcileReason, ReconcileResult};
use serde_json::{Value, json};

/// The daemon's own token in the API cases.
const DAEMON_TOKEN: &str = "3333333333333333333333333333333333333333333333333333333333333333";

/// The service manager of a case: what runs, and every host role it was asked to start.
#[derive(Debug, Default, Clone)]
struct Fake {
    running: BTreeSet<String>,
    calls: Vec<String>,
    hosts: Vec<HostStart>,
}

impl Units for Fake {
    fn start(&mut self, service: &Service) -> Result<()> {
        self.calls.push(format!("start {}", service.unit));
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
        bail!("no enable of {unit} in these cases")
    }

    fn disable(&mut self, unit: &str) -> Result<()> {
        bail!("no disable of {unit} in these cases")
    }

    fn start_host_role(&mut self, start: &HostStart) -> Result<()> {
        self.calls.push(format!("host {}", start.role));
        self.hosts.push(start.clone());
        self.running.insert(start.unit.clone());
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
            .join("host-roles")
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

    fn daemon_dir(&self) -> PathBuf {
        self.dir.join("daemon")
    }

    /// Writes the config file with one instance, `alpha`, and `globals.roles` naming `roles`.
    fn config(&self, roles: &[&str]) {
        let root = self.dir.join("alpha");
        let mut text = format!(
            "version: conductor.config/2\ndaemon: {{port: 7399, store: {:?}}}\n",
            format!("sqlite:{}", self.dir.join("shared.sqlite").display())
        );
        if !roles.is_empty() {
            text.push_str("globals:\n  roles:\n");
            for role in roles {
                text.push_str(&format!(
                    "    - {{role: {role}, harness: claude, model: opus}}\n"
                ));
            }
        }
        text.push_str(&format!(
            "instances:\n  - name: alpha\n    placement: local\n    session_prefix: alpha\n    \
             sources: [{{github: example-org}}]\n    \
             checkouts: {{root: {r:?}, trees: {t:?}}}\n    records: {rec:?}\n    state: {s:?}\n    \
             cache: {c:?}\n",
            r = root.join("checkouts"),
            t = root.join("trees"),
            rec = root.join("records"),
            s = root.join("state"),
            c = root.join("cache"),
        ));
        fs::write(self.file(), text).expect("write the config file");
    }

    fn daemon(&self) -> Daemon<Fake> {
        Daemon::open(
            self.file(),
            self.home(),
            self.daemon_dir(),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            Fake::default(),
        )
        .expect("open the daemon")
    }
}

fn host_actions(done: &[conductor_cli::daemon::reconcile::Done]) -> Vec<(String, ReconcileAction)> {
    done.iter()
        .filter(|done| done.instance.starts_with("host:"))
        .map(|done| (done.instance.clone(), done.action))
        .collect()
}

#[test]
fn the_daemon_starts_each_listed_host_role_through_session_start_at_the_host_placement() {
    let case = Case::new("start");
    case.config(&["supervisor"]);
    let mut daemon = case.daemon();
    let now = Instant::now();
    let done = daemon.pass(ReconcileReason::Cadence, now).expect("pass");
    assert_eq!(
        host_actions(&done),
        [("host:supervisor".to_owned(), ReconcileAction::Start)],
        "{done:?}"
    );
    assert!(
        done.iter()
            .all(|done| done.result == ReconcileResult::Succeeded)
    );
    let [start] = daemon.units.hosts.as_slice() else {
        panic!("one host role started: {:?}", daemon.units.hosts);
    };
    assert_eq!(start.role, "supervisor");
    assert_eq!(start.placement, host_roles::PLACEMENT);
    assert_eq!(start.placement, "host");
    assert_eq!(start.unit, "conductor_supervisor.service");
    let state = case.daemon_dir().join("host").display().to_string();
    assert_eq!(
        start.command,
        [
            "/opt/conductor/bin/conductor",
            "--config",
            &case.file().display().to_string(),
            "--state-dir",
            &state,
            "session",
            "start",
            "--role",
            "supervisor",
        ],
    );
    assert_eq!(
        start.cwd, case.dir,
        "a host role starts beside the config file"
    );
    assert!(
        start
            .env
            .iter()
            .any(|set| set == "CONDUCTOR_INSTANCE=alpha"),
        "{:?}",
        start.env
    );
    // The teacher is not listed, so it is not started; a second pass starts nothing.
    let again = daemon
        .pass(ReconcileReason::Cadence, now + Duration::from_secs(1))
        .expect("second pass");
    assert!(host_actions(&again).is_empty(), "{again:?}");
    assert_eq!(daemon.units.hosts.len(), 1);
    assert_eq!(
        *daemon.host_roles(),
        BTreeMap::from([("supervisor".to_owned(), true)])
    );
}

#[test]
fn a_lost_host_role_is_started_again_and_one_no_longer_listed_is_stopped() {
    let case = Case::new("keep");
    case.config(&["supervisor"]);
    let mut daemon = case.daemon();
    let now = Instant::now();
    daemon.pass(ReconcileReason::Cadence, now).expect("pass");
    daemon.units.running.remove("conductor_supervisor.service");
    let due = daemon.due(now + Duration::from_secs(1)).expect("due");
    assert_eq!(due, Some(ReconcileReason::LostRuntime));
    let done = daemon
        .pass(ReconcileReason::LostRuntime, now + Duration::from_secs(1))
        .expect("pass");
    assert_eq!(
        host_actions(&done),
        [("host:supervisor".to_owned(), ReconcileAction::Start)]
    );
    // The teacher starts when the file lists it (decision H1); the supervisor stops when it no
    // longer does.
    case.config(&["teacher"]);
    let done = daemon
        .pass(ReconcileReason::ConfigChange, now + Duration::from_secs(2))
        .expect("pass");
    assert_eq!(
        host_actions(&done),
        [
            ("host:teacher".to_owned(), ReconcileAction::Start),
            ("host:supervisor".to_owned(), ReconcileAction::Stop),
        ],
        "{done:?}"
    );
    assert!(
        daemon
            .units
            .calls
            .contains(&"stop conductor_supervisor.service".to_owned())
    );
}

#[test]
fn a_host_role_asked_for_starts_now_and_one_not_listed_is_refused() {
    let case = Case::new("asked");
    case.config(&["supervisor", "teacher"]);
    let mut daemon = case.daemon();
    let done = daemon
        .start_asked(&BTreeSet::from(["teacher".to_owned()]))
        .expect("start the asked role");
    assert_eq!(
        host_actions(&done),
        [("host:teacher".to_owned(), ReconcileAction::Start)]
    );
    assert_eq!(daemon.units.hosts.len(), 1);
    case.config(&["supervisor"]);
    daemon
        .pass(ReconcileReason::ConfigChange, Instant::now())
        .expect("pass");
    let refused = daemon
        .start_asked(&BTreeSet::from(["teacher".to_owned()]))
        .expect("answer");
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert_eq!(refused[0].result, ReconcileResult::Refused);
    assert!(
        refused[0]
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("globals.roles")),
        "{refused:?}"
    );
}

#[test]
fn a_host_role_session_runs_in_conductor_host_slice() {
    assert_eq!(systemd::HOST_SLICE, "conductor_host.slice");
    for role in ["supervisor", "teacher"] {
        assert_eq!(
            systemd::session_slice(role, Some("alpha")),
            "conductor_host.slice"
        );
        assert_eq!(systemd::session_slice(role, None), "conductor_host.slice");
    }
    assert_eq!(
        systemd::session_slice("director", Some("alpha")),
        "conductor_alpha.slice"
    );
    assert_eq!(systemd::session_slice("lead", None), "conductor.slice");
}

#[test]
fn session_start_refuses_a_host_role_globals_roles_does_not_list() {
    let case = Case::new("unlisted");
    case.config(&["supervisor"]);
    let output = common::conductor(case.home())
        .arg("--config")
        .arg(case.file())
        .arg("--state-dir")
        .arg(case.dir.join("state"))
        .args(["session", "start", "--role", "teacher"])
        .output()
        .expect("run conductor");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("UnknownRole"), "{stderr}");
    assert!(stderr.contains("globals.roles"), "{stderr}");
}

/// Serves `api` on a loopback listener of its own; answers its port.
fn listen(api: Arc<Api>) -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    std::thread::spawn(move || api::serve(&listener, &api));
    port
}

/// A loopback write with the daemon's token; answers the status and the JSON body.
fn write(port: u16, target: &str, body: &Value) -> (u16, Value) {
    let body = serde_json::to_vec(body).expect("a JSON body");
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let head = format!(
        "POST {target} HTTP/1.1\r\nConnection: close\r\nHost: 127.0.0.1:{port}\r\n\
         Origin: http://127.0.0.1:{port}\r\nX-Conductor-Token: {DAEMON_TOKEN}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).expect("send");
    stream.write_all(&body).expect("send");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {head}"));
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

#[test]
fn ask_reaches_the_supervisor_once_it_runs() {
    let case = Case::new("ask");
    let delivered: Arc<Mutex<Vec<(PathBuf, String, String)>>> = Arc::default();
    let seen = Arc::clone(&delivered);
    let api = Api::new(DAEMON_TOKEN.to_owned(), "owner".to_owned(), None).with_deliver(Arc::new(
        move |supervisor: &Supervisor, text: &str| {
            seen.lock().expect("lock").push((
                supervisor.state.clone(),
                supervisor.name.clone(),
                text.to_owned(),
            ));
            Ok(())
        },
    ));
    api.publish(
        vec![json!({"name": "alpha", "state": "Running"})],
        BTreeMap::new(),
        Vec::new(),
    );
    let api = Arc::new(api);
    let port = listen(Arc::clone(&api));
    let (status, body) = write(
        port,
        "/v1/ask",
        &json!({"text": "which repository is red?"}),
    );
    assert_eq!(status, 503, "no supervisor runs yet: {body}");
    assert_eq!(body["error"], "conductor.instance.SupervisorNotRunning");
    let state = case.dir.join("host");
    api.publish_host(
        vec!["supervisor".to_owned()],
        Some(Supervisor {
            state: state.clone(),
            name: "supervisor".to_owned(),
        }),
    );
    let (status, body) = write(
        port,
        "/v1/ask",
        &json!({"text": "which repository is red?"}),
    );
    assert_eq!(status, 202, "{body}");
    assert_eq!(body["delivered_to"], "supervisor", "{body}");
    let (status, body) = write(
        port,
        "/v1/ask",
        &json!({"text": "is the build green?", "to": "alpha"}),
    );
    assert_eq!(status, 202, "{body}");
    assert_eq!(body["instance"], "alpha", "{body}");
    let delivered = delivered.lock().expect("lock").clone();
    assert_eq!(delivered.len(), 2, "{delivered:?}");
    assert_eq!(
        delivered[0],
        (
            state.clone(),
            "supervisor".to_owned(),
            "which repository is red?".to_owned()
        )
    );
    assert!(delivered[1].2.contains("alpha"), "{delivered:?}");
    assert!(
        delivered[1].2.contains("is the build green?"),
        "{delivered:?}"
    );
}

#[test]
fn post_roles_start_asks_the_daemon_for_a_listed_host_role() {
    let api = Arc::new(Api::new(DAEMON_TOKEN.to_owned(), "owner".to_owned(), None));
    api.publish_host(vec!["supervisor".to_owned()], None);
    let port = listen(Arc::clone(&api));
    let (status, body) = write(port, "/v1/roles/supervisor/start", &json!({}));
    assert_eq!(status, 202, "{body}");
    assert_eq!(body["role"], "supervisor", "{body}");
    assert!(host_roles::take_asked().contains("supervisor"));
    let (status, body) = write(port, "/v1/roles/teacher/start", &json!({}));
    assert_eq!(status, 404, "the teacher is not listed: {body}");
    assert!(
        body["error"]
            .as_str()
            .is_some_and(|error| error.contains("globals.roles")),
        "{body}"
    );
    let (status, _) = write(port, "/v1/roles/director/start", &json!({}));
    assert_eq!(status, 404, "a director is no host role");
}

/// The workspace root, which holds `Taskfile.yml`.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

#[test]
fn the_host_role_tasks_call_the_daemon_and_no_task_runs_claude_bg_or_a_user_adapter() {
    let text = fs::read_to_string(root().join("Taskfile.yml")).expect("read Taskfile.yml");
    let taskfile: serde_yaml::Value = serde_yaml::from_str(&text).expect("Taskfile.yml is YAML");
    for role in ["supervisor", "teacher"] {
        let task = &taskfile["tasks"][format!("{role}:start").as_str()];
        let cmds: Vec<&str> = task["cmds"]
            .as_sequence()
            .into_iter()
            .flatten()
            .filter_map(serde_yaml::Value::as_str)
            .collect();
        assert_eq!(
            cmds,
            [format!("conductor daemon start-role --role {role}")],
            "{role}:start: {task:?}"
        );
    }
    let tasks = taskfile["tasks"].as_mapping().expect("tasks");
    for (name, task) in tasks {
        let run = serde_yaml::to_string(&(&task["cmds"], &task["preconditions"], &task["vars"]))
            .expect("the task as text");
        for gone in [
            "claude --bg",
            "--agent conductor-supervisor",
            "--agent conductor-teacher",
            "claude agents --json | jq -e --arg n conductor-supervisor",
            "claude agents --json | jq -e --arg n conductor-teacher",
        ] {
            assert!(!run.contains(gone), "task {name:?} runs {gone:?}: {run}");
        }
    }
}
