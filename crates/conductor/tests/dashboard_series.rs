//! `story:dashboard-series-api`: `GET /api/series` lists the series, and
//! `GET /api/series?series=<name>&by=<key>&days=<n>` answers each one's rows of day, key and value
//! from the daily rollups of the fixture (`series_fixture/mod.rs`); a series without a source
//! answers 404 naming the story that adds it, an unknown series or `by` 400 naming the allowed
//! values. The rows are read through [`series::answer`] at the fixture's "today"; the served route
//! is checked over HTTP, where "today" is the runner's.

mod active;
mod series_fixture;

use std::ffi::OsString;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;

use conductor_cli::dashboard::{self, Sources, series};
use conductor_model::behaviour::HostGrantStorage;
use conductor_model::config::InstanceName;
use conductor_model::host::{GrantId, GrantKind, HostGrantData, HostGrantSnapshot, HostGrantState};
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};
use series_fixture::{Case, today};

/// One row of a series: its day, key and value.
type Row = (String, String, Value);

/// The answer to `target` over the fixture's store, at the fixture's today.
fn ask(case: &Case, target: &str) -> (u16, Value) {
    let store = case.store();
    series::answer(target, Some(&store), today())
}

/// The rows of `series` grouped by `by` over 30 days, as `(day, key, value)`.
fn rows(case: &Case, query: &str) -> Vec<Row> {
    let (status, answer) = ask(case, &format!("/api/series?{query}"));
    assert_eq!(status, 200, "{query}: {answer}");
    answer["rows"]
        .as_array()
        .unwrap_or_else(|| panic!("{query}: no rows in {answer}"))
        .iter()
        .map(|row| {
            (
                row["day"].as_str().expect("a day").to_owned(),
                row["key"].as_str().expect("a key").to_owned(),
                row["value"].clone(),
            )
        })
        .collect()
}

fn row(day: &str, key: &str, value: Value) -> Row {
    (day.to_owned(), key.to_owned(), value)
}

/// Case `name`'s directory, the process's instance fixed first: the dashboard reads it.
fn case(name: &str) -> Case {
    active::isolate();
    Case::new("dashboard-series", name)
}

#[test]
fn the_list_names_every_series_with_its_source_or_story() {
    let case = case("list");
    let (status, answer) = ask(&case, "/api/series");
    assert_eq!(status, 200);
    let list = answer.as_array().expect("an array");
    let names: Vec<&str> = list
        .iter()
        .map(|entry| entry["series"].as_str().expect("a name"))
        .collect();
    for name in [
        "prs_opened",
        "prs_merged",
        "prs_closed_unmerged",
        "reverts",
        "ci_passed",
        "ci_failed",
        "releases",
        "merged_to_released_hours",
        "dispatch_lead_hours",
        "operator_touches",
        "guard_denials",
        "disk_granted",
        "disk_refused",
        "disk_waited",
        "disk_peak_vs_reserved",
        "tokens_in",
        "tokens_out",
        "idle_hours",
        "waiting_hours",
        "operator_touches_per_pr",
        "unnecessary_escalations_pct",
        "ci_failed_per_pr",
        "patch_per_feature_release",
    ] {
        assert!(names.contains(&name), "{name} is listed: {names:?}");
    }
    let merged = list
        .iter()
        .find(|entry| entry["series"] == "prs_merged")
        .expect("prs_merged");
    assert_eq!(merged["title"], "PRs merged");
    assert_eq!(merged["unit"], "count");
    assert_eq!(merged["by"], json!(["repository"]));
    assert_eq!(merged["kpi"], "K1");
    assert!(merged["source"].is_string());
    let tokens = list
        .iter()
        .find(|entry| entry["series"] == "tokens_out")
        .expect("tokens_out");
    assert_eq!(tokens["source"], Value::Null);
    assert_eq!(tokens["story"], "story:kpi-transcripts");
    assert_eq!(tokens["by"], json!(["repository", "session"]));
    for entry in list {
        for field in ["series", "title", "unit", "by", "kpi", "source"] {
            assert!(entry.get(field).is_some(), "{field} in {entry}");
        }
    }
}

#[test]
fn each_series_with_a_source_answers_its_rows() {
    let case = case("rows");
    case.build();
    let cases: Vec<(&str, Vec<Row>)> = vec![
        (
            "series=prs_merged&by=repository",
            vec![
                row("2026-10-11", "alpha", json!(1)),
                row("2026-10-12", "alpha", json!(1)),
                row("2026-10-12", "beta", json!(1)),
                row("2026-10-13", "alpha", json!(1)),
                row("2026-10-13", "beta", json!(1)),
            ],
        ),
        (
            "series=prs_merged",
            vec![
                row("2026-10-11", "total", json!(1)),
                row("2026-10-12", "total", json!(2)),
                row("2026-10-13", "total", json!(2)),
            ],
        ),
        (
            "series=prs_opened&by=repository",
            vec![
                row("2026-10-11", "alpha", json!(1)),
                row("2026-10-12", "alpha", json!(1)),
                row("2026-10-12", "beta", json!(1)),
                row("2026-10-13", "beta", json!(1)),
            ],
        ),
        (
            "series=prs_closed_unmerged&by=repository",
            vec![row("2026-10-13", "alpha", json!(1))],
        ),
        ("series=reverts", vec![row("2026-10-12", "total", json!(1))]),
        (
            "series=ci_passed&by=workflow",
            vec![
                row("2026-10-12", "check", json!(1)),
                row("2026-10-13", "check", json!(1)),
                row("2026-10-13", "release", json!(1)),
            ],
        ),
        (
            "series=ci_failed&by=repository",
            vec![
                row("2026-10-12", "alpha", json!(1)),
                row("2026-10-12", "beta", json!(1)),
            ],
        ),
        (
            "series=releases&by=repository",
            vec![
                row("2026-10-12", "alpha", json!(1)),
                row("2026-10-13", "alpha", json!(1)),
            ],
        ),
        (
            "series=merged_to_released_hours",
            vec![
                row("2026-10-12", "p50", json!(2)),
                row("2026-10-12", "p90", json!(17)),
                row("2026-10-13", "p50", json!(2)),
                row("2026-10-13", "p90", json!(2)),
            ],
        ),
        (
            "series=dispatch_lead_hours",
            vec![
                row("2026-10-12", "p50", json!(2)),
                row("2026-10-12", "p90", json!(2)),
                row("2026-10-13", "p50", json!(6)),
                row("2026-10-13", "p90", json!(6)),
            ],
        ),
        (
            "series=operator_touches",
            vec![
                row("2026-10-12", "total", json!(2)),
                row("2026-10-13", "total", json!(2)),
            ],
        ),
        (
            "series=guard_denials&by=rule",
            vec![
                row("2026-10-12", "Bash", json!(1)),
                row("2026-10-12", "Edit", json!(1)),
                row("2026-10-13", "Bash", json!(1)),
            ],
        ),
        (
            "series=guard_denials",
            vec![
                row("2026-10-12", "total", json!(2)),
                row("2026-10-13", "total", json!(1)),
            ],
        ),
        (
            "series=disk_granted&by=repository",
            vec![
                row("2026-10-12", "alpha", json!(1)),
                row("2026-10-13", "alpha", json!(1)),
            ],
        ),
        (
            "series=disk_refused&by=repository",
            vec![row("2026-10-12", "beta", json!(1))],
        ),
        (
            "series=disk_waited&by=repository",
            vec![row("2026-10-13", "alpha", json!(1))],
        ),
        (
            "series=operator_touches_per_pr",
            vec![
                row("2026-10-11", "total", json!(0)),
                row("2026-10-12", "total", json!(1)),
                row("2026-10-13", "total", json!(1)),
            ],
        ),
        (
            "series=unnecessary_escalations_pct",
            vec![
                row("2026-10-12", "total", json!(0)),
                row("2026-10-13", "total", json!(100)),
            ],
        ),
        (
            "series=ci_failed_per_pr",
            vec![
                row("2026-10-11", "total", json!(0)),
                row("2026-10-12", "total", json!(1)),
                row("2026-10-13", "total", json!(0)),
            ],
        ),
        (
            "series=patch_per_feature_release",
            vec![row("2026-10-12", "total", json!(0))],
        ),
    ];
    for (query, expected) in cases {
        assert_eq!(
            rows(&case, &format!("{query}&days=30")),
            expected,
            "{query}"
        );
    }
}

/// One GiB, in bytes.
const GIB: i64 = 1 << 30;

/// `story:dashboard-disk-series`: the host ledger's runs, each with the bytes it reserved and the
/// peak it measured (`host record-peak`), charted as the peak over the reservation, in percent,
/// per day of the request and repository. Two alpha runs on 10-13, 1 GiB of 4 and 2 GiB of 2:
/// 3 of 6, 50 %; one beta run on 10-12 over its 2 GiB, 3 GiB: 150 %. A run still running has no
/// final peak, and a run with no peak recorded (the fixture's G-1) nothing to chart: neither
/// counts.
#[test]
fn disk_peak_vs_reserved_charts_the_runs_peak_over_their_reservation() {
    let case = case("disk-peak");
    let b = case.build();
    let mut store = case.store();
    for (id, repository, state, requested, bytes, peak) in [
        (
            "G-11",
            "alpha",
            HostGrantState::Released,
            "2026-10-13T02:00:00Z",
            4 * GIB,
            GIB,
        ),
        (
            "G-12",
            "alpha",
            HostGrantState::Released,
            "2026-10-13T03:00:00Z",
            2 * GIB,
            2 * GIB,
        ),
        (
            "G-13",
            "beta",
            HostGrantState::Released,
            "2026-10-12T09:00:00Z",
            2 * GIB,
            3 * GIB,
        ),
        (
            "G-14",
            "beta",
            HostGrantState::Running,
            "2026-10-13T04:00:00Z",
            GIB,
            8 * GIB,
        ),
    ] {
        let at = |text: &str| Timestamp(text.to_owned());
        HostGrantStorage::put(
            &mut store,
            HostGrantSnapshot {
                state,
                data: HostGrantData {
                    grant_id: GrantId(id.to_owned()),
                    instance: InstanceName("example-org".to_owned()),
                    repository: Some(RepositoryName(repository.to_owned())),
                    kind: GrantKind::Disk,
                    bytes: Some(bytes),
                    peak_bytes: Some(peak),
                    pauses: None,
                    command: Some("cargo test".to_owned()),
                    process_group: None,
                    session_name: None,
                    reclaimed_bytes: None,
                    shortfall_bytes: None,
                    requested_at: at(requested),
                    started_at: Some(at(requested)),
                    released_at: (state == HostGrantState::Released).then(|| at(requested)),
                    release_reason: None,
                },
            },
        );
    }
    store.check().expect("the store kept the runs");
    conductor_cli::rollup::update(&mut store, &b, &case.records()).expect("the rollup of the runs");
    store.check().expect("the store kept the rollups");
    drop(store);

    let (status, list) = ask(&case, "/api/series");
    assert_eq!(status, 200);
    let entry = list
        .as_array()
        .and_then(|list| {
            list.iter()
                .find(|entry| entry["series"] == "disk_peak_vs_reserved")
        })
        .unwrap_or_else(|| panic!("disk_peak_vs_reserved is listed: {list}"));
    assert_eq!(entry["unit"], "percent", "{entry}");
    assert_eq!(entry["by"], json!(["repository"]), "{entry}");
    assert!(
        entry["source"]
            .as_str()
            .is_some_and(|source| source.starts_with("host ledger")),
        "{entry}"
    );
    assert_eq!(entry.get("story"), None, "a series with a source: {entry}");

    assert_eq!(
        rows(&case, "series=disk_peak_vs_reserved&by=repository&days=30"),
        vec![
            row("2026-10-12", "beta", json!(150)),
            row("2026-10-13", "alpha", json!(50)),
        ]
    );
    assert_eq!(
        rows(&case, "series=disk_peak_vs_reserved&days=30"),
        vec![
            row("2026-10-12", "total", json!(150)),
            row("2026-10-13", "total", json!(50)),
        ]
    );
    let (_, answer) = ask(&case, "/api/series?series=disk_peak_vs_reserved");
    assert_eq!(answer["unit"], "percent");
}

#[test]
fn the_answer_names_its_series_by_days_and_unit() {
    let case = case("head");
    case.build();
    let (status, answer) = ask(&case, "/api/series?series=prs_merged&by=repository&days=7");
    assert_eq!(status, 200);
    assert_eq!(answer["series"], "prs_merged");
    assert_eq!(answer["by"], "repository");
    assert_eq!(answer["days"], 7);
    assert_eq!(answer["unit"], "count");
    let (_, total) = ask(&case, "/api/series?series=prs_merged");
    assert_eq!(total["by"], Value::Null, "absent by: the total");
    assert_eq!(total["days"], 30, "30 days without days");
}

#[test]
fn days_bounds_the_rows_to_the_last_days() {
    let case = case("days");
    case.build();
    assert_eq!(
        rows(&case, "series=prs_merged&days=1"),
        vec![row("2026-10-13", "total", json!(2))]
    );
    assert_eq!(
        rows(&case, "series=prs_merged&days=2"),
        vec![
            row("2026-10-12", "total", json!(2)),
            row("2026-10-13", "total", json!(2)),
        ]
    );
    for days in ["0", "366", "many", "-1"] {
        let (status, answer) = ask(&case, &format!("/api/series?series=prs_merged&days={days}"));
        assert_eq!(status, 400, "days={days}: {answer}");
        assert!(answer["error"].is_string(), "{answer}");
    }
}

#[test]
fn a_series_without_a_source_answers_404_naming_its_story() {
    let case = case("no-source");
    case.build();
    for name in ["tokens_in", "tokens_out", "idle_hours", "waiting_hours"] {
        let (status, answer) = ask(&case, &format!("/api/series?series={name}&by=session"));
        assert_eq!(status, 404, "{name}");
        assert_eq!(
            answer,
            json!({"error": "no source yet", "story": "story:kpi-transcripts"}),
            "{name}"
        );
    }
}

#[test]
fn an_unknown_series_or_by_answers_400_naming_the_allowed_values() {
    let case = case("unknown");
    case.build();
    let (status, answer) = ask(&case, "/api/series?series=nothing");
    assert_eq!(status, 400);
    let allowed: Vec<&str> = answer["allowed"]
        .as_array()
        .expect("the allowed series")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(allowed.contains(&"prs_merged"), "{answer}");
    assert!(
        !allowed.contains(&"escalations"),
        "a rollup only a KPI reads is no series: {answer}"
    );
    let (status, answer) = ask(&case, "/api/series?series=prs_merged&by=workflow");
    assert_eq!(status, 400);
    assert_eq!(answer["allowed"], json!(["repository"]), "{answer}");
    let (status, answer) = ask(
        &case,
        "/api/series?series=merged_to_released_hours&by=repository",
    );
    assert_eq!(status, 400);
    assert_eq!(answer["allowed"], json!([]), "{answer}");
}

#[test]
fn without_a_store_a_series_has_no_rows() {
    let case = case("no-store");
    let (status, answer) = series::answer("/api/series?series=prs_merged", None, today());
    assert_eq!(status, 200);
    assert_eq!(answer["rows"], json!([]));
    drop(case);
}

// ---------------------------------------------------------------------------------------------
// Served
// ---------------------------------------------------------------------------------------------

/// Serves the case's store; the port.
fn start(case: &Case) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
    let port = listener.local_addr().expect("the bound address").port();
    let home = case.dir.join("home");
    let mut sources = Sources::new(home.clone(), case.records());
    let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
    sources.agents = cat(Path::new("/dev/null"));
    sources.sweep = cat(Path::new("/dev/null"));
    sources.gc = cat(Path::new("/dev/null"));
    sources.checkouts = home.join("example-org");
    sources.trees = home.join("trees");
    sources.targets = home.join("target");
    sources.watch = home.join("watch");
    sources.ci_state = home.join("watch/ci.state");
    sources.var_tmp = home.join("var-tmp");
    let state = case.state();
    std::thread::spawn(move || dashboard::serve_over(listener, sources, Some(state)));
    port
}

fn request(port: u16, method: &str, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\
         Content-Length: 0\r\n\r\n"
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

#[test]
fn the_dashboard_serves_the_series() {
    let case = case("served");
    case.build();
    let port = start(&case);
    let (status, body) = request(port, "GET", "/api/series");
    assert_eq!(status, 200, "{body}");
    let list: Value = serde_json::from_str(&body).expect("JSON");
    assert!(list.as_array().is_some_and(|list| !list.is_empty()));
    let (status, body) = request(
        port,
        "GET",
        "/api/series?series=prs_merged&by=repository&days=365",
    );
    assert_eq!(status, 200, "{body}");
    let answer: Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(answer["series"], "prs_merged");
    assert!(answer["rows"].is_array());
    let (status, body) = request(port, "GET", "/api/series?series=tokens_out");
    assert_eq!(status, 404, "{body}");
    let answer: Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(answer["story"], "story:kpi-transcripts");
    let (status, body) = request(port, "GET", "/api/series?series=nothing");
    assert_eq!(status, 400, "{body}");
    let (status, _) = request(port, "POST", "/api/series");
    assert_eq!(status, 405);
    let (status, body) = request(port, "GET", "/api/kpis");
    assert_eq!(status, 200, "{body}");
    let kpis: Value = serde_json::from_str(&body).expect("JSON");
    assert_eq!(kpis.as_array().map(Vec::len), Some(16), "{kpis}");
}
