//! `story:observation-retention`: the observations live in their own store, `state/observations/`,
//! one tenant per snapshot, and only the newest `retention.snapshots` complete snapshots' are kept.
//!
//! A case keeps its state directory in this test target's temporary directory in `target/`. It
//! takes snapshots through the library ([`snapshot::take_from`], over collectors that record fixed
//! observations), with this process's config fixed to `tests/fixtures/retention/conductor.yaml`
//! (`retention: {snapshots: 12}`), and reads them back through the built binary with
//! `--state-dir`, a `HOME` of its own and an empty `PATH`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::bail;
use conductor_cli::collect::Collector;
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::{config, store};
use conductor_model::observation::{
    CommitSha, Harness, Mergeability, RecordBlocker, RecordMergedPullRequest, RecordPullRequest,
    RecordRelease, RecordRepository, RecordSession, RecordSpecification, RecordWorkflowRun,
    RepositoryName, RunConclusion, SessionState, SnapshotId, SnapshotState, SpecificationPresence,
    StartSnapshot, ValidationResult, Visibility,
};
use conductor_model::primitives::Timestamp;
use eventlog_core::{
    AppendGroup, AtomicEventStore as _, CommandMeta, EventStore as _, Expected, MAX_READ_LIMIT,
    NewEvent, StreamAppend, StreamId, TenantId, new_event_id, request_hash,
};
use eventlog_tree::TreeEventStore;
use serde_json::{Value, json};
use time::OffsetDateTime;

/// `conductor.observation.Snapshot`, which stays in `state/tree/`.
const SNAPSHOT: &str = "conductor.observation.Snapshot";

/// The views of the observations, by the name the `snapshot` group gives them.
const VIEWS: [&str; 8] = [
    "repositories",
    "pull-requests",
    "workflow-runs",
    "sessions",
    "blockers",
    "specifications",
    "releases",
    "merged-pull-requests",
];

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/retention")
        .join(name)
}

/// Fixes this process's config to the fixture's, whose one instance keeps 12 snapshots: what a
/// snapshot the library takes reads `retention.snapshots` from.
fn configured() {
    static SET: OnceLock<()> = OnceLock::new();
    SET.get_or_init(|| {
        config::set_active(Some(&fixture("conductor.yaml")))
            .expect("the retention fixture is a config file this build reads");
    });
}

/// The compiled specification, `ess specify compile --path spec --format json`.
fn model() -> &'static Value {
    static MODEL: OnceLock<Value> = OnceLock::new();
    MODEL.get_or_init(|| {
        let output = Command::new("ess")
            .args(["specify", "compile", "--format", "json", "--path"])
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec"))
            .output()
            .expect("`ess` is on PATH");
        assert!(output.status.success(), "{}", describe(&output));
        serde_json::from_slice(&output.stdout).expect("`ess` prints JSON")
    })
}

/// Every `*Observation` entity of the observation domain, as the specification declares them.
fn observation_entities() -> BTreeSet<String> {
    let entities = model()["entities"]
        .as_object()
        .expect("the model lists its entities");
    let found: BTreeSet<String> = entities
        .keys()
        .filter(|name| name.starts_with("conductor.observation.") && name.ends_with("Observation"))
        .cloned()
        .collect();
    assert!(
        found.len() >= 8,
        "the observation domain declares its observations: {found:?}"
    );
    found
}

/// One case: `state/`, an empty `work/` the binary runs in, a `home/` that is its `HOME`, and an
/// empty `path/` that is its whole `PATH`.
struct Case {
    root: PathBuf,
    state: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        configured();
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("observation_retention")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case's directory");
        }
        for dir in ["work", "home", "path"] {
            fs::create_dir_all(root.join(dir)).expect("create the case's directories");
        }
        Self {
            state: root.join("state"),
            root,
        }
    }

    /// Takes one snapshot started at `started_at` through `collectors`.
    fn take(&self, started_at: &str, collectors: &[Collector]) -> Taken {
        let store = store::open(&self.state).expect("open the store");
        snapshot::take_from(
            store,
            StartSnapshot {
                started_at: Timestamp(started_at.to_owned()),
                disk_free_bytes: 1,
            },
            collectors,
        )
        .expect("the driver ends the snapshot")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_conductor"))
            .current_dir(self.root.join("work"))
            .env("HOME", self.root.join("home"))
            .env("PATH", self.root.join("path"))
            .env_remove("CONDUCTOR_CONFIG")
            .env_remove("CONDUCTOR_INSTANCE")
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    /// Standard output of `args`, which must succeed.
    fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "`conductor {}`: {}",
            args.join(" "),
            describe(&output)
        );
        String::from_utf8(output.stdout).expect("standard output is UTF-8")
    }

    /// The rows of the view `snapshot <name>`.
    fn view(&self, name: &str) -> Vec<Value> {
        match serde_json::from_str(&self.ok(&["snapshot", name, "--format", "json"])) {
            Ok(Value::Array(rows)) => rows,
            other => panic!("`snapshot {name}` printed {other:?}"),
        }
    }

    /// The snapshot ids the rows of the view `snapshot <name>` name.
    fn snapshots_in(&self, name: &str) -> BTreeSet<String> {
        self.view(name)
            .iter()
            .map(|row| {
                row["snapshot_id"]
                    .as_str()
                    .expect("a row names its snapshot")
                    .to_owned()
            })
            .collect()
    }

    /// A config file with `retention` as the instance's, under the case's directory.
    fn config(&self, retention: &str) -> PathBuf {
        let file = self.root.join("conductor.yaml");
        fs::write(
            &file,
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: case\n\
                 \x20   sources: [{{github: case}}]\n\
                 \x20   checkouts: {{root: ~/case, trees: ~/trees}}\n\
                 {retention}"
            ),
        )
        .expect("write the config file");
        file
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `2026-10-01T00:00:00Z` and `hours` after it, in RFC 3339.
fn at(hours: u32) -> String {
    format!("2026-10-{:02}T{:02}:00:00Z", 1 + hours / 24, hours % 24)
}

/// The names of the directories directly under `dir`; none where it is not there.
fn names(dir: &Path) -> BTreeSet<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return BTreeSet::new();
    };
    entries
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// The tenants of the observation store under `state`.
fn tenants(state: &Path) -> BTreeSet<String> {
    names(&state.join("observations/tenants"))
}

/// The stream types of conductor's tenant in the tree under `state`, by their directories.
fn tree_stream_types(state: &Path) -> BTreeSet<String> {
    names(&state.join("tree/tenants/conductor/streams"))
}

/// Every event of conductor's tenant in the tree under `state`, as `(stream type, stream id)`,
/// read from a handle opened afresh.
fn tree_events(state: &Path) -> Vec<(String, String)> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let log = TreeEventStore::open(state.join("tree"))
            .await
            .expect("open the tree");
        let tenant = TenantId::new("conductor").expect("the tenant");
        let mut events = Vec::new();
        let mut after = 0;
        loop {
            let page = log
                .read_feed(&tenant, after, MAX_READ_LIMIT)
                .await
                .expect("read the tree's feed");
            let more = page.has_more && !page.events.is_empty();
            after = page.next_position;
            events.extend(
                page.events
                    .into_iter()
                    .map(|event| (event.stream_type, event.stream_id)),
            );
            if !more {
                break;
            }
        }
        events
    })
}

/// Plants a `Snapshot` left collecting, started at `started_at`, in the tree under `state`, as a
/// run that stopped leaves one.
fn left_collecting(state: &Path, snapshot: &str, started_at: &str) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let log = TreeEventStore::open(state.join("tree"))
            .await
            .expect("open the tree");
        let tenant = TenantId::new("conductor").expect("the tenant");
        let stream = StreamId::new(tenant, SNAPSHOT, snapshot).expect("the snapshot's stream");
        let events = [NewEvent::new(
            "stored",
            1,
            json!({
                "state": "Collecting",
                "snapshot_id": snapshot,
                "started_at": started_at,
                "disk_free_bytes": 1,
                "failure_reason": null,
            }),
        )
        .expect("an event")];
        let id = new_event_id();
        let meta = CommandMeta {
            idempotency_key: id.clone(),
            request_hash: request_hash(&events).expect("a hash"),
            subject: "conductor".to_owned(),
            actor: "conductor".to_owned(),
            request_id: id.clone(),
            trace_id: id,
            causation_id: None,
            causation_depth: 0,
            occurred_at: OffsetDateTime::now_utc(),
            claim: None,
        };
        log.append(&stream, Expected::NoStream, &events, &meta)
            .await
            .expect("plant the snapshot");
    });
}

fn repository(snapshot: &SnapshotId, name: &str) -> RecordRepository {
    RecordRepository {
        snapshot_id: snapshot.clone(),
        repository: RepositoryName(name.to_owned()),
        visibility: Visibility::Public,
        archived: false,
        local_checkout: true,
        main_head: CommitSha("0a1b2c3".to_owned()),
        main_committed_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        main_commits_7d: 1,
        real_commits_7d: 1,
        behind_main: 0,
        dirty_files: 0,
        worktrees: 1,
        open_issues: 0,
        oldest_open_issue_at: None,
        latest_release: None,
        latest_release_at: None,
        unreleased_commits: None,
        planning_store_version: None,
        in_catalog: Some(true),
    }
}

/// Records one observation of each of the eight kinds.
fn every_kind() -> Collector {
    Collector::new("every-kind", |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        let alpha = RepositoryName("alpha".to_owned());
        record.repository(repository(&snapshot, "alpha"))?;
        record.pull_request(RecordPullRequest {
            snapshot_id: snapshot.clone(),
            repository: alpha.clone(),
            number: 7,
            title: "Seven".to_owned(),
            draft: false,
            mergeability: Mergeability::Mergeable,
            failing_checks: 0,
            opened_at: Timestamp("2026-09-01T00:00:00Z".to_owned()),
            updated_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        })?;
        record.workflow_run(RecordWorkflowRun {
            snapshot_id: snapshot.clone(),
            repository: alpha.clone(),
            workflow: "check".to_owned(),
            conclusion: RunConclusion::Success,
            run_at: Timestamp("2026-10-02T00:00:00Z".to_owned()),
        })?;
        record.session(RecordSession {
            snapshot_id: snapshot.clone(),
            harness: Harness::Claude,
            session_ref: "session-1".to_owned(),
            name: None,
            cwd: "~/alpha".to_owned(),
            repository: Some(alpha.clone()),
            activity: SessionState::Idle,
        })?;
        record.blocker(RecordBlocker {
            snapshot_id: snapshot.clone(),
            repository: alpha.clone(),
            reference: "blocker:one".to_owned(),
            blocker_kind: "decision".to_owned(),
            title: "One".to_owned(),
        })?;
        record.specification(RecordSpecification {
            snapshot_id: snapshot.clone(),
            repository: alpha.clone(),
            presence: SpecificationPresence::Present,
            path: Some("spec".to_owned()),
            format: Some("ess/23".to_owned()),
            required_ess: None,
            validation: ValidationResult::Valid,
            validation_refusals: 0,
            scenarios: Some(1),
            synthesis_refusals: None,
            conformance_status: None,
        })?;
        record.release(RecordRelease {
            snapshot_id: snapshot.clone(),
            repository: alpha.clone(),
            tag: "v1.0.0".to_owned(),
            published_at: Timestamp("2026-09-30T00:00:00Z".to_owned()),
            url: "https://example.invalid/alpha/releases/v1.0.0".to_owned(),
        })?;
        record.merged_pull_request(RecordMergedPullRequest {
            snapshot_id: snapshot,
            repository: alpha,
            number: 6,
            title: "Six".to_owned(),
            merged_at: Timestamp("2026-09-30T00:00:00Z".to_owned()),
            url: "https://example.invalid/alpha/pull/6".to_owned(),
        })?;
        Ok(())
    })
}

/// Records `count` repository observations.
fn repositories(count: usize) -> Collector {
    Collector::new("repositories", move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        for n in 0..count {
            record.repository(repository(&snapshot, &format!("r{n:05}")))?;
        }
        Ok(())
    })
}

/// Records one repository observation, then fails.
fn fails_after_one() -> Collector {
    Collector::new("fails", |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        record.repository(repository(&snapshot, "alpha"))?;
        bail!("the fixture fails here")
    })
}

// ---------------------------------------------------------------------------------------------
// 1. After 14 complete snapshots with `retention.snapshots: 12`, the views list the newest 12, and
//    `state/tree/` holds no observation.
// ---------------------------------------------------------------------------------------------

#[test]
fn after_14_complete_snapshots_the_views_list_the_newest_12_and_the_tree_holds_no_observation() {
    let case = Case::new("fourteen");
    let taken: Vec<String> = (0..14)
        .map(|n| {
            let taken = case.take(&at(2 * n), &[every_kind()]);
            assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
            taken.snapshot_id.0.0
        })
        .collect();
    let newest: BTreeSet<String> = taken[2..].iter().cloned().collect();

    for view in VIEWS {
        assert_eq!(
            case.snapshots_in(view),
            newest,
            "`snapshot {view}` lists the observations of the newest 12 snapshots"
        );
        assert_eq!(
            case.view(view).len(),
            12,
            "{view}: one row per kept snapshot"
        );
    }
    assert_eq!(
        case.snapshots_in("snapshots"),
        taken.iter().cloned().collect::<BTreeSet<_>>(),
        "every Snapshot record stays"
    );
    assert_eq!(
        tenants(&case.state),
        newest,
        "state/observations/ holds one tenant per kept snapshot, named after it"
    );

    let observations = observation_entities();
    let in_tree: Vec<(String, String)> = tree_events(&case.state)
        .into_iter()
        .filter(|(stream_type, _)| observations.contains(stream_type))
        .collect();
    assert!(
        in_tree.is_empty(),
        "state/tree/ holds observation events: {in_tree:?}"
    );
    let types = tree_stream_types(&case.state);
    assert!(
        types.is_disjoint(&observations),
        "state/tree/ keeps observation streams: {types:?}"
    );
    assert!(
        types.contains(SNAPSHOT),
        "the Snapshot records are in state/tree/: {types:?}"
    );
}

/// The class: every `*Observation` entity the specification declares is kept in the tenant of its
/// snapshot under `state/observations/`, and none in `state/tree/`.
#[test]
fn every_observation_entity_of_the_specification_is_kept_in_its_snapshots_tenant() {
    let case = Case::new("every-entity");
    let taken = case.take(&at(0), &[every_kind()]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    let snapshot = &taken.snapshot_id.0.0;
    let held = names(
        &case
            .state
            .join("observations/tenants")
            .join(snapshot)
            .join("streams"),
    );
    let observations = observation_entities();
    let missing: Vec<&String> = observations.difference(&held).collect();
    assert!(
        missing.is_empty(),
        "the snapshot's tenant holds no stream of {missing:?}; it holds {held:?}"
    );
    let types = tree_stream_types(&case.state);
    let in_tree: Vec<&String> = observations.intersection(&types).collect();
    assert!(in_tree.is_empty(), "state/tree/ keeps {in_tree:?}");
}

// ---------------------------------------------------------------------------------------------
// 2. Opening `state/tree/` (the guard's path) does not grow with the observations.
// ---------------------------------------------------------------------------------------------

/// Snapshots taken in each case of the measurement.
const SNAPSHOTS: u32 = 10;

/// Observations each snapshot holds in the case that has them: 10,000 in all.
const PER_SNAPSHOT: usize = 1_000;

/// Of those, the ones each snapshot records through the driver; the rest are written into its
/// tenant in one command ([`plant`]), since one append per observation fsyncs two files and takes
/// the measurement past ten minutes on a busy disk.
const RECORDED: usize = 10;

/// One command that writes `count` repository observations of `snapshot` into its tenant, each as
/// the store writes one (`encode_repository` in `src/store/snapshot.rs`).
fn observations_of(snapshot: &str, count: usize) -> AppendGroup {
    let tenant = TenantId::new(snapshot).expect("the snapshot's tenant");
    let appends = (0..count)
        .map(|n| {
            let observation = new_event_id();
            let body = json!({
                "state": "Recorded",
                "observation_id": observation,
                "snapshot_id": snapshot,
                "repository": format!("p{n:05}"),
                "visibility": "Public",
                "archived": false,
                "local_checkout": true,
                "main_head": "0a1b2c3",
                "main_committed_at": "2026-10-01T00:00:00Z",
                "main_commits_7d": 1,
                "real_commits_7d": 1,
                "behind_main": 0,
                "dirty_files": 0,
                "worktrees": 1,
                "open_issues": 0,
                "oldest_open_issue_at": null,
                "latest_release": null,
                "latest_release_at": null,
                "unreleased_commits": null,
                "planning_store_version": null,
                "in_catalog": true,
            });
            StreamAppend {
                stream: StreamId::new(
                    tenant.clone(),
                    "conductor.observation.RepositoryObservation",
                    observation,
                )
                .expect("the observation's stream"),
                expected: Expected::NoStream,
                events: vec![NewEvent::new("stored", 1, body).expect("an event")],
            }
        })
        .collect::<Vec<_>>();
    let id = new_event_id();
    AppendGroup {
        tenant,
        meta: CommandMeta {
            idempotency_key: id.clone(),
            request_hash: request_hash(
                &appends
                    .iter()
                    .map(|append| &append.events)
                    .collect::<Vec<_>>(),
            )
            .expect("a hash"),
            subject: "conductor".to_owned(),
            actor: "conductor".to_owned(),
            request_id: id.clone(),
            trace_id: id,
            causation_id: None,
            causation_depth: 0,
            occurred_at: OffsetDateTime::now_utc(),
            claim: None,
        },
        appends,
    }
}

/// Writes `count` repository observations of each of `snapshots` into its tenant of the
/// observation store under `state`, one command per snapshot, through one handle.
fn plant(state: &Path, snapshots: &[String], count: usize) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let log = TreeEventStore::open(state.join("observations"))
            .await
            .expect("open the observation store");
        for snapshot in snapshots {
            log.append_group(&observations_of(snapshot, count))
                .await
                .expect("write the observations");
        }
    });
}

/// Opens of the store measured in each case; the cheapest is the cost of the open itself.
const OPENS: usize = 7;

/// The time of the first open of the store under `state`, and the cheapest of [`OPENS`].
fn open_times(state: &Path) -> (Duration, Duration) {
    let mut times = Vec::new();
    for _ in 0..OPENS {
        let started = Instant::now();
        let opened = store::open(state).expect("open the store");
        times.push(started.elapsed());
        drop(opened);
    }
    (times[0], *times.iter().min().expect("an open"))
}

/// The number of files under `state/tree/tenants/`.
fn tree_files(state: &Path) -> usize {
    fn count(dir: &Path) -> usize {
        fs::read_dir(dir).map_or(0, |entries| {
            entries
                .map(|entry| {
                    let entry = entry.expect("a directory entry");
                    if entry.file_type().expect("its type").is_dir() {
                        count(&entry.path())
                    } else {
                        1
                    }
                })
                .sum()
        })
    }
    count(&state.join("tree/tenants"))
}

/// With 10,000 observation events over 10 snapshots, opening the store (`state/tree/`) takes no
/// longer than with the same snapshots and none: the tree holds the same files. Each snapshot is
/// taken through the driver and records [`RECORDED`] observations through it; the rest of its
/// 1,000 are written into its tenant, and the view lists all 10,000. Prints both times;
/// `--nocapture` shows them.
#[test]
fn opening_the_tree_with_10000_observations_takes_no_longer_than_with_none() {
    let none = Case::new("open-none");
    let many = Case::new("open-10000");
    let started = Instant::now();
    let mut taken = Vec::new();
    for n in 0..SNAPSHOTS {
        let empty = none.take(&at(2 * n), &[repositories(0)]);
        assert_eq!(empty.state, SnapshotState::Complete, "{empty:?}");
        let full = many.take(&at(2 * n), &[repositories(RECORDED)]);
        assert_eq!(full.state, SnapshotState::Complete, "{full:?}");
        taken.push(full.snapshot_id.0.0);
    }
    plant(&many.state, &taken, PER_SNAPSHOT - RECORDED);
    let recorded = started.elapsed();
    assert_eq!(
        many.view("repositories").len(),
        SNAPSHOTS as usize * PER_SNAPSHOT,
        "10,000 observations are recorded and kept"
    );

    assert_eq!(
        tree_files(&many.state),
        tree_files(&none.state),
        "state/tree/ holds the same files with 10,000 observations as with none"
    );
    let (none_first, none_open) = open_times(&none.state);
    let (many_first, many_open) = open_times(&many.state);
    println!(
        "took {SNAPSHOTS} snapshots holding {} observations, {RECORDED} each recorded through \
         the driver, in {recorded:?}; opening state/tree/ with none: first {none_first:?}, \
         cheapest of {OPENS} {none_open:?}; with 10,000: first {many_first:?}, cheapest of \
         {OPENS} {many_open:?}",
        SNAPSHOTS as usize * PER_SNAPSHOT
    );
    assert!(
        many_open <= none_open * 2 + Duration::from_millis(10),
        "opening state/tree/ took {many_open:?} with 10,000 observations and {none_open:?} with \
         none"
    );
}

// ---------------------------------------------------------------------------------------------
// 3. Retention: the field, and which snapshots' observations it drops.
// ---------------------------------------------------------------------------------------------

/// `retention.snapshots` is 12 when the file leaves it out, `config show` writes it, a whole
/// number of 1 or more is taken, and anything else is refused by its YAML path.
#[test]
fn retention_is_12_when_absent_and_anything_but_a_whole_number_of_1_or_more_is_refused() {
    let case = Case::new("config");
    for (written, expected) in [
        ("", 12),
        ("    retention: {}\n", 12),
        ("    retention: {snapshots: 3}\n", 3),
        ("    retention: {snapshots: 1}\n", 1),
    ] {
        let file = case.config(written);
        let shown: Value = serde_json::from_str(&case.ok(&[
            "--config",
            file.to_str().expect("UTF-8"),
            "config",
            "show",
            "--format",
            "json",
        ]))
        .expect("config show prints JSON");
        assert_eq!(
            shown["instances"][0]["retention"],
            json!({"snapshots": expected}),
            "{written:?}: {shown:#}"
        );
    }
    for value in ["0", "-1", "2.5", "twelve"] {
        let file = case.config(&format!("    retention: {{snapshots: {value}}}\n"));
        let output = case.run(&[
            "--config",
            file.to_str().expect("UTF-8"),
            "config",
            "validate",
        ]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.code() == Some(1)
                && stderr.contains("instances[0].retention.snapshots: "),
            "snapshots: {value} is refused by its path: {}",
            describe(&output)
        );
    }
    let file = case.config("    retention: {snapshots: 3, days: 1}\n");
    let output = case.run(&[
        "--config",
        file.to_str().expect("UTF-8"),
        "config",
        "validate",
    ]);
    assert!(
        output.status.code() == Some(1)
            && String::from_utf8_lossy(&output.stderr).contains("instances[0].retention.days"),
        "an undeclared key is refused by its path: {}",
        describe(&output)
    );
}

/// `complete-snapshot` under a config that keeps 3: the newest 3 complete snapshots keep their
/// observations, the older complete ones and the failed ones that started before the oldest kept
/// lose theirs, and a failed one after it and one still collecting keep theirs. Every `Snapshot`
/// record stays.
#[test]
fn completing_a_snapshot_keeps_the_newest_complete_ones_and_drops_the_older_and_the_failed() {
    let case = Case::new("rule");
    let stale = "0199f000-0000-7000-8000-00000000c000";
    let last = "0199f000-0000-7000-8000-00000000c001";
    let flags = |snapshot: &'static str| {
        vec![
            "snapshot",
            "record-repository",
            "--snapshot-id",
            snapshot,
            "--repository",
            "alpha",
            "--visibility",
            "Public",
            "--archived",
            "false",
            "--local-checkout",
            "true",
            "--main-head",
            "0a1b2c3",
            "--main-committed-at",
            "2026-10-01T00:00:00Z",
            "--main-commits-7d",
            "1",
            "--real-commits-7d",
            "1",
            "--behind-main",
            "0",
            "--dirty-files",
            "0",
            "--worktrees",
            "1",
            "--open-issues",
            "0",
            "--in-catalog",
            "true",
        ]
    };

    // Oldest first: a snapshot left collecting, then complete and failed ones, two hours apart.
    store::open(&case.state).expect("create the store");
    left_collecting(&case.state, stale, &at(0));
    case.ok(&flags(stale));
    let mut ids = Vec::new();
    for (n, collector) in [
        (1, every_kind()),
        (2, fails_after_one()),
        (3, every_kind()),
        (4, every_kind()),
        (5, fails_after_one()),
        (6, every_kind()),
    ] {
        let taken = case.take(&at(2 * n), &[collector]);
        ids.push(taken.snapshot_id.0.0);
    }
    let [
        complete_1,
        failed_2,
        complete_3,
        complete_4,
        failed_5,
        complete_6,
    ] = <[String; 6]>::try_from(ids).expect("six snapshots");
    left_collecting(&case.state, last, &at(14));
    case.ok(&flags(last));
    assert_eq!(
        tenants(&case.state).len(),
        8,
        "every snapshot so far keeps its observations under a config that keeps 12"
    );

    let config = case.config("    retention: {snapshots: 3}\n");
    case.ok(&[
        "--config",
        config.to_str().expect("UTF-8"),
        "snapshot",
        "complete-snapshot",
        "--snapshot-id",
        last,
    ]);

    let kept: BTreeSet<String> = [
        stale.to_owned(),
        complete_4.clone(),
        failed_5.clone(),
        complete_6.clone(),
        last.to_owned(),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        tenants(&case.state),
        kept,
        "kept: the newest 3 complete, the failed one after the oldest of them, the one still \
         collecting; dropped: complete {complete_1} and {complete_3}, failed {failed_2}"
    );
    assert_eq!(case.snapshots_in("repositories"), kept);
    assert_eq!(
        case.snapshots_in("snapshots").len(),
        8,
        "every Snapshot record stays"
    );
}

/// A command that reads no observation never opens `state/observations/`: with its description
/// damaged, proposing a goal and listing the snapshots succeed, and a view of observations names
/// it.
#[test]
fn a_command_that_reads_no_observation_never_opens_the_observation_store() {
    let case = Case::new("lazy");
    let taken = case.take(&at(0), &[every_kind()]);
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
    fs::write(case.state.join("observations/store.json"), b"not json")
        .expect("damage the observation store's description");

    case.ok(&[
        "goal",
        "propose-goal",
        "--goal-id",
        "G1",
        "--title",
        "Unhindered",
        "--exit-evidence",
        "the observation store is never opened",
    ]);
    assert_eq!(case.view("snapshots").len(), 1);
    let output = case.run(&["snapshot", "repositories", "--format", "json"]);
    let observations = case.state.join("observations");
    assert!(
        output.status.code() == Some(1)
            && String::from_utf8_lossy(&output.stderr)
                .contains(observations.to_str().expect("UTF-8")),
        "a view of observations names the store it cannot open: {}",
        describe(&output)
    );
}
