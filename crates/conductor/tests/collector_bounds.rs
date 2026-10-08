//! `story:collector-bounds`: a failed snapshot says why, and no collector waits on a command for
//! ever. Also the columns of every view `snapshot` shows, against the specification.
//!
//! Each case keeps its store under `state/` in its own directory in this test target's temporary
//! directory, takes snapshots through the library ([`snapshot::take`]) and reads them back through
//! the built binary, a later process, with `--state-dir` naming that `state/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use anyhow::bail;
use conductor_cli::collect::{self, Collector};
use conductor_cli::snapshot::{self, Taken};
use conductor_cli::store;
use conductor_model::observation::SnapshotState;
use serde_json::Value;

/// A fresh directory for one case, with an empty `work/` to run the binary from.
fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("collector_bounds")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(dir.join("work")).expect("create the case's directories");
    dir
}

/// Takes one snapshot into `dir/state` through `collectors`.
fn take(dir: &Path, collectors: &[Collector]) -> Taken {
    let store = store::open(&dir.join("state")).expect("open the store");
    snapshot::take(store, collectors).expect("the driver ends the snapshot")
}

/// Runs the binary from `dir/work` with `--state-dir` naming `dir/state`.
fn conductor(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_conductor"))
        .current_dir(dir.join("work"))
        .arg("--state-dir")
        .arg(dir.join("state"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stdout(dir: &Path, args: &[&str]) -> String {
    let output = conductor(dir, args);
    assert!(output.status.success(), "{args:?}: {}", describe(&output));
    String::from_utf8(output.stdout).expect("UTF-8")
}

/// The snapshots `snapshot snapshots --format jsonl` lists.
fn snapshots(dir: &Path) -> Vec<Value> {
    stdout(dir, &["snapshot", "snapshots", "--format", "jsonl"])
        .lines()
        .map(|line| serde_json::from_str(line).expect("one JSON object per line"))
        .collect()
}

/// A collector whose source answers an error.
fn broken() -> Collector {
    Collector::new("broken", |_| bail!("the source did not answer"))
}

/// A collector that records nothing.
fn quiet() -> Collector {
    Collector::new("quiet", |_| Ok(()))
}

/// Acceptance: a collector that returns an error leaves a failed snapshot whose reason names it,
/// and `snapshot snapshots --format jsonl` shows that reason in a later process.
#[test]
fn a_collector_that_answers_an_error_is_named_by_the_failed_snapshot_in_a_later_process() {
    let dir = case_dir("error");
    let taken = take(&dir, &[quiet(), broken(), quiet()]);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    assert_eq!(
        taken.reason.as_deref(),
        Some("broken: the source did not answer")
    );

    let listed = snapshots(&dir);
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0]["snapshot_id"], taken.snapshot_id.0.0.as_str());
    assert_eq!(listed[0]["state"], "Failed");
    assert_eq!(
        listed[0]["failure_reason"], "broken: the source did not answer",
        "{listed:?}"
    );
}

/// Acceptance (the bound is per command): a collector whose command sleeps past its bound of 2 s
/// lets `snapshot::take` return within the bound plus 5 s, and the snapshot is `Failed` with a
/// reason naming the collector, the program and the bound, read in a later process.
#[test]
fn a_command_past_its_bound_fails_the_snapshot_naming_collector_program_and_bound() {
    let dir = case_dir("bound");
    let slow = Collector::new("slow", |_| {
        collect::run(Command::new("sleep").arg("30"), Duration::from_secs(2))?;
        Ok(())
    });
    let started = Instant::now();
    let taken = take(&dir, &[slow]);
    let took = started.elapsed();
    assert!(took < Duration::from_secs(7), "the snapshot took {took:?}");
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = "slow: `sleep` gave no answer within 2 s";
    assert_eq!(taken.reason.as_deref(), Some(reason));

    let listed = snapshots(&dir);
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0]["state"], "Failed");
    assert_eq!(listed[0]["failure_reason"], reason, "{listed:?}");
}

/// `snapshot snapshots` shows `failure_reason` in every format: the reason of a failed snapshot,
/// nothing for a complete one.
#[test]
fn snapshot_snapshots_shows_the_failure_reason_in_every_format() {
    let dir = case_dir("formats");
    let complete = take(&dir, &[quiet()]);
    assert_eq!(complete.state, SnapshotState::Complete, "{complete:?}");
    let failed = take(&dir, &[broken()]);
    assert_eq!(failed.state, SnapshotState::Failed, "{failed:?}");
    let reason = "broken: the source did not answer";

    let listed = snapshots(&dir);
    assert_eq!(listed.len(), 2, "{listed:?}");
    assert_eq!(listed[0]["failure_reason"], Value::Null, "{listed:?}");
    assert_eq!(listed[1]["failure_reason"], reason, "{listed:?}");

    let json: Value = serde_json::from_str(&stdout(
        &dir,
        &["snapshot", "snapshots", "--format", "json"],
    ))
    .expect("one JSON array");
    assert_eq!(json[0]["failure_reason"], Value::Null, "{json}");
    assert_eq!(json[1]["failure_reason"], reason, "{json}");

    for format in ["text", "markdown"] {
        let shown = stdout(&dir, &["snapshot", "snapshots", "--format", format]);
        let mut lines = shown.lines();
        let header = lines.next().expect("a header line");
        assert!(header.contains("failure_reason"), "{format}: {shown}");
        let failed_line = shown
            .lines()
            .find(|line| line.contains(failed.snapshot_id.0.0.as_str()))
            .expect("the failed snapshot's line");
        assert!(failed_line.contains(reason), "{format}: {shown}");
    }
}

/// The field names of each view of the specification, by its wire name, in the order
/// `ess specify compile` lists them.
fn spec_view_fields() -> Vec<(String, Vec<String>)> {
    let spec = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec");
    let output = Command::new("ess")
        .args(["specify", "compile", "--path"])
        .arg(&spec)
        .args(["--format", "json"])
        .output()
        .expect("`ess` is on PATH");
    assert!(
        output.status.success(),
        "`ess specify compile` exited {}",
        output.status
    );
    let model: Value = serde_json::from_slice(&output.stdout).expect("compile prints JSON");
    model["views"]
        .as_object()
        .expect("the model has views")
        .values()
        .map(|view| {
            let wire = view["naming"]["wire"]
                .as_str()
                .expect("a view has a wire name")
                .to_owned();
            let fields = view["fields"]
                .as_array()
                .expect("a view has fields")
                .iter()
                .map(|field| {
                    field["name"]
                        .as_str()
                        .expect("a field has a name")
                        .to_owned()
                })
                .collect();
            (wire, fields)
        })
        .collect()
}

/// Every view `src/snapshot.rs` shows carries the specification's fields for it, named and ordered
/// as declared: a field the specification adds to one of them (as `failure_reason` and the two
/// repository dates were) is a column the view shows, or this is red.
#[test]
fn every_snapshot_view_shows_the_specifications_fields_in_order() {
    const SHOWN: [&str; 6] = [
        "snapshots",
        "repositories",
        "pull-requests",
        "workflow-runs",
        "sessions",
        "blockers",
    ];
    let spec = spec_view_fields();
    let dir = case_dir("columns");
    take(&dir, &[quiet()]);
    let mut wrong = Vec::new();
    for view in SHOWN {
        let fields = &spec
            .iter()
            .find(|(wire, _)| wire == view)
            .unwrap_or_else(|| panic!("the specification declares no view `{view}`"))
            .1;
        let shown = stdout(&dir, &["snapshot", view, "--format", "text"]);
        let header: Vec<&str> = shown
            .lines()
            .next()
            .expect("a header line")
            .split_whitespace()
            .collect();
        if header != *fields {
            wrong.push(format!(
                "`snapshot {view}` shows {header:?}, the specification {fields:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
