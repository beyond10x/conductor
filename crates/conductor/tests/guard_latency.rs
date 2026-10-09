//! `story:guard-hook`: the hook's wall time with 10,000 verdicts already recorded. Every tool call
//! of every controller runs the hook, and the store it records into verifies its whole history
//! when it opens, so the time of a recorded verdict grows with the record. That is why only
//! denials are recorded (correction 1): an allowance is answered without opening the store, and
//! its time here should not depend on the record at all.
//!
//! A measurement, not a check, and ignored by default. Run it in a release build:
//!
//! ```console
//! cargo test --release -p conductor-cli --test guard_latency -- --ignored --nocapture
//! ```
//!
//! It seeds the verdicts through one store handle, through the same generated
//! `RecordGuardDecision` behaviour the hook runs, keeps the seeded store under this test target's
//! temporary directory so a second run measures without seeding again, then runs the built binary
//! 50 times on an allowed and 50 times on a denied recorded payload and prints p50 and p95 of each
//! and how many of the verdicts were not recorded within the hook's deadline. Last it times
//! recording alone, with no deadline: opening the store and recording one verdict, as the hook's
//! recording thread does, 10 times in this process. It asserts only that every run answered as the
//! rule table says.

mod common;

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use conductor_cli::store::{self, Store};
use conductor_model::behaviour::{Generated, GuardDecisionStorage, TryContext, unmet_context};
use conductor_model::direction::ServingId;
use conductor_model::dispatch::obligations::{GuardDecisionsQuery, RecordGuardDecisionBehavior};
use conductor_model::dispatch::{
    GuardDecisionId, GuardDecisionSnapshot, MessageId, RecordGuardDecision, ResourceRequestId,
    Verdict,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{BoardId, ObservationId, RepositoryName, SnapshotId};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::Value;

/// How many verdicts are recorded before the hook is timed.
const SEEDED: usize = 10_000;

/// How many times each payload is timed.
const RUNS: usize = 50;

/// The store as the generated behaviour's ports, assigning each verdict a fresh id.
struct Ports(Store);

impl GuardDecisionStorage for Ports {
    fn get(&self, identity: &GuardDecisionId) -> Option<GuardDecisionSnapshot> {
        GuardDecisionStorage::get(&self.0, identity)
    }

    fn put(&mut self, snapshot: GuardDecisionSnapshot) {
        GuardDecisionStorage::put(&mut self.0, snapshot);
    }

    fn delete(&mut self, identity: &GuardDecisionId) {
        GuardDecisionStorage::delete(&mut self.0, identity);
    }

    fn list(&self) -> Vec<GuardDecisionSnapshot> {
        GuardDecisionStorage::list(&self.0)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<ServingId, UnmetObligation> {
        Err(unmet_context("not assigned here"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<GuardDecisionId, UnmetObligation> {
        Ok(GuardDecisionId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<MessageId, UnmetObligation> {
        Err(unmet_context("not assigned here"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<ResourceRequestId, UnmetObligation> {
        Err(unmet_context("not assigned here"))
    }

    fn try_generate_conductor_observation_board_id(&mut self) -> Result<BoardId, UnmetObligation> {
        Err(unmet_context("not assigned here"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<ObservationId, UnmetObligation> {
        Err(unmet_context("not assigned here"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<SnapshotId, UnmetObligation> {
        Err(unmet_context("not assigned here"))
    }
}

/// The recorded payload `name`, every `~/` value under `home`.
fn payload(name: &str, home: &Path) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/guard")
        .join(format!("{name}.json"));
    let text = fs::read_to_string(path).expect("read the recorded payload");
    let localised = text.replace("\"~/", &format!("\"{}/", home.display()));
    let value: Value = serde_json::from_str(&localised).expect("the payload is JSON");
    serde_json::to_vec(&value).expect("serialise the payload")
}

/// The `n`th verdict seeded, shaped like the hook's.
fn verdict(n: usize) -> RecordGuardDecision {
    RecordGuardDecision {
        repository: RepositoryName(".guard-probe-w04".to_owned()),
        session_ref: "00000000-0000-4000-8000-0000000000a1".to_owned(),
        tool: if n.is_multiple_of(2) {
            "Write"
        } else {
            "SendMessage"
        }
        .to_owned(),
        target: format!("~/example-org/.guard-probe-w04/seed-{n}.txt"),
        verdict: if n.is_multiple_of(2) {
            Verdict::Allow
        } else {
            Verdict::Deny
        },
        reason: "Write target is inside ~/example-org/.guard-probe-w04/ or its managed worktrees"
            .to_owned(),
        decided_at: Timestamp("2026-10-07T00:00:00Z".to_owned()),
    }
}

/// Records `count` verdicts into the store under `state`, through one handle.
fn seed(state: &Path, count: usize) {
    let mut generated = Generated::new(Ports(store::open(state).expect("open the store")));
    let started = Instant::now();
    for n in 0..count {
        generated
            .record_guard_decision(verdict(n))
            .expect("RecordGuardDecision decides an outcome");
        generated
            .ports
            .0
            .check()
            .expect("the store kept the verdict");
        if (n + 1) % 1000 == 0 {
            eprintln!("seeded {} in {:?}", n + 1, started.elapsed());
        }
    }
}

/// How many verdicts the store under `state` holds, or 0 where there is none.
fn held(state: &Path) -> usize {
    if !state.join("tree").exists() {
        return 0;
    }
    let generated = Generated::new(store::open_existing(state).expect("open the store"));
    let rows = generated
        .guard_decisions()
        .expect("the generated query answers");
    generated.ports.check().expect("the store reads");
    rows.len()
}

/// Wall times of `RUNS` hook runs on `stdin`, sorted, each asserted to exit `expected`, and how
/// many of the runs said their verdict was not recorded.
fn time(home: &Path, state: &Path, stdin: &[u8], expected: i32) -> (Vec<Duration>, usize) {
    let mut unrecorded = 0;
    let mut times: Vec<Duration> = (0..RUNS)
        .map(|_| {
            let started = Instant::now();
            let mut child = common::conductor(home)
                .arg("--state-dir")
                .arg(state)
                .args(["guard", "record-guard-decision", "--from-pre-tool-use"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .expect("the conductor binary runs");
            child
                .stdin
                .take()
                .expect("the hook's standard input")
                .write_all(stdin)
                .expect("write the payload");
            let output = child.wait_with_output().expect("the hook ends");
            let took = started.elapsed();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(expected), "stderr: {stderr}");
            if stderr.contains("the verdict was not recorded") {
                unrecorded += 1;
            }
            took
        })
        .collect();
    times.sort();
    (times, unrecorded)
}

/// Wall times of `runs` recordings with no deadline, sorted: each opens the store under `state`
/// and records one verdict, as the hook's recording thread does.
fn record_alone(state: &Path, runs: usize) -> Vec<Duration> {
    let mut times: Vec<Duration> = (0..runs)
        .map(|n| {
            let started = Instant::now();
            let mut generated = Generated::new(Ports(store::open(state).expect("open the store")));
            generated
                .record_guard_decision(verdict(n))
                .expect("RecordGuardDecision decides an outcome");
            generated
                .ports
                .0
                .check()
                .expect("the store kept the verdict");
            started.elapsed()
        })
        .collect();
    times.sort();
    times
}

/// The nearest-rank `percentile` of sorted `times`.
fn percentile(times: &[Duration], percentile: usize) -> Duration {
    let rank = (percentile * times.len()).div_ceil(100).max(1);
    times[rank - 1]
}

#[test]
#[ignore = "a measurement: seeds 10,000 verdicts; run with --release -- --ignored --nocapture"]
fn the_hook_with_ten_thousand_verdicts_recorded() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("guard_latency");
    let home = dir.join("home");
    let state = dir.join("state");
    fs::create_dir_all(&home).expect("create the home");
    let before = held(&state);
    if before < SEEDED {
        seed(&state, SEEDED - before);
    }
    let seeded = held(&state);
    assert!(seeded >= SEEDED, "{seeded} verdicts recorded");

    let allowed = time(&home, &state, &payload("write", &home), 0);
    let denied = time(&home, &state, &payload("send-message", &home), 2);
    for (name, (times, unrecorded)) in
        [("allowed Write", &allowed), ("denied SendMessage", &denied)]
    {
        println!(
            "hook, {name}: {RUNS} runs with {seeded}+ verdicts recorded: p50 {:?}, p95 {:?}, \
             min {:?}, max {:?}; not recorded within the deadline: {unrecorded} of {RUNS}",
            percentile(times, 50),
            percentile(times, 95),
            times[0],
            times[times.len() - 1]
        );
    }
    let alone = record_alone(&state, 10);
    println!(
        "recording alone, no deadline: 10 runs with {}+ verdicts recorded: p50 {:?}, max {:?}",
        held(&state) - 10,
        percentile(&alone, 50),
        alone[alone.len() - 1]
    );
}
