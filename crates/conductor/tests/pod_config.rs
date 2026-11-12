//! The config file of one instance pod (design: instance pods, decision D8):
//! `conductor_cli::config::pod::render` writes a complete `conductor.config/2` file that holds the
//! one instance and what its runtime reads: the instance's own sqlite store under the data mount,
//! or the daemon's `postgres://` URL; a `files` secrets source over the directory the pod mounts
//! its Secret on; `daemon.unattended`; and every path of the instance under the data mount, the
//! ones it leaves out filled in there. It holds no `daemon.kubernetes` and no
//! `daemon.control_token`: an instance pod creates no pods and admits no control plane. The file
//! passes `conductor config validate`, and a runtime started on it with the environment a pod
//! gives it comes up.

mod active;
mod common;

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::config::{self, pod};
use serde_yaml::Value;

/// The token of a runtime of a case.
const TOKEN: &str = "6666666666666666666666666666666666666666666666666666666666666666";

/// A case's directory, removed after a passing run.
struct Case {
    dir: PathBuf,
}

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
            .join("pc")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    /// Writes `text` to `name` in the case and answers its path.
    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.dir.join(name);
        fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        fs::write(&path, text).expect("write a case file");
        path
    }

    /// `conductor --config <file> config validate` of the case.
    fn validate(&self, file: &Path) -> std::process::Output {
        common::conductor(self.home())
            .arg("--config")
            .arg(file)
            .args(["config", "validate"])
            .stdin(Stdio::null())
            .output()
            .expect("run config validate")
    }
}

/// The daemon's config file: instance pods (`daemon.kubernetes`), a control token, its own store
/// `store`, and the instances `instances`.
fn daemon_file(store: &str, instances: &str) -> String {
    format!(
        "version: conductor.config/2\n\
         daemon:\n\
         \x20 store: {store:?}\n\
         \x20 control_token: secret:control-plane\n\
         \x20 operators: [{{name: example-operator, role: owner, grants: [start-instances]}}]\n\
         \x20 kubernetes:\n\
         \x20   namespace: conductor-instances\n\
         \x20   image: registry.example.com/conductor:1.0.0\n\
         \x20   owner: example-org\n\
         \x20   api_port: 7310\n\
         \x20   volume: {{size: 20G}}\n\
         \x20   env: [{{name: GH_TOKEN, secret: secret:forge-token}}]\n\
         globals:\n\
         \x20 cadence: {{watch: 120s}}\n\
         instances:\n\
         {instances}"
    )
}

/// The instance `alpha`, its paths left out, with `extra` lines appended.
fn alpha(extra: &str) -> String {
    format!(
        "\x20 - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: [{{github: example-org}}]\n\
         \x20   model_token: secret:alpha-model\n\
         {extra}"
    )
}

/// The rendered file of the only instance of `text`, as text.
fn rendered(case: &Case, text: &str) -> String {
    let config = config::parse(text, &case.home()).expect("the daemon's file is valid");
    pod::render(&config, &config.instances[0])
}

fn yaml(text: &str) -> Value {
    serde_yaml::from_str(text).unwrap_or_else(|error| panic!("not YAML ({error}): {text}"))
}

fn under(path: &str) -> String {
    Path::new(pod::DATA_MOUNT)
        .join(path)
        .to_string_lossy()
        .into_owned()
}

/// The rendered file of an instance that leaves out every path: each is filled in under the data
/// mount, the store is its own sqlite file there, its secrets the Secret's mount, and it holds no
/// key that creates a pod or admits a control plane. It validates.
#[test]
fn the_rendered_file_fills_every_path_under_the_data_mount_and_validates() {
    let case = Case::new("filled");
    let text = rendered(
        &case,
        &daemon_file("sqlite:/srv/conductor.sqlite", &alpha("")),
    );
    let file = yaml(&text);
    assert_eq!(file["version"], "conductor.config/2", "{text}");
    let daemon = &file["daemon"];
    assert!(
        daemon.get("kubernetes").is_none_or(Value::is_null),
        "{text}"
    );
    assert!(
        daemon.get("control_token").is_none_or(Value::is_null),
        "{text}"
    );
    assert_eq!(
        daemon["store"],
        format!("sqlite:{}", under(pod::STORE_FILE)),
        "{text}"
    );
    assert_eq!(daemon["secrets"]["source"], "files", "{text}");
    assert_eq!(daemon["secrets"]["dir"], pod::SECRET_MOUNT, "{text}");
    assert_eq!(daemon["unattended"], true, "{text}");

    let instances = file["instances"]
        .as_sequence()
        .expect("a list of instances");
    assert_eq!(instances.len(), 1, "{text}");
    let instance = &instances[0];
    assert_eq!(instance["name"], "alpha", "{text}");
    assert_eq!(instance["checkouts"]["root"], under("checkouts"), "{text}");
    assert_eq!(instance["checkouts"]["trees"], under("trees"), "{text}");
    assert_eq!(instance["records"], under("records"), "{text}");
    assert_eq!(instance["state"], under("state"), "{text}");
    assert_eq!(instance["cache"], under("cache"), "{text}");
    assert_eq!(instance["model_token"], "secret:alpha-model", "{text}");
    // The global cadence the instance takes is written into it: the pod's file has no globals.
    assert_eq!(instance["cadence"]["watch"], "120s", "{text}");
    assert!(!text.contains("kubernetes"), "{text}");
    assert!(!text.contains("control_token"), "{text}");
    assert!(!text.contains("control-plane"), "{text}");

    let written = case.write("rendered.yaml", &text);
    let output = case.validate(&written);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}\n{text}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A path the instance writes, which lies under the data mount, is kept; a postgres store of the
/// daemon is the instance's too.
#[test]
fn the_rendered_file_keeps_a_written_path_and_the_daemons_postgres_store() {
    let case = Case::new("kept");
    let records = under("own/records");
    let root = under("repositories");
    let text = rendered(
        &case,
        &daemon_file(
            "postgres://conductor@db.example.com/conductor",
            &alpha(&format!(
                "\x20   checkouts: {{root: {root}}}\n\x20   records: {records}\n"
            )),
        ),
    );
    let file = yaml(&text);
    assert_eq!(
        file["daemon"]["store"], "postgres://conductor@db.example.com/conductor",
        "{text}"
    );
    let instance = &file["instances"][0];
    assert_eq!(instance["records"], records.as_str(), "{text}");
    assert_eq!(instance["checkouts"]["root"], root.as_str(), "{text}");
    assert_eq!(instance["checkouts"]["trees"], under("trees"), "{text}");
    assert_eq!(instance["state"], under("state"), "{text}");
    let written = case.write("rendered.yaml", &text);
    let output = case.validate(&written);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}\n{text}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A runtime started on the rendered file with the environment of its pod comes up: the file
/// mounted as the config source, the copy on the data volume as `--config`, a TCP listener and
/// its token, the instance's name and a home of its own. Here the data mount and the Secret's
/// mount are directories of the case ([`pod::render_at`]); the runtime's loop is off.
#[test]
fn a_runtime_comes_up_on_the_rendered_file_with_the_pods_environment() {
    let case = Case::new("runtime");
    let data = case.dir.join("data");
    let secrets = case.dir.join("secrets");
    fs::create_dir_all(&secrets).expect("create the Secret's mount");
    fs::write(secrets.join("alpha-model"), "model-value\n").expect("mount the model token");
    let daemon = daemon_file("sqlite:/srv/conductor.sqlite", &alpha(""));
    let config = config::parse(&daemon, &case.home()).expect("the daemon's file is valid");
    let text = pod::render_at(&config, &config.instances[0], &data, &secrets);
    let source = secrets.join(pod::CONFIG_FILE);
    fs::write(&source, &text).expect("mount the rendered file");
    let programs = case.dir.join("programs");
    fs::create_dir_all(&programs).expect("an empty PATH directory");

    let mut child = common::conductor(case.home())
        .arg("--config")
        .arg(data.join(pod::CONFIG_FILE))
        .args(["instance", "serve"])
        .env("PATH", &programs)
        .env("CONDUCTOR_API_LISTEN", "127.0.0.1:0")
        .env("CONDUCTOR_API_TOKEN", TOKEN)
        .env("CONDUCTOR_CONFIG_SOURCE", &source)
        .env(config::INSTANCE_VARIABLE, "alpha")
        .env("CONDUCTOR_RUNTIME_LOOP", "off")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run instance serve");
    let printed = Arc::new(Mutex::new(String::new()));
    for mut pipe in [
        Box::new(child.stdout.take().expect("stdout")) as Box<dyn std::io::Read + Send>,
        Box::new(child.stderr.take().expect("stderr")),
    ] {
        let printed = Arc::clone(&printed);
        thread::spawn(move || {
            let mut chunk = [0_u8; 1024];
            while let Ok(read @ 1..) = pipe.read(&mut chunk) {
                if let Ok(mut printed) = printed.lock() {
                    printed.push_str(&String::from_utf8_lossy(&chunk[..read]));
                }
            }
        });
    }
    let running = Running(child);
    let said = || printed.lock().map(|text| text.clone()).unwrap_or_default();
    let deadline = Instant::now() + Duration::from_secs(60);
    let address: SocketAddr = loop {
        let text = said();
        if let Some(address) = text
            .lines()
            .find_map(|line| line.split_once("serving /v1/ on "))
            .and_then(|(_, at)| at.trim().parse().ok())
        {
            break address;
        }
        assert!(Instant::now() < deadline, "never served: {text}");
        thread::sleep(Duration::from_millis(50));
    };
    let ask = |target: &str, token: Option<&str>| -> (u16, String) {
        let mut stream = TcpStream::connect(address).expect("connect to the runtime");
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .expect("a read timeout");
        let header = token.map_or_else(String::new, |token| {
            format!("X-Conductor-Token: {token}\r\n")
        });
        stream
            .write_all(
                format!(
                    "GET {target} HTTP/1.1\r\nHost: localhost\r\n{header}Connection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .expect("send");
        let mut answer = String::new();
        stream.read_to_string(&mut answer).expect("read");
        let (head, body) = answer.split_once("\r\n\r\n").unwrap_or((&answer, ""));
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|status| status.parse().ok())
            .unwrap_or_default();
        (status, body.to_owned())
    };
    let (status, body) = ask("/healthz", None);
    assert_eq!(status, 200, "{body}\n{}", said());
    let (status, body) = ask("/v1/settings", Some(TOKEN));
    assert_eq!(status, 200, "{body}\n{}", said());
    let settings: serde_json::Value = serde_json::from_str(&body).expect("the settings as JSON");
    assert_eq!(settings["instance"], "alpha", "{body}");
    assert_eq!(
        fs::read_to_string(data.join(pod::CONFIG_FILE)).expect("the copy on the data volume"),
        text
    );
    drop(running);
}

/// A runtime of a case, killed when dropped.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
