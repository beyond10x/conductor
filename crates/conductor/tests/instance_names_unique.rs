//! `story:instance-names-unique`: no two instances on a host share a session name, a unit, a
//! dashboard port or a state or records directory, whether they are in one config file or in
//! several (`docs/design/roles-and-surfaces.md` § 5, I1 to I5).
//!
//! - `config validate` refuses an instance without a `session_prefix` and names the prefix
//!   `config migrate` writes; `config migrate` writes it and keeps every other line.
//! - Two instances of one file never share a state or records directory.
//! - Two distinct config files both validate, and both start their runtimes.
//!
//! The host registry, `~/.b10x/conductor/host/instances.json`, that compared the instances of
//! several files, and the guard's reading of it, are gone in v1 (design architecture-next § 11,
//! `story:spec-v1`): one daemon reads one file. Nothing writes it any more.
//!
//! Every case is a directory of its own under the target's temporary directory, with `home/` as
//! `HOME`; nothing reads the real home.

mod common;

use std::fs;
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// The variable naming the config file, as `common::conductor` removes it.
const CONFIG_VARIABLE: &str = "CONDUCTOR_CONFIG";
use serde_json::Value;

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
fn config_migrate_writes_the_suggested_prefix() {
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
    // The conversion to conductor.config/2 prints its own lines first; the prefix line is last.
    assert!(
        String::from_utf8_lossy(&output.stdout).ends_with("\nb: session_prefix b2\n"),
        "{}",
        shown(&output)
    );
    assert_eq!(fs::read_to_string(&file).expect("the file"), before);

    let output = run(&case, &file, &["config", "migrate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let after = fs::read_to_string(&file).expect("the file");
    // The conversion rewrites the file whole; the prefix is written and the old one is gone.
    assert!(after.contains("session_prefix: b2"), "{after}");
    assert!(!after.contains("session_prefix: null"), "{after}");
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
        written.contains("- name: gamma\n  session_prefix: gamma\n"),
        "{written}"
    );
    let output = run(&case, &file, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
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

/// A runtime, killed and reaped when dropped.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Starts `conductor instance serve` for the config file `config`, and answers it with its first
/// line, which names the socket it serves (`story:instance-api`).
fn serve(case: &Path, config: &Path) -> (Server, String) {
    let child = common::conductor(case.join("home"))
        .args(["instance", "serve"])
        .env(CONFIG_VARIABLE, config)
        .current_dir(case)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start conductor instance serve");
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

/// Acceptance 5: two distinct config files both validate and both start their instance's
/// runtime, each on its own socket (since `story:instance-api`, which retired the per-instance
/// dashboard port).
#[test]
fn two_distinct_config_files_both_validate_and_both_start_their_runtimes() {
    let case = case_dir("two-files");
    let one = config(
        &case,
        "one.yaml",
        &[instance(&case, "alpha", &["session_prefix: a"])],
    );
    let two = config(
        &case,
        "two.yaml",
        &[instance(&case, "beta", &["session_prefix: b"])],
    );
    for file in [&one, &two] {
        let output = run(&case, file, &["config", "validate"]);
        assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    }
    let (_alpha, said_a) = serve(&case, &one);
    let (_beta, said_b) = serve(&case, &two);
    let socket = |said: &str, name: &str| {
        said.trim_end()
            .strip_prefix(&format!("conductor instance {name}: serving /v1/ on "))
            .unwrap_or_else(|| panic!("{name}'s runtime said {said:?}"))
            .to_owned()
    };
    assert_ne!(socket(&said_a, "alpha"), socket(&said_b, "beta"));
}
