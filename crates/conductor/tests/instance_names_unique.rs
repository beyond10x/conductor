//! `story:instance-names-unique`: no two instances on a host share a session name, a unit, a
//! dashboard port or a state or records directory, whether they are in one config file or in
//! several (`docs/design/roles-and-surfaces.md` § 5, I1 to I5).
//!
//! - `config validate` refuses an instance without a `session_prefix` and names the prefix
//!   `config migrate` writes; `config migrate` writes it and keeps every other line.
//! - The host registry, `~/.b10x/conductor/host/instances.json`, lists every config file
//!   `config validate` found valid; a second file whose instance holds a prefix, a port, a state
//!   or a records directory a registered instance holds is refused, naming both.
//! - Two instances of one file never share a state or records directory.
//! - Two distinct config files both validate, and both start their dashboards.
//! - The guard refuses, and records, a message to a session of another registered instance.
//!
//! Every case is a directory of its own under the target's temporary directory, with `home/` as
//! `HOME`; nothing reads the real home.

mod common;

use std::fs;
use std::io::{BufRead as _, BufReader, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// The variable naming the config file, as `common::conductor` removes it.
const CONFIG_VARIABLE: &str = "CONDUCTOR_CONFIG";
use serde_json::{Map, Value};

/// The host registry under the case's home.
const REGISTRY: &str = ".b10x/conductor/host/instances.json";

fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("instance_names_unique")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(dir.join("home")).expect("create the case's home");
    dir.canonicalize().expect("the case's directory")
}

/// One instance of a config file: its name, and the lines it writes besides `name`, `sources`
/// and `checkouts` (each indented as an instance's key), such as `session_prefix: a`.
fn instance(case: &Path, name: &str, extra: &[&str]) -> String {
    let mut text = format!(
        "  - name: {name}\n\
         \x20   sources: [{{github: example-org}}]\n\
         \x20   checkouts: {{root: {root}, trees: {trees}}}\n",
        root = case.join(format!("{name}-root")).display(),
        trees = case.join(format!("{name}-trees")).display(),
    );
    for line in extra {
        text.push_str(&format!("    {line}\n"));
    }
    text
}

/// Writes a config file `file` under the case directory holding `instances`, and answers its
/// path.
fn config(case: &Path, file: &str, instances: &[String]) -> PathBuf {
    let path = case.join(file);
    fs::write(
        &path,
        format!(
            "version: conductor.config/1\ninstances:\n{}",
            instances.concat()
        ),
    )
    .expect("write the config file");
    path
}

/// `conductor <args>` with `CONDUCTOR_CONFIG` naming `config`, from the case directory.
fn run(case: &Path, config: &Path, args: &[&str]) -> Output {
    common::conductor(case.join("home"))
        .args(args)
        .env(CONFIG_VARIABLE, config)
        .current_dir(case)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

fn shown(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The host registry, or `None` when it is not there.
fn registry(case: &Path) -> Option<Value> {
    let text = fs::read_to_string(case.join("home").join(REGISTRY)).ok()?;
    Some(serde_json::from_str(&text).expect("the registry is JSON"))
}

/// The config files the host registry lists.
fn registered(case: &Path) -> Vec<String> {
    registry(case)
        .map(|registry| {
            registry["files"]
                .as_array()
                .expect("the registry's files")
                .iter()
                .map(|file| file["config"].as_str().expect("a file's path").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn text_of(path: &Path) -> String {
    path.display().to_string()
}

/// Acceptance 1: an instance without a `session_prefix` is refused by its path, and the refusal
/// names the prefix `conductor config migrate` writes. Nothing is registered.
#[test]
fn config_validate_refuses_an_instance_without_a_session_prefix() {
    let case = case_dir("missing-prefix");
    let file = config(
        &case,
        "conductor.yaml",
        &[instance(&case, "example_org", &[])],
    );
    let output = run(&case, &file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let said = stderr(&output);
    assert!(
        said.contains("instances[0].session_prefix: missing")
            && said.contains("conductor config migrate")
            && said.contains("\"example-org\""),
        "{said}"
    );
    assert!(registered(&case).is_empty(), "{:?}", registry(&case));

    // An explicit null is missing too.
    let file = config(
        &case,
        "conductor.yaml",
        &[instance(&case, "alpha", &["session_prefix: null"])],
    );
    let output = run(&case, &file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert!(
        stderr(&output).contains("instances[0].session_prefix: missing"),
        "{}",
        shown(&output)
    );
}

/// Acceptance 1: `config migrate --check` prints the prefix of each instance without one and
/// writes nothing; `config migrate` writes it, keeps every other line and comment, and the file
/// validates. A prefix another instance holds, or nests with, gets the first free number.
#[test]
fn config_migrate_writes_the_suggested_prefix_and_keeps_every_other_line() {
    let case = case_dir("migrate");
    let first = instance(
        &case,
        "b",
        &["# the operator's own note", "session_prefix: null"],
    );
    let file = config(
        &case,
        "conductor.yaml",
        &[
            first,
            instance(
                &case,
                "beta",
                &["session_prefix: b", "dashboard_port: 7400"],
            ),
        ],
    );
    let before = fs::read_to_string(&file).expect("the file");

    let output = run(&case, &file, &["config", "migrate", "--check"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "b: session_prefix b2\n",
        "{}",
        shown(&output)
    );
    assert_eq!(fs::read_to_string(&file).expect("the file"), before);

    let output = run(&case, &file, &["config", "migrate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let after = fs::read_to_string(&file).expect("the file");
    assert_eq!(
        after,
        before.replace("session_prefix: null", "session_prefix: b2"),
        "only the prefix's line changed"
    );
    let output = run(&case, &file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));

    // Nothing left to write: the file is unchanged and nothing is printed.
    let output = run(&case, &file, &["config", "migrate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(output.stdout.is_empty(), "{}", shown(&output));
    assert_eq!(fs::read_to_string(&file).expect("the file"), after);

    // An instance that writes no `session_prefix` line gets one after its name. Its port is its
    // own: 7313 is the registered instance b's.
    let file = config(
        &case,
        "other.yaml",
        &[instance(&case, "gamma", &["dashboard_port: 7401"])],
    );
    let output = run(&case, &file, &["config", "migrate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let written = fs::read_to_string(&file).expect("the file");
    assert!(
        written.contains("  - name: gamma\n    session_prefix: gamma\n"),
        "{written}"
    );
    let output = run(&case, &file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
}

/// Acceptance 2: a valid file is written to the host registry with each instance's prefix,
/// port, state and records.
#[test]
fn config_validate_writes_the_file_to_the_host_registry() {
    let case = case_dir("registry");
    let file = config(
        &case,
        "conductor.yaml",
        &[instance(&case, "alpha", &["session_prefix: a"])],
    );
    let output = run(&case, &file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let registry = registry(&case).expect("the host registry is written");
    assert_eq!(
        registry["version"], "conductor.host.instances/1",
        "{registry:#}"
    );
    let files = registry["files"].as_array().expect("files");
    assert_eq!(files.len(), 1, "{registry:#}");
    assert_eq!(files[0]["config"], text_of(&file), "{registry:#}");
    let home = case.join("home");
    let alpha = &files[0]["instances"][0];
    assert_eq!(alpha["name"], "alpha");
    assert_eq!(alpha["session_prefix"], "a");
    assert_eq!(alpha["dashboard_port"], 7313);
    assert_eq!(
        alpha["state"],
        text_of(&home.join(".b10x/conductor/alpha/state"))
    );
    assert_eq!(
        alpha["records"],
        text_of(&home.join(".b10x/conductor/alpha/records"))
    );

    // Validating again keeps one entry for the file.
    let output = run(&case, &file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(registered(&case), [text_of(&file)]);
}

/// Acceptance 2 and 5: a second config file whose instance holds the prefix (or one nesting with
/// it), the port, the state or the records directory of a registered instance is refused by the
/// key's path, naming both instances and both files, and is not registered.
#[test]
fn a_second_config_file_with_a_colliding_instance_is_refused_naming_both() {
    let case = case_dir("collisions");
    let alpha_state = case.join("alpha-state");
    let alpha_records = case.join("alpha-records");
    let first = config(
        &case,
        "one.yaml",
        &[instance(
            &case,
            "alpha",
            &[
                "session_prefix: a",
                &format!("state: {}", alpha_state.display()),
                &format!("records: {}", alpha_records.display()),
            ],
        )],
    );
    let output = run(&case, &first, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));

    let state = format!("state: {}", alpha_state.display());
    let records = format!("records: {}", alpha_records.display());
    for (what, extra, path) in [
        (
            "the prefix",
            vec!["session_prefix: a", "dashboard_port: 7400"],
            "instances[0].session_prefix",
        ),
        (
            "a nesting prefix",
            vec!["session_prefix: a-x", "dashboard_port: 7400"],
            "instances[0].session_prefix",
        ),
        (
            "the filled port",
            vec!["session_prefix: g"],
            "instances[0].dashboard_port",
        ),
        (
            "the state",
            vec!["session_prefix: g", "dashboard_port: 7400", &state],
            "instances[0].state",
        ),
        (
            "the records",
            vec!["session_prefix: g", "dashboard_port: 7400", &records],
            "instances[0].records",
        ),
    ] {
        let second = config(&case, "two.yaml", &[instance(&case, "gamma", &extra)]);
        let output = run(&case, &second, &["config", "validate"]);
        assert_eq!(output.status.code(), Some(1), "{what}: {}", shown(&output));
        let said = stderr(&output);
        assert!(
            said.contains(&format!("{}: {path}: ", second.display()))
                && said.contains("instance gamma")
                && said.contains("instance alpha")
                && said.contains(&text_of(&first)),
            "{what}: {said}"
        );
        assert_eq!(registered(&case), [text_of(&first)], "{what}");
    }

    // A removed file holds nothing: once one.yaml is gone, gamma takes alpha's prefix.
    fs::remove_file(&first).expect("remove one.yaml");
    let second = config(
        &case,
        "two.yaml",
        &[instance(&case, "gamma", &["session_prefix: a"])],
    );
    let output = run(&case, &second, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(registered(&case), [text_of(&second)]);
}

/// I5: two instances of one file never share a state or a records directory.
#[test]
fn two_instances_of_one_file_never_share_a_state_or_records_directory() {
    let case = case_dir("one-file-dirs");
    for key in ["state", "records"] {
        let shared = format!("{key}: {}", case.join("shared").display());
        let file = config(
            &case,
            "conductor.yaml",
            &[
                instance(&case, "alpha", &["session_prefix: a", &shared]),
                instance(&case, "beta", &["session_prefix: b", &shared]),
            ],
        );
        let output = run(&case, &file, &["config", "validate"]);
        assert_eq!(output.status.code(), Some(1), "{key}: {}", shown(&output));
        assert!(
            stderr(&output).contains(&format!("instances[1].{key}: "))
                && stderr(&output).contains("instances[0]"),
            "{key}: {}",
            shown(&output)
        );
    }
}

/// A port no process holds now.
fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("a free port")
}

/// A dashboard server, killed and reaped when dropped.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Starts `conductor dashboard serve` for the config file `config` without `--port`, so it
/// serves its instance's `dashboard_port`, and answers it with its first line.
fn serve(case: &Path, config: &Path) -> (Server, String) {
    let child = common::conductor(case.join("home"))
        .args(["dashboard", "serve"])
        .env(CONFIG_VARIABLE, config)
        .current_dir(case)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start conductor dashboard serve");
    let mut server = Server(child);
    let stdout = server
        .0
        .stdout
        .take()
        .expect("the server's standard output");
    let (lines, line) = mpsc::channel();
    std::thread::spawn(move || {
        let mut first = String::new();
        let _ = BufReader::new(stdout).read_line(&mut first);
        let _ = lines.send(first);
    });
    let first = line
        .recv_timeout(Duration::from_secs(30))
        .expect("the server prints a first line within 30 s");
    (server, first)
}

/// Acceptance 5: two distinct config files both validate, both are registered, and both start
/// their dashboards, each on its own port.
#[test]
fn two_distinct_config_files_both_validate_and_both_start_their_dashboards() {
    let case = case_dir("two-files");
    let (port_a, port_b) = (free_port(), free_port());
    let one = config(
        &case,
        "one.yaml",
        &[instance(
            &case,
            "alpha",
            &["session_prefix: a", &format!("dashboard_port: {port_a}")],
        )],
    );
    let two = config(
        &case,
        "two.yaml",
        &[instance(
            &case,
            "beta",
            &["session_prefix: b", &format!("dashboard_port: {port_b}")],
        )],
    );
    for file in [&one, &two] {
        let output = run(&case, file, &["config", "validate"]);
        assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    }
    let mut files = registered(&case);
    files.sort();
    assert_eq!(files, [text_of(&one), text_of(&two)]);

    let (_alpha, said_a) = serve(&case, &one);
    let (_beta, said_b) = serve(&case, &two);
    assert_eq!(
        said_a.trim_end(),
        format!("listening on http://127.0.0.1:{port_a}")
    );
    assert_eq!(
        said_b.trim_end(),
        format!("listening on http://127.0.0.1:{port_b}")
    );
}

// ---------------------------------------------------------------------------------------------
// The guard (acceptance 4)
// ---------------------------------------------------------------------------------------------

/// The recorded SendMessage payload from a session working in `cwd`, to `to`.
fn send_message(cwd: &Path, to: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/guard/send-message.json");
    let mut payload: Value =
        serde_json::from_str(&fs::read_to_string(path).expect("the recorded payload"))
            .expect("the recorded payload is JSON");
    payload["cwd"] = Value::String(text_of(cwd));
    let input = payload["tool_input"]
        .as_object_mut()
        .expect("the payload's tool_input");
    input.insert("to".to_owned(), Value::String(to.to_owned()));
    input.insert("recipient".to_owned(), Value::String(to.to_owned()));
    serde_json::to_vec(&payload).expect("serialise the payload")
}

/// Runs the guard hook for the config file `config` with the payload `stdin`, recording into
/// `state`.
fn hook(case: &Path, config: &Path, state: &Path, stdin: &[u8]) -> Output {
    let mut child = common::conductor(case.join("home"))
        .arg("--state-dir")
        .arg(state)
        .args(["guard", "record-guard-decision", "--from-pre-tool-use"])
        .env(CONFIG_VARIABLE, config)
        .current_dir(case)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the conductor binary runs");
    child
        .stdin
        .take()
        .expect("the hook's standard input")
        .write_all(stdin)
        .expect("write the payload");
    child.wait_with_output().expect("the hook ends")
}

/// The guard's recorded denials in `state`.
fn denials(case: &Path, state: &Path) -> Vec<Map<String, Value>> {
    let output = common::conductor(case.join("home"))
        .arg("--state-dir")
        .arg(state)
        .args(["guard", "guard-decisions", "--format", "json"])
        .current_dir(case)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert!(output.status.success(), "{}", shown(&output));
    match serde_json::from_slice(&output.stdout).expect("the view's output is JSON") {
        Value::Array(rows) => rows
            .into_iter()
            .filter_map(|row| row.as_object().cloned())
            .collect(),
        other => panic!("the view's output is not an array: {other}"),
    }
}

/// Acceptance 4: conductor's message to a session of another registered config file's instance
/// is refused and recorded, naming that instance, whether or not its own instance has a prefix;
/// a session no registered instance claims stays its own.
#[test]
fn a_message_to_another_registered_instance_s_session_is_refused_and_recorded() {
    let case = case_dir("guard-message");
    let beta = config(
        &case,
        "beta.yaml",
        &[instance(
            &case,
            "beta",
            &["session_prefix: b", "dashboard_port: 7400"],
        )],
    );
    let output = run(&case, &beta, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));

    let records = case.join("alpha-records");
    fs::create_dir_all(&records).expect("create alpha's records");
    let records_line = format!("records: {}", records.display());
    for (how, extra) in [
        ("without a prefix", vec![records_line.as_str()]),
        (
            "with a prefix",
            vec!["session_prefix: a", records_line.as_str()],
        ),
    ] {
        let alpha = config(&case, "alpha.yaml", &[instance(&case, "alpha", &extra)]);
        let state = case.join(format!("state-{}", how.replace(' ', "-")));
        let output = hook(
            &case,
            &alpha,
            &state,
            &send_message(&records, "b-conductor"),
        );
        assert_eq!(output.status.code(), Some(2), "{how}: {}", shown(&output));
        assert!(
            stderr(&output).contains("instance beta"),
            "{how}: the denial names the other instance: {}",
            shown(&output)
        );
        let rows = denials(&case, &state);
        assert_eq!(rows.len(), 1, "{how}: one denial recorded: {rows:#?}");
        assert_eq!(rows[0]["tool"], "SendMessage", "{how}: {rows:#?}");

        if how == "without a prefix" {
            let output = hook(&case, &alpha, &state, &send_message(&records, "nobody"));
            assert_eq!(output.status.code(), Some(0), "{how}: {}", shown(&output));
        }
    }
}
