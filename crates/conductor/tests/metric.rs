//! `story:kpi-records`: `conductor metric compute --day <date>` defines the metrics of
//! `docs/design/self-improvement.md` § 2 it computes when they are absent, and records one
//! reading per metric, scope and day through `RecordMetricReading`; and the metric commands and
//! views answer through their own handlers.
//!
//! Every case builds its own fixture in this test target's temporary directory: a config file of
//! one instance (`example-org`) naming the case's `records/` and `state/`, the records' two logs
//! written as the commands write them, and one complete snapshot taken through the library with
//! the forge's releases and merged pull requests. The binary runs with the case's empty `home/`
//! as `HOME`. Each metric has its own case, with the value counted by hand over the fixture in
//! the case's comment.
//!
//! The day is 2026-10-14. The fixture, in UTC:
//!
//! - **decisions/**: DEC-1 decided by the operator at 08:00; DEC-2 decided by conductor at 09:00;
//!   REQ-1 escalated as O at 10:00 and answered at 14:00; REQ-2 escalated as H at 11:00 and
//!   withdrawn at 15:00; REQ-3 escalated as O the day before and withdrawn at 13:00; REQ-5
//!   escalated as H the day before and withdrawn at 16:00; REQ-6 never escalated and withdrawn at
//!   17:00; REQ-7 escalated as O two days before and answered at 18:00; DEC-9 decided by the
//!   operator the day after.
//! - **dispatches/**: alpha's D-1 sent 08:00, done 09:00 (1 h); alpha's D-2 sent the evening
//!   before at 20:00, done 10:00 (14 h); beta's D-3 sent 06:00, done 08:30 (2.5 h); beta's D-4
//!   sent 07:00, done the day after; beta's D-5 sent 05:00 and failed; alpha's D-6 sent 00:00,
//!   done 03:00 (3 h).
//! - **the snapshot** (started 2026-10-15T00:30Z, complete): alpha's releases v0.9 (10-13 12:00)
//!   and v1.0 (10-14 12:00), beta's v2.0 (10-14 20:00); merged pull requests alpha#1 (10-13
//!   10:00, released by v0.9), alpha#2 (06:00, v1.0 after 6 h), alpha#3 (11:00, v1.0 after 1 h),
//!   alpha#4 (13:00, not released), beta#5 (10:00, v2.0 after 10 h), beta#6 (10-13 20:00, v2.0
//!   after 24 h). An older complete snapshot holds one more merged pull request on the day,
//!   which no metric counts: the newest complete snapshot is read.

mod active;
mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use conductor_cli::collect::Collector;
use conductor_cli::snapshot::{self, Recorder};
use conductor_cli::store;
use conductor_model::observation::{
    RecordMergedPullRequest, RecordRelease, RepositoryName, SnapshotState, StartSnapshot,
};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};

/// The day every case computes.
const DAY: &str = "2026-10-14";

/// The decision log of the fixture, as the commands write its lines.
const DECISIONS: &str = r#"{"at":"2026-10-12T09:00:00Z","event":"escalated","repo":"alpha","id":"REQ-7","escalation_class":"O","reason":"needs the operator"}
{"at":"2026-10-13T11:00:00Z","event":"escalated","repo":"alpha","id":"REQ-3","escalation_class":"O","reason":"needs the operator"}
{"at":"2026-10-13T12:00:00Z","event":"escalated","repo":"beta","id":"REQ-5","escalation_class":"H","reason":"only a person runs it"}
{"at":"2026-10-14T08:00:00Z","event":"decided","repo":"alpha","id":"DEC-1","class":"O","decided_by":"Operator","question":"q","options":"a|b","choice":"a","reason":"r","evidence":"e","recorded_by":"conductor","operator_words":"a"}
{"at":"2026-10-14T09:00:00Z","event":"decided","repo":"alpha","id":"DEC-2","class":"C","decided_by":"Conductor","question":"q","options":"a|b","choice":"b","reason":"r","evidence":"e"}
{"at":"2026-10-14T10:00:00Z","event":"escalated","repo":"alpha","id":"REQ-1","escalation_class":"O","reason":"needs the operator"}
{"at":"2026-10-14T11:00:00Z","event":"escalated","repo":"beta","id":"REQ-2","escalation_class":"H","reason":"only a person runs it"}
{"at":"2026-10-14T13:00:00Z","event":"withdrawn","repo":"alpha","id":"REQ-3","reason":"moot"}
{"at":"2026-10-14T14:00:00Z","event":"answered","repo":"alpha","id":"REQ-1","decision_id":"DEC-1"}
{"at":"2026-10-14T15:00:00Z","event":"withdrawn","repo":"beta","id":"REQ-2","reason":"resolved without the operator"}
{"at":"2026-10-14T16:00:00Z","event":"withdrawn","repo":"beta","id":"REQ-5","reason":"moot"}
{"at":"2026-10-14T17:00:00Z","event":"withdrawn","repo":"beta","id":"REQ-6","reason":"moot"}
{"at":"2026-10-14T18:00:00Z","event":"answered","repo":"alpha","id":"REQ-7","decision_id":"DEC-1"}
{"at":"2026-10-15T08:00:00Z","event":"decided","repo":"alpha","id":"DEC-9","class":"O","decided_by":"Operator","question":"q","options":"a|b","choice":"a","reason":"r","evidence":"e","recorded_by":"conductor","operator_words":"a"}
"#;

/// The dispatch log of the fixture, as the commands write its lines.
const DISPATCHES: &str = r#"{"at":"2026-10-13T20:00:00Z","event":"sent","repo":"alpha","id":"D-2","to":"alpha","brief":"b"}
{"at":"2026-10-14T00:00:00Z","event":"sent","repo":"alpha","id":"D-6","to":"alpha","brief":"b"}
{"at":"2026-10-14T03:00:00Z","event":"done","repo":"alpha","id":"D-6","evidence":"alpha#9"}
{"at":"2026-10-14T05:00:00Z","event":"sent","repo":"beta","id":"D-5","to":"beta","brief":"b"}
{"at":"2026-10-14T06:00:00Z","event":"sent","repo":"beta","id":"D-3","to":"beta","brief":"b"}
{"at":"2026-10-14T06:30:00Z","event":"failed","repo":"beta","id":"D-5","reason":"r"}
{"at":"2026-10-14T07:00:00Z","event":"sent","repo":"beta","id":"D-4","to":"beta","brief":"b"}
{"at":"2026-10-14T08:00:00Z","event":"sent","repo":"alpha","id":"D-1","to":"alpha","brief":"b"}
{"at":"2026-10-14T08:30:00Z","event":"done","repo":"beta","id":"D-3","evidence":"beta#5"}
{"at":"2026-10-14T09:00:00Z","event":"done","repo":"alpha","id":"D-1","evidence":"alpha#2"}
{"at":"2026-10-14T10:00:00Z","event":"done","repo":"alpha","id":"D-2","evidence":"alpha#3"}
{"at":"2026-10-15T07:00:00Z","event":"done","repo":"beta","id":"D-4","evidence":"beta#7"}
"#;

/// The releases of the newest snapshot: repository, tag, published at.
const RELEASES: [(&str, &str, &str); 3] = [
    ("alpha", "v0.9", "2026-10-13T12:00:00Z"),
    ("alpha", "v1.0", "2026-10-14T12:00:00Z"),
    ("beta", "v2.0", "2026-10-14T20:00:00Z"),
];

/// The merged pull requests of the newest snapshot: repository, number, merged at.
const MERGED: [(&str, i64, &str); 6] = [
    ("alpha", 1, "2026-10-13T10:00:00Z"),
    ("alpha", 2, "2026-10-14T06:00:00Z"),
    ("alpha", 3, "2026-10-14T11:00:00Z"),
    ("alpha", 4, "2026-10-14T13:00:00Z"),
    ("beta", 5, "2026-10-14T10:00:00Z"),
    ("beta", 6, "2026-10-13T20:00:00Z"),
];

/// One case's directory: `home/`, `records/` (the working directory of every run), `state/` and
/// the config file naming them.
struct Case {
    dir: PathBuf,
}

impl Case {
    /// A fresh case holding nothing but its config file and empty directories.
    fn empty(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("metric")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "records", "state"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        fs::write(
            case.config(),
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   session_prefix: ex\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n",
                root = case.dir.join("home/example-org").display(),
                trees = case.dir.join("trees").display(),
                records = case.records().display(),
                state = case.state().display(),
            ),
        )
        .expect("write the config file");
        case
    }

    /// A case holding the whole fixture the module documentation describes.
    fn fixture(name: &str) -> Self {
        let case = Self::empty(name);
        for (log, text) in [("decisions", DECISIONS), ("dispatches", DISPATCHES)] {
            let dir = case.records().join(log);
            fs::create_dir_all(&dir).expect("create a log's directory");
            fs::write(dir.join("2026-10.jsonl"), text).expect("write a log");
        }
        // An older complete snapshot, whose pull request merged on the day no metric counts.
        case.snapshot(
            "2026-10-14T23:00:00Z",
            &[],
            &[("gamma", 77, "2026-10-14T09:00:00Z")],
        );
        case.snapshot("2026-10-15T00:30:00Z", &RELEASES, &MERGED);
        case
    }

    /// Takes one complete snapshot started at `started_at` holding `releases` and `merged`.
    fn snapshot(
        &self,
        started_at: &str,
        releases: &[(&'static str, &'static str, &'static str)],
        merged: &[(&'static str, i64, &'static str)],
    ) {
        let releases = releases.to_vec();
        let merged = merged.to_vec();
        let forge = Collector::new("forge", move |record: &mut Recorder<'_>| {
            let snapshot = record.snapshot().clone();
            for (repository, tag, at) in &releases {
                record.release(RecordRelease {
                    snapshot_id: snapshot.clone(),
                    repository: RepositoryName((*repository).to_owned()),
                    tag: (*tag).to_owned(),
                    published_at: Timestamp((*at).to_owned()),
                    url: format!("https://github.com/example-org/{repository}/releases/tag/{tag}"),
                })?;
            }
            for (repository, number, at) in &merged {
                record.merged_pull_request(RecordMergedPullRequest {
                    snapshot_id: snapshot.clone(),
                    repository: RepositoryName((*repository).to_owned()),
                    number: *number,
                    title: format!("Pull request {number}"),
                    merged_at: Timestamp((*at).to_owned()),
                    url: format!("https://github.com/example-org/{repository}/pull/{number}"),
                })?;
            }
            Ok(())
        });
        let store = store::open(&self.state()).expect("open the store");
        let start = StartSnapshot {
            started_at: Timestamp(started_at.to_owned()),
            disk_free_bytes: 1,
            budget: None,
        };
        let taken = snapshot::take_from(store, start, &[forge]).expect("the snapshot ends");
        assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// `conductor <args>` from the records directory, with the case's home and config file.
    fn run(&self, args: &[&str]) -> Output {
        common::conductor(self.dir.join("home"))
            .args(args)
            .current_dir(self.records())
            .env("CONDUCTOR_CONFIG", self.config())
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// Runs a command that must succeed, and answers its standard output.
    fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "`conductor {}`: {}",
            args.join(" "),
            describe(&output)
        );
        String::from_utf8(output.stdout).expect("stdout is UTF-8")
    }

    /// The rows of a view, read with `--format json`.
    fn rows(&self, group: &str, view: &str) -> Vec<Value> {
        match serde_json::from_str(&self.ok(&[group, view, "--format", "json"])) {
            Ok(Value::Array(rows)) => rows,
            other => panic!("`{group} {view}` printed no JSON array: {other:?}"),
        }
    }

    /// `metric compute --day <DAY>`, which must succeed.
    fn compute(&self) -> String {
        self.ok(&["metric", "compute", "--day", DAY])
    }

    /// The reading of `metric` for `scope` on [`DAY`], or none.
    fn reading(&self, metric: &str, scope: &str) -> Option<Value> {
        let id = format!("{metric}|{scope}|{DAY}");
        self.rows("metric", "metric-readings")
            .into_iter()
            .find(|row| row["reading_id"] == id.as_str())
    }

    /// The value of the reading of `metric` for `scope` on [`DAY`], which must be there.
    fn value(&self, metric: &str, scope: &str) -> f64 {
        let row = self
            .reading(metric, scope)
            .unwrap_or_else(|| panic!("no reading of {metric} for {scope}: {:#?}", self.all()));
        assert_eq!(row["metric_id"], metric, "{row}");
        assert_eq!(row["day"], DAY, "{row}");
        assert_eq!(row["scope"], scope, "{row}");
        assert_eq!(row["state"], "Recorded", "{row}");
        value(&row["value"])
    }

    fn all(&self) -> Vec<Value> {
        self.rows("metric", "metric-readings")
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A decimal as the views print it: a string (or a number), read as a float.
fn value(value: &Value) -> f64 {
    match value {
        Value::String(text) => text
            .parse()
            .unwrap_or_else(|_| panic!("no decimal: {value}")),
        Value::Number(number) => number.as_f64().expect("a number"),
        other => panic!("no decimal: {other}"),
    }
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 0.001,
        "{what}: {actual}, expected {expected}"
    );
}

fn computed(name: &str) -> Case {
    let case = Case::fixture(name);
    case.compute();
    case
}

// ---------------------------------------------------------------------------------------------
// One case per metric, each value counted by hand over the fixture.
// ---------------------------------------------------------------------------------------------

/// K1: the operator's touches on the day (DEC-1 decided by the operator, REQ-1 escalated as O,
/// REQ-2 escalated as H: 3; DEC-2 is conductor's, REQ-3, REQ-5 and REQ-7 were escalated before
/// the day and DEC-9 after it) over the pull requests merged on the day in the newest snapshot
/// (alpha#2, alpha#3, alpha#4, beta#5: 4; gamma#77 is the older snapshot's): 3 / 4 = 0.75.
#[test]
fn k1_is_the_operator_s_touches_over_the_pull_requests_merged_that_day() {
    let case = computed("k1");
    close(case.value("K1", "instance"), 0.75, "K1");
    let row = case.reading("K1", "instance").expect("K1");
    let sources: Vec<&str> = row["sources"]
        .as_array()
        .expect("sources")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for id in [
        "DEC-1", "REQ-1", "REQ-2", "alpha#2", "alpha#3", "alpha#4", "beta#5",
    ] {
        assert!(sources.contains(&id), "K1's sources name {id}: {sources:?}");
    }
    assert!(!sources.contains(&"gamma#77"), "{sources:?}");
}

/// K2.O: of the O items closed on the day (REQ-3 withdrawn; REQ-1 and REQ-7 answered), the share
/// withdrawn: 1 / 3 = 33.333 %.
#[test]
fn k2_o_is_the_share_of_o_items_closed_that_day_that_were_withdrawn() {
    let case = computed("k2-o");
    close(case.value("K2.O", "instance"), 100.0 / 3.0, "K2.O");
}

/// K2.H: of the H items closed on the day (REQ-2 and REQ-5, both withdrawn; REQ-6 was never
/// escalated), the share withdrawn: 2 / 2 = 100 %.
#[test]
fn k2_h_is_the_share_of_h_items_closed_that_day_that_were_withdrawn() {
    let case = computed("k2-h");
    close(case.value("K2.H", "instance"), 100.0, "K2.H");
}

/// K4.median: the dispatches done on the day took 3 h (D-6), 14 h (D-2), 1 h (D-1) and 2.5 h
/// (D-3); D-4 was done the day after and D-5 failed. Sorted 1, 2.5, 3, 14: the median is
/// (2.5 + 3) / 2 = 2.75 h. alpha's 1, 3, 14: 3 h; beta's 2.5: 2.5 h.
#[test]
fn k4_median_is_the_median_hours_from_sent_to_done_of_the_dispatches_done_that_day() {
    let case = computed("k4-median");
    close(case.value("K4.median", "instance"), 2.75, "K4.median");
    close(case.value("K4.median", "alpha"), 3.0, "K4.median of alpha");
    close(case.value("K4.median", "beta"), 2.5, "K4.median of beta");
}

/// K4.p90: the nearest rank, the ⌈0.9 n⌉-th smallest: of 1, 2.5, 3, 14 the 4th, 14 h; alpha's
/// 1, 3, 14 the 3rd, 14 h; beta's 2.5, the 1st.
#[test]
fn k4_p90_is_the_p90_hours_from_sent_to_done_of_the_dispatches_done_that_day() {
    let case = computed("k4-p90");
    close(case.value("K4.p90", "instance"), 14.0, "K4.p90");
    close(case.value("K4.p90", "alpha"), 14.0, "K4.p90 of alpha");
    close(case.value("K4.p90", "beta"), 2.5, "K4.p90 of beta");
}

/// K5.median: the pull requests released on the day waited 6 h (alpha#2), 1 h (alpha#3), 10 h
/// (beta#5) and 24 h (beta#6); alpha#1 was released the day before. Sorted 1, 6, 10, 24: the
/// median is 8 h. alpha's 1, 6: 3.5 h; beta's 10, 24: 17 h.
#[test]
fn k5_median_is_the_median_hours_from_merged_to_released_of_the_pull_requests_released_that_day() {
    let case = computed("k5-median");
    close(case.value("K5.median", "instance"), 8.0, "K5.median");
    close(case.value("K5.median", "alpha"), 3.5, "K5.median of alpha");
    close(case.value("K5.median", "beta"), 17.0, "K5.median of beta");
}

/// K5.p90: of 1, 6, 10, 24 the 4th, 24 h; alpha's 1, 6 the 2nd, 6 h; beta's 10, 24 the 2nd,
/// 24 h.
#[test]
fn k5_p90_is_the_p90_hours_from_merged_to_released_of_the_pull_requests_released_that_day() {
    let case = computed("k5-p90");
    close(case.value("K5.p90", "instance"), 24.0, "K5.p90");
    close(case.value("K5.p90", "alpha"), 6.0, "K5.p90 of alpha");
    close(case.value("K5.p90", "beta"), 24.0, "K5.p90 of beta");
}

/// K5.unreleased: the pull requests merged by the end of the day with no release published
/// after them by then: alpha#4, 1; alpha's 1; beta's 0.
#[test]
fn k5_unreleased_counts_the_pull_requests_merged_and_not_released_by_the_end_of_the_day() {
    let case = computed("k5-unreleased");
    close(
        case.value("K5.unreleased", "instance"),
        1.0,
        "K5.unreleased",
    );
    close(
        case.value("K5.unreleased", "alpha"),
        1.0,
        "K5.unreleased of alpha",
    );
    close(
        case.value("K5.unreleased", "beta"),
        0.0,
        "K5.unreleased of beta",
    );
}

// ---------------------------------------------------------------------------------------------
// The metrics compute defines, and what a second run does.
// ---------------------------------------------------------------------------------------------

/// Compute defines each metric it records when it is absent, without a target, and a second run
/// for the same day records nothing new and still exits 0.
#[test]
fn compute_defines_the_metrics_it_records_and_a_second_run_records_nothing_new() {
    let case = computed("defines");
    let mut ids: Vec<String> = case
        .rows("metric", "metrics")
        .iter()
        .map(|row| {
            assert_eq!(row["state"], "Untargeted", "{row}");
            assert!(row["target"].is_null(), "{row}");
            row["metric_id"].as_str().expect("an id").to_owned()
        })
        .collect();
    ids.sort();
    assert_eq!(
        ids,
        [
            "K1",
            "K2.H",
            "K2.O",
            "K4.median",
            "K4.p90",
            "K5.median",
            "K5.p90",
            "K5.unreleased"
        ]
    );
    let baseline = case
        .rows("metric", "metrics")
        .into_iter()
        .find(|row| row["metric_id"] == "K1")
        .expect("K1");
    close(value(&baseline["baseline"]), 0.245, "K1's baseline");
    assert_eq!(baseline["baseline_period"], "2026-10-06..13", "{baseline}");

    let before = case.all();
    assert!(before.iter().all(|row| row["met"].is_null()), "{before:#?}");
    case.compute();
    assert_eq!(case.all(), before, "a second run records nothing new");
    assert!(
        case.rows("metric", "kpis").is_empty(),
        "no metric has a target, so none is a KPI"
    );
}

/// A day with no decision, dispatch or merge and no snapshot records only the metrics it can
/// measure: K2 and K4 have no item and no dispatch to read, and K1 and K5 no snapshot, so nothing
/// is recorded, and compute still exits 0.
#[test]
fn compute_records_no_reading_it_cannot_measure() {
    let case = Case::empty("nothing");
    case.compute();
    assert_eq!(case.all(), Vec::<Value>::new());
}

/// `--day` must be a date.
#[test]
fn compute_refuses_a_day_that_is_no_date() {
    let case = Case::empty("no-date");
    let output = case.run(&["metric", "compute", "--day", "2026-13-01"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--day"),
        "{}",
        describe(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// KPIs: a metric with a target.
// ---------------------------------------------------------------------------------------------

/// The fields of the view `Kpis`, and nothing else, in the specification's order.
const KPI_FIELDS: [&str; 10] = [
    "metric_id",
    "state",
    "name",
    "unit",
    "instrument",
    "baseline",
    "target",
    "direction",
    "target_owner",
    "target_set_at",
];

/// The fields of the view `KpiReadings`, in the specification's order.
const KPI_READING_FIELDS: [&str; 7] = [
    "metric_id",
    "scope",
    "day",
    "value",
    "target",
    "met",
    "reading_id",
];

/// The keys of a JSON row, sorted: the parsed object does not keep the printed order.
fn keys(row: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = row
        .as_object()
        .expect("a row is an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

/// `fields`, sorted, to compare with [`keys`].
fn sorted<const N: usize>(mut fields: [&'static str; N]) -> Vec<&'static str> {
    fields.sort_unstable();
    fields.to_vec()
}

/// A metric the operator set a target for is a KPI: `metric kpis` lists it with its target and
/// no more than its view declares, and `metric kpi-readings` lists its newest instance-wide
/// reading with the target it was judged against and whether it met it. K1 ≤ 0.10 is missed by
/// 0.75; K4.median ≤ 3 is met by 2.75; K4.p90 has no target and is no KPI. The reading of
/// 2026-10-13 is superseded by the day's.
#[test]
fn a_targeted_metric_is_a_kpi_listed_with_whether_its_newest_reading_meets_the_target() {
    let case = Case::fixture("kpis");
    // Compute an earlier day first, so the metrics are defined and a reading precedes the day's.
    case.ok(&["metric", "compute", "--day", "2026-10-13"]);
    for (metric, target) in [("K1", "0.10"), ("K4.median", "3")] {
        case.ok(&[
            "metric",
            "set-metric-target",
            "--metric-id",
            metric,
            "--target",
            target,
            "--direction",
            "AtMost",
            "--target-owner",
            "operator",
            "--target-set-at",
            "2026-10-14T23:30:00Z",
        ]);
    }
    case.compute();

    let kpis = case.rows("metric", "kpis");
    let mut ids: Vec<&str> = kpis
        .iter()
        .map(|row| row["metric_id"].as_str().expect("an id"))
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, ["K1", "K4.median"], "{kpis:#?}");
    let kpi = |id: &str| {
        kpis.iter()
            .find(|row| row["metric_id"] == id)
            .expect("the KPI")
            .clone()
    };
    for row in &kpis {
        assert_eq!(
            keys(row),
            sorted(KPI_FIELDS),
            "`metric kpis` prints its view's fields only"
        );
    }
    let k1 = kpi("K1");
    assert_eq!(k1["state"], "Targeted", "{k1}");
    close(value(&k1["target"]), 0.1, "K1's target");
    assert_eq!(k1["direction"], "AtMost", "{k1}");
    assert_eq!(k1["target_owner"], "operator", "{k1}");

    let readings = case.rows("metric", "kpi-readings");
    for row in &readings {
        assert_eq!(keys(row), sorted(KPI_READING_FIELDS), "{row}");
    }
    let mut ids: Vec<&str> = readings
        .iter()
        .map(|row| row["metric_id"].as_str().expect("an id"))
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, ["K1", "K4.median"], "one row per KPI: {readings:#?}");
    let reading = |id: &str| {
        readings
            .iter()
            .find(|row| row["metric_id"] == id)
            .expect("the KPI's reading")
            .clone()
    };
    let k1 = reading("K1");
    assert_eq!(k1["scope"], "instance", "{k1}");
    assert_eq!(k1["day"], DAY, "{k1}");
    assert_eq!(k1["reading_id"], format!("K1|instance|{DAY}"), "{k1}");
    close(value(&k1["value"]), 0.75, "K1's newest value");
    close(
        value(&k1["target"]),
        0.1,
        "the target K1's reading was judged against",
    );
    assert_eq!(k1["met"], false, "{k1}");
    let k4 = reading("K4.median");
    assert_eq!(k4["day"], DAY, "{k4}");
    close(value(&k4["value"]), 2.75, "K4.median's newest value");
    close(
        value(&k4["target"]),
        3.0,
        "the target K4.median's reading was judged against",
    );
    assert_eq!(k4["met"], true, "{k4}");

    // The reading recorded before the target keeps no verdict and is superseded by the day's:
    // K1 on 2026-10-13 is REQ-3 and REQ-5 escalated over alpha#1 and beta#6 merged, 2 / 2 = 1.
    let earlier = case
        .all()
        .into_iter()
        .find(|row| row["reading_id"] == "K1|instance|2026-10-13")
        .expect("the earlier K1 reading");
    close(value(&earlier["value"]), 1.0, "K1 on 2026-10-13");
    assert!(earlier["met"].is_null(), "{earlier}");
    assert!(earlier["target"].is_null(), "{earlier}");
    assert_eq!(earlier["state"], "Superseded", "{earlier}");
    let newest = case
        .all()
        .into_iter()
        .find(|row| row["reading_id"] == format!("K1|instance|{DAY}"))
        .expect("the day's K1 reading");
    assert_eq!(newest["state"], "Recorded", "{newest}");

    // Computing the earlier day again records nothing and supersedes nothing newer.
    case.ok(&["metric", "compute", "--day", "2026-10-13"]);
    assert_eq!(case.rows("metric", "kpi-readings"), readings);

    // A second target replaces the first: the KPI stays one.
    case.ok(&[
        "metric",
        "set-metric-target",
        "--metric-id",
        "K1",
        "--target",
        "0.8",
        "--direction",
        "AtMost",
        "--target-owner",
        "operator",
        "--target-set-at",
        "2026-10-15T09:00:00Z",
    ]);
    let k1 = case
        .rows("metric", "kpis")
        .into_iter()
        .find(|row| row["metric_id"] == "K1")
        .expect("K1");
    close(value(&k1["target"]), 0.8, "K1's new target");
    assert_eq!(k1["target_set_at"], "2026-10-15T09:00:00Z", "{k1}");
}

// ---------------------------------------------------------------------------------------------
// The commands, by hand.
// ---------------------------------------------------------------------------------------------

fn refused(output: &Output, error: &str, what: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "{what}: {}",
        describe(output)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{what}: {}", describe(output));
    assert!(stderr.contains(error), "{what}: {}", describe(output));
}

/// define-metric, record-metric-reading and set-metric-target answer their declared refusals.
#[test]
fn the_metric_commands_answer_their_declared_refusals() {
    let case = Case::empty("commands");
    let define = |id: &str| {
        case.run(&[
            "metric",
            "define-metric",
            "--metric-id",
            id,
            "--name",
            "a metric",
            "--unit",
            "count",
            "--instrument",
            "by hand",
            "--definition",
            "what it counts",
        ])
    };
    let record = |metric: &str, reading: &str| {
        case.run(&[
            "metric",
            "record-metric-reading",
            "--reading-id",
            reading,
            "--metric-id",
            metric,
            "--day",
            DAY,
            "--scope",
            "instance",
            "--value",
            "2",
            "--sources",
            r#"["a"]"#,
            "--target",
            "1",
            "--met",
            "true",
            "--recorded-at",
            "2026-10-15T00:00:00Z",
        ])
    };
    let supersede = |reading: &str| {
        case.run(&[
            "metric",
            "supersede-metric-reading",
            "--reading-id",
            reading,
            "--by",
            "X1|instance|2026-10-15",
        ])
    };
    let target = |metric: &str| {
        case.run(&[
            "metric",
            "set-metric-target",
            "--metric-id",
            metric,
            "--target",
            "1",
            "--direction",
            "AtLeast",
            "--target-owner",
            "operator",
            "--target-set-at",
            "2026-10-15T00:00:00Z",
        ])
    };

    refused(
        &record("X1", "X1|instance|2026-10-14"),
        "MetricNotFound",
        "a reading of no metric",
    );
    refused(&target("X1"), "MetricNotFound", "a target on no metric");
    assert!(define("X1").status.success());
    refused(&define("X1"), "MetricExists", "a second definition");
    assert!(record("X1", "X1|instance|2026-10-14").status.success());
    refused(
        &record("X1", "X1|instance|2026-10-14"),
        "MetricReadingExists",
        "a second reading",
    );
    assert!(target("X1").status.success());
    let rows = case.rows("metric", "metrics");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert_eq!(rows[0]["state"], "Targeted", "{rows:#?}");
    assert_eq!(rows[0]["direction"], "AtLeast", "{rows:#?}");
    let kpis = case.rows("metric", "kpis");
    assert_eq!(kpis.len(), 1, "{kpis:#?}");
    assert_eq!(keys(&kpis[0]), sorted(KPI_FIELDS), "{kpis:#?}");
    assert_eq!(kpis[0]["target"], json!("1"), "{kpis:#?}");

    let readings = case.rows("metric", "kpi-readings");
    assert_eq!(readings.len(), 1, "{readings:#?}");
    assert_eq!(readings[0]["value"], json!("2"), "{readings:#?}");
    assert_eq!(readings[0]["target"], json!("1"), "{readings:#?}");
    assert_eq!(readings[0]["met"], true, "{readings:#?}");

    refused(
        &supersede("X1|instance|2026-10-01"),
        "MetricReadingNotFound",
        "superseding no reading",
    );
    assert!(supersede("X1|instance|2026-10-14").status.success());
    refused(
        &supersede("X1|instance|2026-10-14"),
        "MetricReadingStateConflict",
        "superseding a superseded reading",
    );
    assert_eq!(
        case.rows("metric", "kpi-readings"),
        Vec::<Value>::new(),
        "a superseded reading is no KPI's newest"
    );
    assert_eq!(case.all()[0]["state"], "Superseded", "{:#?}", case.all());
}

/// The views create nothing: without a store each answers an error and leaves no `state/`.
#[test]
fn the_metric_views_create_no_store() {
    let case = Case::empty("views");
    fs::remove_dir_all(case.state()).expect("remove state/");
    for view in ["metrics", "kpis", "metric-readings", "kpi-readings"] {
        let output = case.run(&["metric", view]);
        assert!(!output.status.success(), "{view}: {}", describe(&output));
        assert!(!case.state().exists(), "`metric {view}` created the store");
    }
}
