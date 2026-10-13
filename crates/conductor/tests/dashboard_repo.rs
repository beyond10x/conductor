//! `story:dashboard-repo-view`: what the repository page reads, through the library's
//! [`conductor_cli::dashboard::repo`]: what each repository waits on (one case per cause, each
//! with a fixture that meets its rule and one that just misses it), a repository's sessions, its
//! share of the events, and the plan of the newest complete snapshot. The last case holds the
//! keys of the page's fixtures (`crates/conductor/dashboard/src/views/__tests__/fixtures/`) to the
//! rows these answer.
//!
//! Every case keeps its files under its own directory in this test target's temporary
//! directory; nothing is read from the environment.

mod active;

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use conductor_cli::collect::Collector;
use conductor_cli::dashboard::repo::{
    self, CAUSES, Facts, Need, PullRequest, Request, ResourceWait,
};
use conductor_cli::dashboard::{HealthSources, Sources};
use conductor_cli::snapshot::{self, Recorder};
use conductor_cli::store;
use conductor_cli::usage;
use conductor_model::observation::{
    RecordBlocker, RecordSpecification, RepositoryName, SnapshotState, SpecificationPresence,
    StartSnapshot, ValidationResult,
};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, Time};

/// A fresh directory for one case, with `home/` in it.
fn case_dir(case: &str) -> PathBuf {
    active::isolate();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("dashboard-repo")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(dir.join("home")).expect("create the case's home");
    dir
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
    fs::write(path, text).expect("write a case file");
}

/// The checkouts root and the managed trees every fact case uses.
const CHECKOUTS: &str = "/fixture-home/example-org";
const TREES: &str = "/fixture-home/.local/state/worktree/trees/example-org";

/// A session-list row: `name`, working in `cwd`, in list state `state`, live when `live`.
fn session(name: &str, cwd: &str, id: &str, state: &str, live: bool) -> Value {
    json!({
        "name": name, "kind": "background", "cwd": cwd, "sessionId": id, "state": state,
        "pid": if live { json!(4242) } else { Value::Null },
        "startedAt": 1_791_000_000_000_i64,
    })
}

/// Facts with `alpha` and `beta`, `alpha`'s controller idle in its checkout and `beta`'s in its.
fn facts() -> Facts {
    Facts {
        checkouts: PathBuf::from(CHECKOUTS),
        trees: PathBuf::from(TREES),
        repositories: vec!["alpha".to_owned(), "beta".to_owned()],
        sessions: vec![
            session(
                "alpha",
                &format!("{CHECKOUTS}/alpha"),
                "aaaaaaaa-1",
                "idle",
                true,
            ),
            session(
                "beta",
                &format!("{CHECKOUTS}/beta"),
                "bbbbbbbb-1",
                "working",
                true,
            ),
        ],
        ..Facts::default()
    }
}

/// The cause names `/api/waiting-on` answers for `repository` over `facts`.
fn causes_of(facts: &Facts, repository: &str) -> Vec<String> {
    let answer = repo::waiting_on(facts);
    let row = answer["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["repository"] == repository)
        .unwrap_or_else(|| panic!("no row for {repository}: {answer}"))
        .clone();
    row["causes"]
        .as_array()
        .expect("causes")
        .iter()
        .map(|cause| cause["cause"].as_str().expect("a cause name").to_owned())
        .collect()
}

#[test]
fn operator_an_escalated_class_o_or_h_request_and_not_an_answered_one() {
    let mut meets = facts();
    meets.requests = vec![Request {
        request_id: "alpha-1".to_owned(),
        repository: "alpha".to_owned(),
        open: true,
        class: Some("H".to_owned()),
        question: "a login?".to_owned(),
    }];
    assert_eq!(causes_of(&meets, "alpha"), ["operator"]);
    assert!(causes_of(&meets, "beta").is_empty());

    let mut misses = meets.clone();
    misses.requests[0].open = false;
    assert!(causes_of(&misses, "alpha").is_empty(), "answered");
    let mut misses = meets;
    misses.requests[0].class = None;
    assert!(causes_of(&misses, "alpha").is_empty(), "not escalated");
}

#[test]
fn ci_a_failing_pull_request_while_no_session_of_the_repository_works() {
    let mut meets = facts();
    meets.pull_requests = vec![PullRequest {
        repository: "alpha".to_owned(),
        number: 7,
        failing_checks: 1,
    }];
    assert_eq!(causes_of(&meets, "alpha"), ["ci"]);

    let mut misses = meets.clone();
    misses.sessions.push(session(
        "alpha-worker",
        &format!("{TREES}/alpha/alpha-w01"),
        "cccccccc-1",
        "working",
        true,
    ));
    assert!(
        causes_of(&misses, "alpha").is_empty(),
        "a session in a managed tree works"
    );
    let mut misses = meets;
    misses.pull_requests[0].failing_checks = 0;
    assert!(causes_of(&misses, "alpha").is_empty(), "no failing check");
}

#[test]
fn build_slot_a_requested_resource_and_not_a_granted_one() {
    let mut meets = facts();
    meets.resource_requests = vec![ResourceWait {
        resource_request_id: "rr-1".to_owned(),
        repository: "alpha".to_owned(),
        resource: "BuildSlot".to_owned(),
        requested: true,
    }];
    assert_eq!(causes_of(&meets, "alpha"), ["build-slot"]);

    let mut misses = meets;
    misses.resource_requests[0].requested = false;
    assert!(causes_of(&misses, "alpha").is_empty());
}

#[test]
fn another_repository_a_need_routed_to_an_open_dispatch_of_another_repository() {
    let mut meets = facts();
    meets.needs = vec![Need {
        sender: "alpha".to_owned(),
        first_line: "[NEED beta] an endpoint".to_owned(),
        dispatch_id: Some("DSP-20261013-01".to_owned()),
        dispatch_repository: Some("beta".to_owned()),
        dispatch_open: true,
    }];
    assert_eq!(causes_of(&meets, "alpha"), ["another-repository"]);
    let answer = repo::waiting_on(&meets);
    let alpha = answer["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["repository"] == "alpha")
        .expect("alpha's row");
    assert_eq!(alpha["causes"][0]["on"], "beta", "names the repository");
    assert!(causes_of(&meets, "beta").is_empty());

    let mut misses = meets.clone();
    misses.needs[0].dispatch_open = false;
    assert!(
        causes_of(&misses, "alpha").is_empty(),
        "the dispatch is done"
    );
    let mut misses = meets;
    misses.needs[0].dispatch_repository = Some("alpha".to_owned());
    assert!(causes_of(&misses, "alpha").is_empty(), "its own repository");
}

#[test]
fn usage_limit_the_watch_names_the_repository_s_session() {
    let mut meets = facts();
    meets.limits = Some("5-hour limit reached ∙ resets 14:00\talpha\n".to_owned());
    assert_eq!(causes_of(&meets, "alpha"), ["usage-limit"]);
    assert!(causes_of(&meets, "beta").is_empty());

    let mut misses = meets;
    misses.limits = Some("5-hour limit reached ∙ resets 14:00\talpha-old\n".to_owned());
    assert!(causes_of(&misses, "alpha").is_empty());
}

#[test]
fn silent_the_watch_reported_the_repository_s_session() {
    let mut meets = facts();
    meets.silent = Some("aaaaaaaa-1 1791000000000000000\n".to_owned());
    assert_eq!(causes_of(&meets, "alpha"), ["silent"]);
    assert!(causes_of(&meets, "beta").is_empty());

    let mut misses = meets;
    misses.silent = Some("aaaaaaaa-2 1791000000000000000\n".to_owned());
    assert!(causes_of(&misses, "alpha").is_empty());
}

#[test]
fn every_cause_is_named_in_the_order_the_page_lists_them() {
    assert_eq!(
        CAUSES,
        [
            "operator",
            "ci",
            "build-slot",
            "another-repository",
            "usage-limit",
            "silent"
        ]
    );
}

/// The sources of a case under `home`: the checkouts root `home/example-org`, the managed trees
/// under `home/.local/state/worktree/trees/example-org`, the watch under `home/watch`.
fn sources(home: &Path) -> Sources {
    let cat = vec![OsString::from("cat"), OsString::from("/dev/null")];
    Sources {
        agents: cat.clone(),
        root: home.join("records"),
        ci_state: home.join("watch/ci.state"),
        home: home.to_path_buf(),
        checkouts: home.join("example-org"),
        trees: home.join(".local/state/worktree/trees/example-org"),
        targets: home.join(".cache/target"),
        disk: PathBuf::from("/"),
        watch: home.join("watch"),
        sweep: cat.clone(),
        gc: cat,
        var_tmp: home.join("var-tmp"),
        measure_every: Duration::from_secs(600),
        measure_timeout: Duration::from_secs(30),
        context_handover: 300_000,
        health: HealthSources::new(home.to_path_buf(), &home.join("records")),
    }
}

/// An assistant line of the transcript of `session` at `at` with one call.
fn call(session: &str, id: &str, at: OffsetDateTime) -> String {
    json!({
        "type": "assistant", "sessionId": session,
        "timestamp": at.format(&Rfc3339).expect("format an instant"),
        "message": {"id": id, "type": "message", "role": "assistant", "model": "claude-fixture",
            "content": [{"type": "text", "text": "fixture"}],
            "usage": {"input_tokens": 10, "output_tokens": 20, "cache_read_input_tokens": 300,
                "cache_creation_input_tokens": 40}},
    })
    .to_string()
}

#[test]
fn a_repository_s_sessions_carry_their_dispatch_transcript_and_today_s_spend() {
    let dir = case_dir("sessions");
    let home = dir.join("home");
    let sources = sources(&home);
    let alpha = home.join("example-org/alpha");
    let tree = home.join(".local/state/worktree/trees/example-org/alpha/alpha-w01");
    let beta = home.join("example-org/beta");
    for checkout in [&alpha, &tree, &beta] {
        fs::create_dir_all(checkout).expect("create a checkout");
    }
    let agents = vec![
        session(
            "alpha",
            &alpha.display().to_string(),
            "s-alpha",
            "working",
            true,
        ),
        session(
            "alpha-w01",
            &tree.display().to_string(),
            "s-tree",
            "idle",
            false,
        ),
        session("beta", &beta.display().to_string(), "s-beta", "idle", true),
    ];
    let dispatches = vec![
        json!({"id": "DSP-20261013-01", "to": "alpha", "repository": "alpha", "goal": "G1",
            "brief": "Fix the refund retries", "sent_at": "2026-10-13T08:00:00Z"}),
    ];

    let now = OffsetDateTime::now_utc();
    let midnight = now.replace_time(Time::MIDNIGHT);
    let transcript = home
        .join(".claude/projects")
        .join(encode(&alpha))
        .join("s-alpha.jsonl");
    write(
        &transcript,
        &format!(
            "{}\n{}\n",
            call(
                "s-alpha",
                "msg-yesterday",
                midnight - time::Duration::hours(1)
            ),
            call(
                "s-alpha",
                "msg-today",
                midnight + time::Duration::seconds(1)
            ),
        ),
    );
    let measured = usage::measure(
        &usage::Sources {
            projects: home.join(".claude/projects"),
            home: home.clone(),
            root: sources.checkouts.clone(),
            trees: sources.trees.clone(),
        },
        Some(midnight),
    );
    let usage = json!({"rows": measured.repository_objects()});

    let answer = repo::sessions(&sources, "alpha", &agents, &dispatches, &usage, now);
    let rows = answer["rows"].as_array().expect("rows");
    let names: Vec<&str> = rows
        .iter()
        .map(|row| row["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, ["alpha", "alpha-w01"], "beta's session is absent");
    let keys: BTreeSet<&str> = rows[0]
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        BTreeSet::from([
            "name",
            "repository",
            "state",
            "started_at",
            "dispatch",
            "transcript",
            "spend_today"
        ])
    );
    assert_eq!(rows[0]["repository"], "alpha");
    assert_eq!(rows[0]["state"], "working");
    assert_eq!(rows[1]["state"], "exited");
    assert!(rows[0]["started_at"].is_string());
    assert_eq!(
        rows[0]["dispatch"],
        json!({"id": "DSP-20261013-01", "brief": "Fix the refund retries"})
    );
    assert_eq!(rows[1]["dispatch"], Value::Null);
    let path = rows[0]["transcript"].as_str().expect("a transcript path");
    assert!(path.starts_with("~/"), "{path}");
    assert_eq!(
        path,
        format!("~/.claude/projects/{}/s-alpha.jsonl", encode(&alpha))
    );
    assert_eq!(
        rows[0]["spend_today"]["calls"], 1,
        "today's call counts and yesterday's does not: {}",
        rows[0]["spend_today"]
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A path as the session tool names its project directory.
fn encode(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[test]
fn a_repository_s_events_are_its_dispatch_report_decision_release_and_merge_rows() {
    let row = |repository: Value, kind: &str| {
        json!({"at": "2026-10-13T09:00:00Z", "repository": repository, "kind": kind,
            "text": format!("{kind} of {repository}")})
    };
    let mut rows = Vec::new();
    for kind in ["dispatch", "report", "decision", "release", "merge"] {
        rows.push(row(json!("alpha"), kind));
        rows.push(row(json!("beta"), kind));
    }
    rows.push(row(Value::Null, "decision"));
    let alpha = repo::events_for_repository(rows, "alpha");
    let kinds: Vec<&str> = alpha
        .iter()
        .map(|row| row["kind"].as_str().expect("a kind"))
        .collect();
    assert_eq!(
        kinds,
        ["dispatch", "report", "decision", "release", "merge"]
    );
    assert!(alpha.iter().all(|row| row["repository"] == "alpha"));
}

#[test]
fn a_repository_name_is_read_from_the_query() {
    assert_eq!(
        repo::query("/api/sessions?repository=alpha", "repository").as_deref(),
        Some("alpha")
    );
    assert_eq!(
        repo::query(
            "/api/events?since=2026-10-13T00%3A00%3A00Z&repository=beta-gamma",
            "since"
        )
        .as_deref(),
        Some("2026-10-13T00:00:00Z")
    );
    assert_eq!(repo::query("/api/sessions", "repository"), None);
}

/// A fake collector recording `alpha`'s blocker and specification.
fn plan() -> Collector {
    Collector::new("fixture", |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        record.blocker(RecordBlocker {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName("alpha".to_owned()),
            reference: "story:partner-sandbox".to_owned(),
            blocker_kind: "upstream-blocker".to_owned(),
            title: "partner API keys".to_owned(),
        })?;
        record.specification(RecordSpecification {
            snapshot_id: snapshot,
            repository: RepositoryName("alpha".to_owned()),
            presence: SpecificationPresence::Present,
            path: Some("spec".to_owned()),
            format: Some("ess/22".to_owned()),
            required_ess: None,
            validation: ValidationResult::Valid,
            validation_refusals: 0,
            scenarios: Some(214),
            synthesis_refusals: None,
            conformance_status: None,
        })?;
        Ok(())
    })
}

fn fails() -> Collector {
    Collector::new("fails", |_: &mut Recorder<'_>| {
        anyhow::bail!("the source did not answer")
    })
}

fn take(state: &Path, started_at: &str, collector: Collector) -> SnapshotState {
    let store = store::open(state).expect("open the store");
    let start = StartSnapshot {
        started_at: Timestamp(started_at.to_owned()),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    snapshot::take_from(store, start, &[collector])
        .expect("the driver ends the snapshot")
        .state
}

#[test]
fn the_plan_is_the_newest_complete_snapshot_s_blockers_and_specifications() {
    let dir = case_dir("plan");
    let state = dir.join("state");

    assert_eq!(
        repo::blockers(None),
        json!({"snapshot": null, "rows": []}),
        "no store"
    );
    assert_eq!(
        take(&state, "2026-10-13T08:00:00Z", plan()),
        SnapshotState::Complete
    );
    assert_eq!(
        take(&state, "2026-10-13T09:00:00Z", fails()),
        SnapshotState::Failed
    );
    let held = store::open_existing(&state).expect("the store opens");

    let blockers = repo::blockers(Some(&held));
    assert_eq!(blockers["snapshot"]["started_at"], "2026-10-13T08:00:00Z");
    let rows = blockers["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 1, "{blockers}");
    assert_eq!(rows[0]["repository"], "alpha");
    assert_eq!(rows[0]["blocker_kind"], "upstream-blocker");
    assert_eq!(rows[0]["reference"], "story:partner-sandbox");

    let specifications = repo::specifications(Some(&held));
    assert_eq!(
        specifications["snapshot"]["started_at"],
        "2026-10-13T08:00:00Z"
    );
    let rows = specifications["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 1, "{specifications}");
    assert_eq!(rows[0]["presence"], "Present");
    assert_eq!(rows[0]["validation"], "Valid");
    assert_eq!(rows[0]["scenarios"], 214);
    drop(held);
    let _ = fs::remove_dir_all(&dir);
}

/// The page's fixture `name`, under `crates/conductor/dashboard/src/views/__tests__/fixtures/`.
fn page_fixture(name: &str) -> Value {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("dashboard/src/views/__tests__/fixtures")
        .join(format!("{name}.json"));
    let text = fs::read_to_string(&file)
        .unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
    serde_json::from_str(&text).expect("the fixture is JSON")
}

fn keys(row: &Value) -> BTreeSet<String> {
    row.as_object()
        .unwrap_or_else(|| panic!("{row} is no object"))
        .keys()
        .cloned()
        .collect()
}

/// The keys of every row of `answer`, which must all be alike.
fn row_keys(answer: &Value) -> BTreeSet<String> {
    let rows = answer["rows"].as_array().expect("rows");
    let first = keys(rows.first().expect("a row"));
    for row in rows {
        assert_eq!(keys(row), first, "{row}");
    }
    first
}

#[test]
fn the_page_s_fixtures_have_the_keys_the_endpoints_answer() {
    // /api/waiting-on: rows and causes.
    let mut served = facts();
    served.needs = vec![Need {
        sender: "alpha".to_owned(),
        first_line: "[NEED beta] an endpoint".to_owned(),
        dispatch_id: Some("DSP-20261013-01".to_owned()),
        dispatch_repository: Some("beta".to_owned()),
        dispatch_open: true,
    }];
    served.limits = Some("limit\talpha\n".to_owned());
    let answer = repo::waiting_on(&served);
    let fixture = page_fixture("waiting-on");
    assert_eq!(row_keys(&fixture), row_keys(&answer));
    let mut served_causes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in answer["rows"].as_array().expect("rows") {
        for cause in row["causes"].as_array().expect("causes") {
            served_causes.insert(
                cause["cause"].as_str().expect("a name").to_owned(),
                keys(cause),
            );
        }
    }
    for row in fixture["rows"].as_array().expect("rows") {
        for cause in row["causes"].as_array().expect("causes") {
            let name = cause["cause"].as_str().expect("a name");
            assert!(CAUSES.contains(&name), "{name} is no cause");
            let expected = served_causes.get(name).cloned().unwrap_or_else(|| {
                if name == "another-repository" {
                    BTreeSet::from(["cause".into(), "detail".into(), "on".into()])
                } else {
                    BTreeSet::from(["cause".into(), "detail".into()])
                }
            });
            assert_eq!(keys(cause), expected, "{cause}");
        }
    }

    // /api/sessions?repository=: the keys of a served row.
    let dir = case_dir("fixture-keys");
    let home = dir.join("home");
    let sources = sources(&home);
    let agents = vec![session(
        "alpha",
        &home.join("example-org/alpha").display().to_string(),
        "s-alpha",
        "working",
        true,
    )];
    let answer = repo::sessions(
        &sources,
        "alpha",
        &agents,
        &[],
        &json!({"rows": []}),
        OffsetDateTime::now_utc(),
    );
    assert_eq!(row_keys(&page_fixture("sessions")), row_keys(&answer));

    // /api/blockers and /api/specifications: the keys of a served row.
    let state = dir.join("state");
    take(&state, "2026-10-13T08:00:00Z", plan());
    let held = store::open_existing(&state).expect("the store opens");
    for (name, answer) in [
        ("blockers", repo::blockers(Some(&held))),
        ("specifications", repo::specifications(Some(&held))),
    ] {
        let fixture = page_fixture(name);
        assert_eq!(keys(&fixture), keys(&answer), "{name}");
        assert_eq!(
            keys(&fixture["snapshot"]),
            keys(&answer["snapshot"]),
            "{name}"
        );
        assert_eq!(row_keys(&fixture), row_keys(&answer), "{name}");
    }
    drop(held);
    let _ = fs::remove_dir_all(&dir);
}
