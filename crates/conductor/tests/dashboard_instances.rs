//! `story:dashboard-instance-switch`: `GET /api/instances` lists every instance of the host
//! registry (`~/.b10x/conductor/host/instances.json`) with its dashboard's port and page, the one
//! whose port the server listens on marked current; without a registry it lists this server alone.
//!
//! Each case serves [`dashboard::serve`] in this process on a free port of `127.0.0.1`, its home a
//! directory under this test target's temporary directory that holds the registry.

mod active;

use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use conductor_cli::dashboard::{self, Sources};
use serde_json::{Value, json};

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
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dashboard-instances")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        let case = Self { dir };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.home().join("example-org/conductor")
    }

    /// Writes a host registry of one config file holding `instances`, each a name and a port.
    fn register(&self, instances: &[(&str, u16)]) {
        let host = self.home().join(".b10x/conductor/host");
        fs::create_dir_all(&host).expect("create the host directory");
        let home = self.home().display().to_string();
        let registry = json!({
            "version": "conductor.host.instances/1",
            "files": [{
                "config": format!("{home}/.b10x/conductor/conductor.yaml"),
                "registered_at": "2026-11-07T09:00:00Z",
                "instances": instances.iter().map(|(name, port)| json!({
                    "name": name,
                    "session_prefix": name,
                    "dashboard_port": port,
                    "state": format!("{home}/.b10x/conductor/{name}/state"),
                    "records": format!("{home}/example-org/{name}"),
                    "checkouts": {
                        "root": format!("{home}/example-org"),
                        "trees": format!("{home}/trees/{name}"),
                    },
                })).collect::<Vec<_>>(),
            }],
        });
        fs::write(
            host.join("instances.json"),
            serde_json::to_string_pretty(&registry).expect("the registry as JSON"),
        )
        .expect("write the registry");
    }

    /// A free port of `127.0.0.1`, bound, and the server started on it once `prepare` has run.
    fn served(&self, prepare: impl FnOnce(u16)) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
        let port = listener.local_addr().expect("the bound address").port();
        prepare(port);
        let home = self.home();
        let mut sources = Sources::new(home.clone(), self.records());
        let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
        sources.agents = cat(&home.join("agents.json"));
        sources.sweep = cat(Path::new("/dev/null"));
        sources.gc = cat(Path::new("/dev/null"));
        sources.inspect = cat(Path::new("/dev/null"));
        sources.checkouts = home.join("example-org");
        sources.trees = home.join("trees");
        sources.watch = home.join("watch");
        std::thread::spawn(move || dashboard::serve(listener, sources));
        port
    }
}

fn get(port: u16, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .expect("send the request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read the answer");
    let (head, body) = raw.split_once("\r\n\r\n").expect("a head and a body");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status in {head:?}"));
    (status, body.to_owned())
}

fn instances(port: u16) -> Value {
    let (status, body) = get(port, dashboard::INSTANCES_PATH);
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).expect("the answer is JSON")
}

#[test]
fn three_registered_instances_are_listed_with_their_pages_and_this_one_marked() {
    let case = Case::new("three");
    let (beta, gamma) = (7401, 7402);
    let port =
        case.served(|port| case.register(&[("alpha", port), ("beta", beta), ("gamma", gamma)]));
    let answer = instances(port);
    assert_eq!(
        answer,
        json!({
            "instances": [
                {"name": "alpha", "dashboard_port": port, "url": format!("http://127.0.0.1:{port}/"), "current": true},
                {"name": "beta", "dashboard_port": beta, "url": "http://127.0.0.1:7401/", "current": false},
                {"name": "gamma", "dashboard_port": gamma, "url": "http://127.0.0.1:7402/", "current": false},
            ],
            "errors": [],
        })
    );
}

#[test]
fn without_a_registry_this_server_is_the_one_instance() {
    let case = Case::new("none");
    let port = case.served(|_| {});
    let answer = instances(port);
    let rows = answer["instances"].as_array().expect("a list");
    assert_eq!(rows.len(), 1, "{answer:#}");
    assert_eq!(rows[0]["dashboard_port"], json!(port));
    assert_eq!(rows[0]["current"], json!(true));
    assert!(
        rows[0]["name"]
            .as_str()
            .is_some_and(|name| !name.is_empty())
    );
    assert_eq!(answer["errors"], json!([]));
}

#[test]
fn only_get_is_answered() {
    let case = Case::new("method");
    let port = case.served(|_| {});
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    write!(
        stream,
        "DELETE {} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n",
        dashboard::INSTANCES_PATH
    )
    .expect("send the request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read the answer");
    assert!(raw.starts_with("HTTP/1.1 405"), "{raw}");
}
