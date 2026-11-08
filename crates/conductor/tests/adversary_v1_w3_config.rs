//! Adversarial cases on `story:config-globals` (wave v1-3): a secret written into the config file
//! through a form the `secret:<name>` rule does not read, the `PUT /v1/config` route the story's
//! acceptance names, the host roles `conductor config migrate` moves to `globals.roles`, the
//! `placement` key at the global level, and concurrent `conductor config put` runs losing a
//! change each reported as committed.
//!
//! Each case is a directory of its own in this test target's temporary directory, holding a home
//! directory `home/` the binary runs with; no case reads the real home's config.

mod common;

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::thread;

use serde_json::Value;

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
            .join("adversary_v1_w3_config")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.dir.join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        fs::write(&path, text).expect("write a case file");
        path
    }

    fn command(&self, file: &Path, words: &[&str]) -> Command {
        let mut command = common::conductor(self.home());
        command
            .current_dir(&self.dir)
            .arg("--config")
            .arg(file)
            .args(words)
            .envs(identity())
            .stdin(Stdio::null());
        command
    }

    fn run(&self, file: &Path, words: &[&str]) -> Output {
        self.command(file, words)
            .output()
            .expect("the conductor binary runs")
    }
}

fn identity() -> [(&'static str, &'static str); 4] {
    [
        ("GIT_AUTHOR_NAME", "example"),
        ("GIT_AUTHOR_EMAIL", "example@example.invalid"),
        ("GIT_COMMITTER_NAME", "example"),
        ("GIT_COMMITTER_EMAIL", "example@example.invalid"),
    ]
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

const INSTANCE: &str = "instances:\n\
                        \x20 - name: one\n\
                        \x20   placement: systemd\n\
                        \x20   session_prefix: one\n\
                        \x20   sources: [{github: example-org}]\n\
                        \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n";

/// A postgres URL takes its password in the query as well as in the user information
/// (`?password=`, as libpq reads it). The file then holds the password in clear, and
/// `config show` prints it.
#[test]
fn a_store_url_password_in_the_query_is_refused_like_one_in_the_user_information() {
    let case = Case::new("query-password");
    let file = case.write(
        "conductor.yaml",
        &format!(
            "version: conductor.config/2\n\
             daemon: {{store: \"postgres://conductor@db.example/conductor?password=hunter2\"}}\n\
             {INSTANCE}"
        ),
    );
    let output = case.run(&file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("daemon.store"), "{said}");
    assert!(!said.contains("hunter2"), "the value is echoed: {said}");
}

/// The class of the two cases around it: a password in any URL of the file, in its user
/// information or its query, is refused by its YAML path and never echoed: the secrets source's
/// URL with `?password=`, and a URL in a key no rule reads as one (a source's `host`).
#[test]
fn a_password_in_any_url_of_the_file_is_refused_by_its_path() {
    let case = Case::new("any-url-password");
    for (body, path) in [
        (
            format!(
                "version: conductor.config/2\n\
                 daemon:\n\
                 \x20 store: \"sqlite:~/daemon/conductor.sqlite\"\n\
                 \x20 secrets: {{source: remote, url: \"https://secrets.example/v1?user=a&password=hunter2\"}}\n\
                 {INSTANCE}"
            ),
            "daemon.secrets.url",
        ),
        (
            "version: conductor.config/2\n\
             daemon: {store: \"sqlite:~/daemon/conductor.sqlite\"}\n\
             instances:\n\
             \x20 - name: one\n\
             \x20   placement: systemd\n\
             \x20   session_prefix: one\n\
             \x20   sources: [{gitlab: example-org, host: \"https://bot:hunter2@gitlab.example\"}]\n\
             \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n"
                .to_owned(),
            "instances[0].sources[0].host",
        ),
    ] {
        let file = case.write("conductor.yaml", &body);
        let output = case.run(&file, &["config", "validate"]);
        assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
        let said = String::from_utf8_lossy(&output.stderr);
        assert!(said.contains(path), "{path}: {said}");
        assert!(!said.contains("hunter2"), "the value is echoed: {said}");
    }
}

/// The remote secrets source's URL with a password in its user information is a secret in the
/// file, which the story says never holds one.
#[test]
fn a_secrets_url_with_a_password_is_refused() {
    let case = Case::new("secrets-url");
    let file = case.write(
        "conductor.yaml",
        &format!(
            "version: conductor.config/2\n\
             daemon:\n\
             \x20 store: \"sqlite:~/daemon/conductor.sqlite\"\n\
             \x20 secrets: {{source: remote, url: \"https://admin:hunter2@secrets.example\"}}\n\
             {INSTANCE}"
        ),
    );
    let output = case.run(&file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("daemon.secrets.url"), "{said}");
    assert!(!said.contains("hunter2"), "the value is echoed: {said}");
}

/// Acceptance: "`PUT /v1/config` validates, writes the file, commits it ...". The daemon's API
/// serves no such route.
///
/// Since `story:instance-api` the route is served through the daemon's router: a loopback write
/// carries the listener's own `Origin` and the daemon's token, and is done for the admitted
/// operator. The change is answered with the `ConfigChanged` it publishes.
#[test]
fn the_daemon_api_serves_put_v1_config() {
    const TOKEN: &str = "3333333333333333333333333333333333333333333333333333333333333333";
    let case = Case::new("put-route");
    let file = case.write(
        "conductor.yaml",
        &format!(
            "version: conductor.config/2\n\
             daemon: {{store: \"sqlite:~/daemon/conductor.sqlite\"}}\n\
             {INSTANCE}"
        ),
    );
    let store = format!("sqlite:{}", case.dir.join("shared.sqlite").display());
    let api = Arc::new(
        conductor_cli::daemon::api::Api::new(TOKEN.to_owned(), "operator-one".to_owned(), None)
            .with_config(file.clone(), case.home(), store),
    );
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("address").port();
    thread::spawn(move || conductor_cli::daemon::api::serve(&listener, &api));
    let body = r#"{"level":"global","path":"cadence.watch","value":"5m","reason":"faster"}"#;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    write!(
        stream,
        "PUT /v1/config HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\n\
         X-Conductor-Token: {TOKEN}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("send");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("read");
    let status = answer.lines().next().unwrap_or_default().to_owned();
    assert!(
        !status.contains(" 404 ") && !status.contains(" 405 "),
        "PUT /v1/config is not served: {answer}"
    );
    assert!(status.contains(" 200 "), "{answer}");
    let (_, json) = answer.split_once("\r\n\r\n").expect("a body");
    let changed: Value = serde_json::from_str(json.trim()).expect("json");
    assert_eq!(changed["path"], "cadence.watch", "{changed}");
    assert_eq!(changed["value"], "5m", "{changed}");
    assert_eq!(changed["reason"], "faster", "{changed}");
    assert!(
        changed["actor"]
            .as_str()
            .is_some_and(|actor| actor.contains("operator-one")),
        "{changed}"
    );
    assert!(
        fs::read_to_string(&file)
            .expect("read")
            .contains("watch: 5m"),
        "the file is not changed"
    );
}

/// `conductor config migrate` moves the host file's roles to `globals.roles`; the host file's
/// roles are `maintainer` and `bridge`. A global role reaches an instance only by the name of a
/// role the instance has, and no instance has those, so the migrated host role changes nothing:
/// the model it names reaches no role of any instance.
#[test]
fn the_host_roles_migrate_moves_to_globals_reach_an_instance() {
    let case = Case::new("migrate-host-roles");
    let file = case.write(
        "conductor.yaml",
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   session_prefix: one\n\
         \x20   sources: [{github: example-org}]\n\
         \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n",
    );
    case.write(
        "home/.b10x/conductor/host/host.yaml",
        "version: conductor.host-config/1\n\
         roles:\n\
         \x20 - {role: maintainer, harness: claude, model: host-model}\n\
         \x20 - {role: bridge, harness: claude, model: host-model}\n",
    );
    let output = case.run(&file, &["config", "migrate"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    // A host role no instance has is kept, and the migration says it reaches no instance.
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        said.contains("globals.roles[1]") && said.contains("applies to no instance"),
        "{said}"
    );
    assert!(!said.contains("globals.roles[0]: maintainer"), "{said}");
    let output = case.run(
        &file,
        &[
            "config",
            "show",
            "--effective",
            "--instance",
            "one",
            "--format",
            "json",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let rows: Value = serde_json::from_slice(&output.stdout).expect("json");
    let reached = rows
        .as_array()
        .expect("rows")
        .iter()
        .any(|row| row["path"] == "model" && row["value"] == "host-model");
    assert!(
        reached,
        "the migrated host role reaches no role of the instance:\n{}\n{}",
        fs::read_to_string(&file).expect("read"),
        String::from_utf8_lossy(&output.stdout)
    );
}

/// Acceptance: "`roles.<role>.kind` and `placement` are keys at every level". By the wave's
/// decision placement stays per instance: `globals.placement` is refused as an unknown key, by
/// its YAML path.
#[test]
fn placement_at_the_global_level_is_refused_by_its_path() {
    let case = Case::new("global-placement");
    let file = case.write(
        "conductor.yaml",
        "version: conductor.config/2\n\
         daemon: {store: \"sqlite:~/daemon/conductor.sqlite\"}\n\
         globals: {placement: systemd}\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   session_prefix: one\n\
         \x20   sources: [{github: example-org}]\n\
         \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n",
    );
    let output = case.run(
        &file,
        &[
            "config",
            "show",
            "--effective",
            "--instance",
            "one",
            "--format",
            "json",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("globals.placement"), "{}", describe(&output));
}

/// Eight `conductor config put` runs at once, each on another instance's key. Every run that
/// exits 0 said its change is written and committed; each such change is in the file at the end.
/// `put_config` reads, writes and commits with no lock, and a run whose commit fails (git's index
/// lock held by another run) writes back the text it read first, over the others' changes.
#[test]
fn concurrent_puts_reported_committed_are_all_in_the_file() {
    let case = Case::new("concurrent-put");
    let names: Vec<String> = (0..8).map(|index| format!("n{index}")).collect();
    let mut text = "version: conductor.config/2\n\
                    daemon: {store: \"sqlite:~/daemon/conductor.sqlite\"}\n\
                    instances:\n"
        .to_owned();
    for name in &names {
        text.push_str(&format!(
            "  - name: {name}\n    placement: systemd\n    session_prefix: {name}\n    \
             sources: [{{github: example-org}}]\n    \
             checkouts: {{root: ~/{name}, trees: ~/{name}-trees}}\n"
        ));
    }
    let file = case.write("conductor.yaml", &text);
    for args in [
        &["init", "-q"][..],
        &["add", "conductor.yaml"],
        &["commit", "-q", "-m", "the config"],
    ] {
        let output = Command::new("git")
            .arg("-C")
            .arg(&case.dir)
            .args(args)
            .envs(identity())
            .output()
            .expect("git runs");
        assert!(output.status.success(), "{}", describe(&output));
    }
    // The store first, so the runs do not race to create it.
    let first = case.run(
        &file,
        &[
            "config",
            "put",
            "--level",
            "global",
            "--path",
            "cadence.watch",
            "--value",
            "9m",
            "--reason",
            "create the store",
        ],
    );
    assert_eq!(first.status.code(), Some(0), "{}", describe(&first));

    let mut children = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let value = format!("{}m", 40 + index);
        let child = case
            .command(
                &file,
                &[
                    "config",
                    "put",
                    "--level",
                    "instance",
                    "--instance",
                    name,
                    "--path",
                    "thresholds.silent_session",
                    "--value",
                    &value,
                    "--reason",
                    "concurrent",
                ],
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn conductor");
        children.push((index, value, child));
    }
    let mut succeeded = Vec::new();
    for (index, value, child) in children {
        let output = child.wait_with_output().expect("wait");
        if output.status.success() {
            succeeded.push((index, value));
        }
    }
    let written: Value =
        serde_yaml::from_str(&fs::read_to_string(&file).expect("read")).expect("yaml");
    let lost: Vec<String> = succeeded
        .iter()
        .filter(|(index, value)| {
            written["instances"][*index]["thresholds"]["silent_session"].as_str()
                != Some(value.as_str())
        })
        .map(|(index, _)| names[*index].clone())
        .collect();
    assert!(
        lost.is_empty(),
        "runs that exited 0 whose change is not in the file: {lost:?} (of {} that exited 0)",
        succeeded.len()
    );
}
