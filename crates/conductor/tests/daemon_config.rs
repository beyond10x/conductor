//! `conductor daemon serve --daemon-config <file>` (design: instance pods): the second file holds
//! only `version` and `daemon`, such as a ConfigMap mounted into the daemon's pod. At every start,
//! before it reads its config file, the daemon writes that file anew from the two: `version` and
//! `daemon` from the daemon-config file, everything else kept from the config file as it is, and
//! no instance where there was no config file yet. A daemon-config file that does not validate
//! stops the start in one line naming the file and the path.
//!
//! No case reaches the host's service manager: a daemon started here runs on a `PATH` of the
//! case's own, which holds stand-ins that know and start no unit.

mod common;

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_yaml::Value;

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
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dc")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("secrets")).expect("create the secrets directory");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    /// The config file on the daemon's volume, in a directory that does not exist yet.
    fn file(&self) -> PathBuf {
        self.dir.join("volume").join("conductor.yaml")
    }

    /// The daemon-config file, as a ConfigMap mounts it.
    fn daemon_config(&self) -> PathBuf {
        self.dir.join("map").join("daemon.yaml")
    }

    /// Writes the daemon-config file: the daemon on `port`, its store in the case, its secrets a
    /// `files` directory, its instance pods owned by `owner`, with `extra` appended.
    fn write_daemon_config(&self, port: u16, owner: &str, extra: &str) {
        let text = format!(
            "version: conductor.config/2\n\
             daemon:\n\
             \x20 port: {port}\n\
             \x20 store: \"sqlite:{store}\"\n\
             \x20 secrets: {{source: files, dir: {secrets:?}}}\n\
             \x20 kubernetes:\n\
             \x20   namespace: conductor-instances\n\
             \x20   image: registry.example.com/conductor:1.0.0\n\
             \x20   owner: {owner}\n\
             \x20   api_port: 7310\n\
             \x20   volume: {{size: 20G}}\n\
             {extra}",
            store = self.dir.join("volume").join("conductor.sqlite").display(),
            secrets = self.dir.join("secrets"),
        );
        let path = self.daemon_config();
        fs::create_dir_all(path.parent().expect("a parent")).expect("create the map's directory");
        fs::write(path, text).expect("write the daemon-config file");
    }

    /// The config file as YAML.
    fn written(&self) -> Value {
        let text = fs::read_to_string(self.file()).expect("the config file was written");
        serde_yaml::from_str(&text).expect("the config file is YAML")
    }

    /// The daemon-config file as YAML.
    fn mapped(&self) -> Value {
        let text = fs::read_to_string(self.daemon_config()).expect("the daemon-config file");
        serde_yaml::from_str(&text).expect("the daemon-config file is YAML")
    }

    /// The `PATH` of a daemon of the case: a `systemctl` that knows no unit and a `systemd-run`
    /// that starts none.
    fn programs(&self) -> PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let programs = self.dir.join("programs");
        fs::create_dir_all(&programs).expect("create the PATH directory");
        for (name, script) in [
            (
                "systemctl",
                "#!/bin/sh\necho LoadState=not-found\necho ActiveState=inactive\n",
            ),
            (
                "systemd-run",
                "#!/bin/sh\necho 'this case starts no unit' >&2\nexit 1\n",
            ),
        ] {
            let program = programs.join(name);
            fs::write(&program, script).expect("write the stand-in");
            fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
                .expect("make the stand-in runnable");
        }
        programs
    }

    /// `conductor --config <file> daemon serve --daemon-config <map> <extra>` of the case.
    fn command(&self, extra: &[&str]) -> std::process::Command {
        let mut command = common::conductor(self.home());
        command
            .arg("--config")
            .arg(self.file())
            .env("PATH", self.programs())
            .env_remove("XDG_RUNTIME_DIR")
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .args(["daemon", "serve", "--state-dir"])
            .arg(self.dir.join("daemon"))
            .arg("--daemon-config")
            .arg(self.daemon_config())
            .args(extra)
            .stdin(Stdio::null());
        command
    }

    /// A start that ends by itself, within its own time.
    fn refused(&self, extra: &[&str]) -> Output {
        self.command(extra).output().expect("run daemon serve")
    }
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("a free port")
}

/// `GET target` on `port`: its status and body; none while nothing listens there.
fn get(port: u16, target: &str) -> Option<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a read timeout");
    stream
        .write_all(
            format!("GET {target} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .ok()?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer).ok()?;
    let (head, body) = answer.split_once("\r\n\r\n")?;
    let status = head.split(' ').nth(1)?.parse().ok()?;
    Some((status, body.to_owned()))
}

/// A `daemon serve` of a case, killed when dropped, with what it printed.
struct Served {
    child: Child,
    printed: Arc<Mutex<String>>,
}

impl Served {
    /// Starts the daemon of `case` on `port` and waits until `/readyz` answers `200`.
    fn start(case: &Case, port: u16) -> Self {
        let mut child = case
            .command(&[])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run daemon serve");
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
        let mut served = Self { child, printed };
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if get(port, "/readyz").is_some_and(|(status, _)| status == 200) {
                return served;
            }
            if let Ok(Some(status)) = served.child.try_wait() {
                panic!("daemon serve exited {status}: {}", served.printed());
            }
            assert!(
                Instant::now() < deadline,
                "/readyz never answered 200: {:?}\n{}",
                get(port, "/readyz"),
                served.printed()
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn printed(&self) -> String {
        self.printed
            .lock()
            .map(|printed| printed.clone())
            .unwrap_or_default()
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Where no config file is yet, the start creates it, with its directory: `version` and
/// `daemon` from the daemon-config file and no instance. The daemon then serves with no
/// instance and is ready.
#[test]
fn a_start_creates_the_config_file_with_no_instance_and_is_ready() {
    let case = Case::new("created");
    let port = free_port();
    case.write_daemon_config(port, "example-org", "");
    assert!(!case.file().exists());

    let served = Served::start(&case, port);
    let written = case.written();
    let mapped = case.mapped();
    assert_eq!(written["version"], mapped["version"], "{written:?}");
    assert_eq!(written["daemon"], mapped["daemon"], "{written:?}");
    assert_eq!(
        written["instances"],
        Value::Sequence(Vec::new()),
        "{written:?}"
    );
    let (status, body) = get(port, "/v1/instances").expect("the daemon answers");
    assert_eq!(status, 200, "{body}\n{}", served.printed());
}

/// The instances the config file holds, and its `globals`, are kept across two starts while the
/// daemon-config file's `daemon` section changes; each start writes the section anew.
#[test]
fn instances_are_kept_across_two_starts_while_the_daemon_section_changes() {
    let case = Case::new("kept");
    fs::create_dir_all(case.file().parent().expect("a parent")).expect("create the volume");
    fs::write(
        case.file(),
        "version: conductor.config/2\n\
         daemon: {port: 1, store: \"sqlite:/nowhere/conductor.sqlite\"}\n\
         globals:\n\
         \x20 cadence: {watch: 120s}\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: [{github: example-org}]\n",
    )
    .expect("write the config file");
    let before: Value =
        serde_yaml::from_str(&fs::read_to_string(case.file()).expect("the config file"))
            .expect("YAML");

    let first = free_port();
    case.write_daemon_config(first, "example-org", "");
    drop(Served::start(&case, first));
    let written = case.written();
    assert_eq!(written["daemon"], case.mapped()["daemon"], "{written:?}");
    assert_eq!(written["instances"], before["instances"], "{written:?}");
    assert_eq!(written["globals"], before["globals"], "{written:?}");

    let second = free_port();
    case.write_daemon_config(second, "another-owner", "");
    drop(Served::start(&case, second));
    let written = case.written();
    assert_eq!(written["daemon"], case.mapped()["daemon"], "{written:?}");
    assert_eq!(written["daemon"]["kubernetes"]["owner"], "another-owner");
    assert_eq!(written["daemon"]["port"], second);
    assert_eq!(written["instances"], before["instances"], "{written:?}");
    assert_eq!(written["globals"], before["globals"], "{written:?}");
}

/// A daemon-config file holds `version` and `daemon` only: another top-level key is refused by
/// its path, and a `daemon` section that does not validate by the path of its key, each in one
/// line naming the file; the config file is not written.
#[test]
fn a_daemon_config_file_that_does_not_validate_stops_the_start_by_its_path() {
    let case = Case::new("refused");
    for (name, extra, port, path) in [
        ("foreign", "instances: []\n", 7, "instances"),
        ("globals", "globals: {}\n", 7, "globals"),
        ("port", "", 0, "daemon.port"),
    ] {
        case.write_daemon_config(port, "example-org", extra);
        let output = case.refused(&[]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{name}: {stderr}");
        let lines: Vec<&str> = stderr.lines().collect();
        assert_eq!(lines.len(), 1, "{name}: one line: {stderr}");
        assert!(
            lines[0].contains(&case.daemon_config().display().to_string())
                && lines[0].contains(&format!("{path}: ")),
            "{name}: names the file and {path}: {stderr}"
        );
        assert!(
            !case.file().exists(),
            "{name}: the config file is not written"
        );
    }

    case.write_daemon_config(7, "Example_Org", "");
    let output = case.refused(&[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("daemon.kubernetes.owner: "),
        "names the key: {stderr}"
    );
    assert!(!case.file().exists());

    // Not with `--install` or `--uninstall`: the unit's start would read no daemon-config file.
    case.write_daemon_config(7, "example-org", "");
    let output = case.refused(&["--install"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("--daemon-config") && stderr.contains("--install"),
        "{stderr}"
    );
    assert!(!case.file().exists());
}
