//! The daemon's secrets source (`story:secrets-source`, design architecture-next § 9):
//! `conductor secret put` writes a value read from standard input into the keychain through the
//! `secrets` crates and refuses a name already on record unless `--replace`; `conductor doctor`
//! names the source and whether it answers; the daemon refuses to start on a source it cannot use
//! and prints no value; `GET /v1/secrets`'s rows ([`secret::view`]) name secrets, their source and
//! the instances that use them, never a value. Every case runs on the fake keychain
//! `common::conductor` names under the case's home; none reaches the OS keychain.

mod active;
mod common;

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::secret;
use conductor_model::secret::Source;

/// A value no output may carry.
const VALUE: &str = "example-org-value-4f1c9a7d";

struct Case {
    dir: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("secrets-source")
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

    /// Writes the config file: `secrets` as `daemon.secrets`, a SQLite store, and one instance.
    fn config(&self, port: u16, secrets: &str) {
        let root = self.dir.join("alpha");
        let text = format!(
            "version: conductor.config/2\ndaemon: {{port: {port}, store: {store:?}, secrets: \
             {secrets}}}\ninstances:\n  - name: alpha\n    placement: box-a\n    \
             session_prefix: alpha\n    sources: [{{github: example-org}}]\n    \
             checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n    records: {records:?}\n    \
             state: {state:?}\n    cache: {cache:?}\n",
            store = format!("sqlite:{}", self.dir.join("shared.sqlite").display()),
            checkouts = root.join("checkouts"),
            trees = root.join("trees"),
            records = root.join("records"),
            state = root.join("state"),
            cache = root.join("cache"),
        );
        fs::write(self.file(), text).expect("write the config file");
    }

    fn conductor(&self) -> Command {
        let mut command = common::conductor(self.home());
        command.arg("--config").arg(self.file());
        command
    }

    /// `conductor secret put` with `args`, the value on standard input.
    fn put(&self, args: &[&str], value: &str) -> Output {
        let mut child = self
            .conductor()
            .args(["secret", "put"])
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run conductor secret put");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(format!("{value}\n").as_bytes())
            .expect("write the value");
        child.wait_with_output().expect("wait for secret put")
    }
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("a free port")
}

#[test]
fn a_put_is_stored_refused_again_without_replace_and_never_printed() {
    let case = Case::new("put");
    case.config(7300, "{source: keychain}");

    let first = case.put(&["--name", "example-org-forge"], VALUE);
    assert_eq!(first.status.code(), Some(0), "{}", said(&first));
    assert!(
        said(&first).contains("example-org-forge"),
        "{}",
        said(&first)
    );

    let again = case.put(&["--name", "example-org-forge"], "another-value");
    assert_eq!(again.status.code(), Some(1), "{}", said(&again));
    let refusal = String::from_utf8_lossy(&again.stderr);
    assert!(refusal.contains("SecretExists"), "{refusal}");
    assert_eq!(refusal.lines().count(), 1, "{refusal}");

    let replaced = case.put(
        &["--name", "example-org-forge", "--replace"],
        "another-value",
    );
    assert_eq!(replaced.status.code(), Some(0), "{}", said(&replaced));

    for output in [&first, &again, &replaced] {
        let text = said(output);
        assert!(!text.contains(VALUE), "a value was printed: {text}");
        assert!(
            !text.contains("another-value"),
            "a value was printed: {text}"
        );
    }
    let keychain = fs::read_to_string(case.home().join(".test-keychain.ron"))
        .expect("the fake keychain holds the put");
    assert!(keychain.contains("example-org-forge"), "{keychain}");
}

#[test]
fn an_agent_session_puts_no_secret() {
    let case = Case::new("agent");
    case.config(7300, "{source: keychain}");
    let mut child = case
        .conductor()
        .env(conductor_cli::actor::SESSION_VARIABLE, "alpha-controller")
        .args(["secret", "put", "--name", "example-org-forge"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run conductor secret put");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(VALUE.as_bytes())
        .expect("write the value");
    let output = child.wait_with_output().expect("wait");
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ActorNotHuman"),
        "{}",
        said(&output)
    );
    assert!(
        !case.home().join(".test-keychain.ron").exists()
            || !fs::read_to_string(case.home().join(".test-keychain.ron"))
                .unwrap_or_default()
                .contains("example-org-forge"),
        "an agent's put reached the keychain"
    );
}

#[test]
fn a_put_to_the_remote_source_answers_source_unreachable() {
    let case = Case::new("remote-put");
    case.config(7300, "{source: remote, url: \"https://secrets.example\"}");
    let output = case.put(&["--name", "example-org-forge"], VALUE);
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    let refusal = String::from_utf8_lossy(&output.stderr);
    assert!(refusal.contains("SourceUnreachable"), "{refusal}");
    assert!(refusal.contains("Remote"), "{refusal}");
}

#[test]
fn doctor_names_the_secrets_source_and_whether_it_answers() {
    let case = Case::new("doctor");
    case.config(7300, "{source: keychain}");
    let output = case.conductor().arg("doctor").output().expect("run doctor");
    let text = said(&output);
    let line = text
        .lines()
        .find(|line| line.starts_with("secrets: "))
        .unwrap_or_else(|| panic!("no secrets line: {text}"));
    assert!(line.contains("keychain"), "{line}");
    assert!(line.ends_with("(answers)"), "{line}");

    case.config(7300, "{source: remote, url: \"https://secrets.example\"}");
    let output = case.conductor().arg("doctor").output().expect("run doctor");
    let text = said(&output);
    let line = text
        .lines()
        .find(|line| line.starts_with("secrets: "))
        .unwrap_or_else(|| panic!("no secrets line: {text}"));
    assert!(line.contains("remote"), "{line}");
    assert!(line.contains("does not answer"), "{line}");
}

/// Runs `daemon serve` until it prints its serving line or exits, then stops it; its output.
fn serve(case: &Case) -> (Option<i32>, String) {
    // An empty PATH: no systemctl or systemd-run, so the reconcile pass after the start reaches
    // no service manager of the host.
    let empty = case.dir.join("no-programs");
    fs::create_dir_all(&empty).expect("create an empty PATH directory");
    let mut child = case
        .conductor()
        .env("PATH", &empty)
        .args(["daemon", "serve", "--state-dir"])
        .arg(case.dir.join("daemon"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run daemon serve");
    let mut stdout = child.stdout.take().expect("stdout");
    let reader = thread::spawn(move || {
        let mut text = String::new();
        let mut byte = [0_u8; 1];
        while let Ok(1) = stdout.read(&mut byte) {
            text.push(char::from(byte[0]));
            if text.contains("serving http") && text.ends_with('\n') {
                break;
            }
        }
        text
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    let code = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status.code();
        }
        if reader.is_finished() || Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(50));
    };
    let mut text = reader.join().expect("read stdout");
    let mut stderr = String::new();
    let _ = child
        .stderr
        .take()
        .expect("stderr")
        .read_to_string(&mut stderr);
    text.push_str(&stderr);
    (code, text)
}

#[test]
fn the_daemon_refuses_to_start_on_a_remote_source_without_url() {
    let case = Case::new("remote-no-url");
    case.config(free_port(), "{source: remote}");
    let (code, text) = serve(&case);
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("SourceUnusable"), "{text}");
    assert!(text.contains("daemon.secrets.url"), "{text}");
}

#[test]
fn no_daemon_log_line_carries_a_secret_value() {
    let case = Case::new("daemon-log");
    case.config(free_port(), "{source: keychain}");
    let put = case.put(&["--name", "example-org-model"], VALUE);
    assert_eq!(put.status.code(), Some(0), "{}", said(&put));

    let (code, text) = serve(&case);
    assert_eq!(code, None, "the daemon did not keep serving: {text}");
    assert!(text.contains("/v1/ on http://"), "{text}");
    let line = text
        .lines()
        .find(|line| line.contains("secrets: keychain"))
        .unwrap_or_else(|| panic!("the daemon named no secrets source: {text}"));
    assert!(line.contains("(answers)"), "{line}");
    assert!(
        !text.contains(VALUE),
        "the daemon's log carries a value: {text}"
    );
}

#[test]
fn the_secrets_view_names_secrets_sources_and_instances_never_values() {
    let case = Case::new("view");
    case.config(7300, "{source: keychain}");
    let put = case.put(&["--name", "example-org-forge"], VALUE);
    assert_eq!(put.status.code(), Some(0), "{}", said(&put));

    let text = fs::read_to_string(case.file()).expect("read the config");
    let store = conductor_cli::store::open_url(
        &format!("sqlite:{}", case.dir.join("shared.sqlite").display()),
        conductor_cli::daemon::TENANT,
    )
    .expect("open the shared store");
    let rows = secret::view(&store, &text, Source::Keychain);
    let row = rows
        .iter()
        .find(|row| row.name == "example-org-forge")
        .unwrap_or_else(|| panic!("the put secret is no row: {rows:?}"));
    assert_eq!(row.source, "keychain");
    let json = serde_json::to_string(&rows).expect("rows as JSON");
    assert!(!json.contains(VALUE), "{json}");
    assert!(json.contains("\"instances\""), "{json}");
}
