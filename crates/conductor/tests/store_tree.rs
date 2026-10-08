//! `story:store-on-eventlog-tree`: the store on eventlog's tree provider at `state/tree/`,
//! `conductor store migrate`, the refusal of a state directory that still holds only an
//! eventlog-file store, and what an append costs as the tree grows. With
//! `story:observation-retention` the observations are migrated into `state/observations/`, one
//! tenant per snapshot, and a `state/tree/` that holds observations is split.
//!
//! `tests/fixtures/store/store/` is an eventlog-file store written by the build this story starts
//! from (48c36b4): three snapshots, one Complete per repository observed and one Failed, and every
//! entity the views read, moved through its commands. `tests/fixtures/store/views/` holds that
//! build's answer to every view over it, with `--format json`, but for one column:
//! `story:catalog-source` named the repositories view's `in_atlas_catalog` `in_catalog`, so
//! `snapshot-repositories.json` holds the same values under the new name, while the store's
//! records keep the old one, which this build still reads. `story:sessions-outside-root-explained`
//! added the sessions view's `role` column after `repository`: `snapshot-sessions.json` holds it
//! as `null` in every row, which is what this build reads from a record written without one.
//!
//! The organization in its repositories' URLs and working directories was then renamed
//! `example-org`, in the store and in the views. The store is chained by hash, so it was not
//! edited in place: every event was read through eventlog's File provider and appended, in feed
//! order, to a fresh store of the same format, with the new name in its body and its stream,
//! version, feed position, name, schema version, subject, actor, request and trace ids, causation
//! and `occurred_at` as they were; each command under its old key, with the request hash of its
//! body, which for the 79 of 85 bodies the rename did not touch is the hash their writer recorded.
//! The store's id, the event ids, the record instants and the chain's digests are new. No view
//! prints any of them, and the record instants keep feed order, which is all a fold reads them
//! for, so the views changed only where they print a renamed URL or working directory.
//!
//! A case copies the store into its own state directory under this test target's temporary
//! directory in `target/`, and runs the built binary there with `--state-dir`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use conductor_cli::store::{self, Store};
use conductor_model::behaviour::GoalStorage;
use conductor_model::direction::{GoalData, GoalId, GoalSnapshot, GoalState};
use eventlog_core::{
    CommandMeta, EventStore as _, Expected, MAX_READ_LIMIT, NewEvent, StreamId, TenantId,
    new_event_id, request_hash,
};
use eventlog_file::FileEventStore;
use serde_json::{Value, json};
use time::OffsetDateTime;

/// The fixture: the eventlog-file store and the views' answers over it.
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/store")
}

/// Every view the fixture holds the answer of, by its file under `views/`, with the arguments
/// before `--format json`.
const VIEWS: [(&str, &[&str]); 23] = [
    ("snapshot-snapshots", &["snapshot", "snapshots"]),
    ("snapshot-repositories", &["snapshot", "repositories"]),
    ("snapshot-pull-requests", &["snapshot", "pull-requests"]),
    ("snapshot-releases", &["snapshot", "releases"]),
    (
        "snapshot-merged-pull-requests",
        &["snapshot", "merged-pull-requests"],
    ),
    ("snapshot-workflow-runs", &["snapshot", "workflow-runs"]),
    ("snapshot-sessions", &["snapshot", "sessions"]),
    ("snapshot-blockers", &["snapshot", "blockers"]),
    ("snapshot-specifications", &["snapshot", "specifications"]),
    ("goal-goals", &["goal", "goals"]),
    ("goal-servings", &["goal", "servings"]),
    (
        "repository-repository-marks",
        &["repository", "repository-marks"],
    ),
    ("repository-activity", &["repository", "activity"]),
    (
        "repository-shipped",
        &["repository", "shipped", "--since", "2026-01-01T00:00:00Z"],
    ),
    ("decision-requests", &["decision", "requests"]),
    ("decision-hands-todo", &["decision", "hands-todo"]),
    ("decision-decisions", &["decision", "decisions"]),
    (
        "decision-show",
        &["decision", "show", "--decision-id", "DEC-20261007-02"],
    ),
    ("controller-controllers", &["controller", "controllers"]),
    ("dispatch-dispatches", &["dispatch", "dispatches"]),
    ("message-messages", &["message", "messages"]),
    (
        "resource-resource-requests",
        &["resource", "resource-requests"],
    ),
    ("guard-guard-decisions", &["guard", "guard-decisions"]),
];

/// One case: the directory the binary runs from, the state directory `--state-dir` names, and an
/// empty directory that is the binary's whole `PATH`, so no command reaches a real source.
struct Case {
    work: PathBuf,
    state: PathBuf,
    path: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("store_tree")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case's directory");
        }
        let work = root.join("work");
        let path = root.join("path");
        fs::create_dir_all(&work).expect("create the working directory");
        fs::create_dir_all(&path).expect("create the empty PATH");
        Self {
            work,
            state: root.join("state"),
            path,
        }
    }

    /// A case whose state directory holds a copy of the fixture's eventlog-file store at
    /// `state/store/`, and nothing else.
    fn with_file_store(name: &str) -> Self {
        let case = Self::new(name);
        copy_dir(&fixture().join("store"), &case.state.join("store"));
        case
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_conductor"));
        command
            .current_dir(&self.work)
            .env("PATH", &self.path)
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null());
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args)
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

    /// The one directory a migration kept the eventlog-file store in.
    fn kept(&self) -> PathBuf {
        let kept: Vec<PathBuf> = fs::read_dir(&self.state)
            .expect("read the state directory")
            .map(|entry| entry.expect("a directory entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("store.eventlog-file-"))
            })
            .collect();
        assert_eq!(kept.len(), 1, "one kept eventlog-file store: {kept:?}");
        kept.into_iter().next().expect("one")
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

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create the copy's directory");
    for entry in fs::read_dir(from).expect("read the directory to copy") {
        let entry = entry.expect("a directory entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("its type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy a file");
        }
    }
}

/// Every entry under `dir`, relative to it, with its length and modification time, sorted: what
/// "writes nothing" compares.
fn walk(dir: &Path) -> Vec<(String, u64, Option<SystemTime>)> {
    fn visit(base: &Path, dir: &Path, out: &mut Vec<(String, u64, Option<SystemTime>)>) {
        let Ok(read) = fs::read_dir(dir) else { return };
        for entry in read.flatten() {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).expect("an entry's metadata");
            out.push((
                path.strip_prefix(base)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
                metadata.len(),
                metadata.modified().ok(),
            ));
            if metadata.is_dir() {
                visit(base, &path, out);
            }
        }
    }
    let mut out = Vec::new();
    visit(dir, dir, &mut out);
    out.sort();
    out
}

fn json(text: &str, what: &str) -> Value {
    serde_json::from_str(text)
        .unwrap_or_else(|error| panic!("{what} is not JSON ({error}): {text}"))
}

/// The number of streams and events per stream type the eventlog-file store at `root` holds,
/// read through eventlog's File provider.
fn file_store_counts(root: &Path) -> BTreeMap<String, (usize, usize)> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    let log = runtime
        .block_on(FileEventStore::open_existing(root))
        .expect("the kept eventlog-file store opens");
    let tenant = TenantId::new("conductor").expect("the tenant");
    let mut streams: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut after = 0;
    loop {
        let page = runtime
            .block_on(log.read_feed(&tenant, after, MAX_READ_LIMIT))
            .expect("read the kept store's feed");
        let more = page.has_more && !page.events.is_empty();
        after = page.next_position;
        for event in page.events {
            *streams
                .entry((event.stream_type, event.stream_id))
                .or_default() += 1;
        }
        if !more {
            break;
        }
    }
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for ((stream_type, _), events) in streams {
        let entry = counts.entry(stream_type).or_default();
        entry.0 += 1;
        entry.1 += events;
    }
    counts
}

// ---------------------------------------------------------------------------------------------
// 1. A migrated store answers every view as the eventlog-file store did.
// ---------------------------------------------------------------------------------------------

/// The fixture covers every view of the `cli:` block that reads the store, and every one answers
/// rows; `snapshot boards` reads `STATUS.md`, not the store.
#[test]
fn the_fixture_holds_an_answer_with_rows_for_every_view_that_reads_the_store() {
    let output = Command::new("ess")
        .args(["specify", "compile", "--path"])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec"))
        .args(["--format", "json"])
        .output()
        .expect("`ess` is on PATH");
    assert!(output.status.success(), "{}", describe(&output));
    let model: Value = serde_json::from_slice(&output.stdout).expect("`ess` prints JSON");
    let component = model["components"]
        .as_object()
        .expect("components")
        .values()
        .find(|component| !component["cli"].is_null())
        .expect("a component declares a cli block");
    let mut placed = Vec::new();
    for group in component["cli"]["groups"].as_array().expect("cli groups") {
        let name = group["name"].as_str().expect("a group's name");
        for view in group["views"].as_array().expect("a group's views") {
            let wire = model["views"][view.as_str().expect("a view's name")]["naming"]["wire"]
                .as_str()
                .expect("a view's wire name");
            if (name, wire) != ("snapshot", "boards") {
                placed.push(format!("{name}-{wire}"));
            }
        }
    }
    for view in &placed {
        assert!(
            VIEWS.iter().any(|(file, _)| file == view),
            "the fixture holds no answer of the view {view}"
        );
    }
    for (file, _) in VIEWS {
        let answer = json(
            &fs::read_to_string(fixture().join("views").join(format!("{file}.json")))
                .expect("read the fixture's answer"),
            file,
        );
        let rows = match &answer {
            Value::Array(rows) => rows.len(),
            Value::Object(_) => 1,
            _ => 0,
        };
        assert!(
            rows > 0,
            "the fixture's answer of {file} has no rows: {answer}"
        );
    }
}

/// The views the fixture holds that list observations.
fn observation_views() -> impl Iterator<Item = &'static str> {
    VIEWS
        .iter()
        .map(|(file, _)| *file)
        .filter(|file| file.starts_with("snapshot-") && *file != "snapshot-snapshots")
}

/// The snapshots the fixture's observations name: the tenants a migration keeps them in.
fn observed_snapshots() -> BTreeSet<String> {
    let mut snapshots = BTreeSet::new();
    for file in observation_views() {
        let rows = json(
            &fs::read_to_string(fixture().join("views").join(format!("{file}.json")))
                .expect("read the fixture's answer"),
            file,
        );
        for row in rows.as_array().expect("a view's rows") {
            snapshots.insert(
                row["snapshot_id"]
                    .as_str()
                    .expect("an observation names its snapshot")
                    .to_owned(),
            );
        }
    }
    assert_eq!(snapshots.len(), 2, "the fixture observes in two snapshots");
    snapshots
}

/// The names of the entries directly under `dir`; none where it is not there.
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

/// The observation stream types conductor's tenant keeps in the tree at `tree`, by their
/// directories.
fn observation_streams(tree: &Path) -> BTreeSet<String> {
    names(&tree.join("tenants/conductor/streams"))
        .into_iter()
        .filter(|stream_type| {
            stream_type.starts_with("conductor.observation.")
                && stream_type.ends_with("Observation")
        })
        .collect()
}

/// Every view answers what the fixture holds; the ones that do not, named.
fn differing_views(case: &Case) -> Vec<String> {
    let mut differing = Vec::new();
    for (file, args) in VIEWS {
        let mut args = args.to_vec();
        args.extend_from_slice(&["--format", "json"]);
        let answered = json(&case.ok(&args), file);
        let source = json(
            &fs::read_to_string(fixture().join("views").join(format!("{file}.json")))
                .expect("read the fixture's answer"),
            file,
        );
        if answered != source {
            differing.push(format!(
                "{file}:\n  source: {source}\n  migrated: {answered}"
            ));
        }
    }
    differing
}

/// `conductor store migrate` over the fixture: every view answers row for row what the build
/// before it answered over the eventlog-file store, observations included; the tree is at
/// `state/tree/` and holds no observation, each snapshot's observations are in its tenant of
/// `state/observations/`, the old store is kept beside them, and nothing of the run is left.
#[test]
fn a_migrated_fixture_store_answers_every_view_as_the_source_store_did() {
    let case = Case::with_file_store("every-view");
    let printed = case.ok(&["store", "migrate"]);

    assert!(
        case.state.join("tree/store.json").is_file(),
        "the tree store is at state/tree/"
    );
    let kept = case.kept();
    assert!(
        kept.join("manifest.json").is_file(),
        "the eventlog-file store is kept whole at {}",
        kept.display()
    );
    assert_eq!(
        names(&case.state),
        [
            kept.file_name()
                .expect("a name")
                .to_string_lossy()
                .into_owned(),
            "observations".to_owned(),
            "tree".to_owned()
        ]
        .into_iter()
        .collect(),
        "only the kept store, the observations and the tree are left in the state directory"
    );
    assert_eq!(
        observation_streams(&case.state.join("tree")),
        BTreeSet::new(),
        "state/tree/ keeps no observation"
    );
    assert_eq!(
        names(&case.state.join("observations/tenants")),
        observed_snapshots(),
        "one tenant per snapshot that observed, named after it"
    );

    let counts = file_store_counts(&kept);
    let (streams, events) = counts.values().fold((0, 0), |(streams, events), (s, e)| {
        (streams + s, events + e)
    });
    let mut expected: Vec<String> = counts
        .iter()
        .map(|(stream_type, (s, e))| format!("{stream_type}: {s} streams, {e} events"))
        .collect();
    expected.push(format!(
        "moved {events} events in {streams} streams from {} to {} and {}; the eventlog-file \
         store is kept at {}",
        case.state.join("store").display(),
        case.state.join("tree").display(),
        case.state.join("observations").display(),
        kept.display()
    ));
    assert_eq!(
        printed.lines().collect::<Vec<_>>(),
        expected,
        "the counts it moved"
    );

    let differing = differing_views(&case);
    assert!(
        differing.is_empty(),
        "{} of {} views answer otherwise after the migration:\n{}",
        differing.len(),
        VIEWS.len(),
        differing.join("\n")
    );
}

/// `--format json` prints the same counts as one object, which match what the kept store holds.
#[test]
fn the_migration_prints_the_counts_it_moved_as_json() {
    let case = Case::with_file_store("json-counts");
    let printed = json(
        &case.ok(&["store", "migrate", "--format", "json"]),
        "the counts",
    );
    let kept = case.kept();
    let counts = file_store_counts(&kept);
    let types: BTreeMap<String, (usize, usize)> = printed["types"]
        .as_array()
        .expect("types")
        .iter()
        .map(|row| {
            (
                row["stream_type"].as_str().expect("a type").to_owned(),
                (
                    usize::try_from(row["streams"].as_u64().expect("streams")).expect("fits"),
                    usize::try_from(row["events"].as_u64().expect("events")).expect("fits"),
                ),
            )
        })
        .collect();
    assert_eq!(types, counts);
    let events: usize = counts.values().map(|(_, events)| events).sum();
    let streams: usize = counts.values().map(|(streams, _)| streams).sum();
    assert_eq!(printed["events"].as_u64(), Some(events as u64));
    assert_eq!(printed["streams"].as_u64(), Some(streams as u64));
    assert_eq!(
        printed["tree"].as_str(),
        Some(case.state.join("tree").to_str().expect("UTF-8"))
    );
    assert_eq!(
        printed["observations"].as_str(),
        Some(case.state.join("observations").to_str().expect("UTF-8"))
    );
    assert_eq!(
        printed["source"].as_str(),
        Some(case.state.join("store").to_str().expect("UTF-8"))
    );
    assert_eq!(printed["kind"].as_str(), Some("eventlog-file"));
    assert_eq!(
        printed["kept"].as_str(),
        Some(kept.to_str().expect("UTF-8"))
    );
}

/// A goal whose id the specification admits and whose stream id, as the File provider kept it,
/// is too long a name for the tree's directory (170 `%`, kept as `%25` 170 times): it migrates,
/// reads back, and moves on.
#[test]
fn a_migrated_goal_whose_stream_the_tree_cannot_name_reads_back_and_moves() {
    let case = Case::new("long-id");
    let goal = "%".repeat(170);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let log = FileEventStore::open(case.state.join("store"))
            .await
            .expect("create an eventlog-file store");
        let tenant = TenantId::new("conductor").expect("the tenant");
        let stream = StreamId::new(tenant, "conductor.direction.Goal", "%25".repeat(170))
            .expect("the stream the File provider kept the goal in");
        let events = [NewEvent::new(
            "stored",
            1,
            json!({
                "state": "Draft",
                "goal_id": goal,
                "title": "A long id",
                "exit_evidence": "it moves",
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
            .expect("append the goal");
    });

    case.ok(&["store", "migrate"]);
    let goals = json(
        &case.ok(&["goal", "goals", "--format", "json"]),
        "the goals",
    );
    assert_eq!(goals[0]["goal_id"].as_str(), Some(goal.as_str()), "{goals}");
    assert_eq!(goals[0]["state"], "Draft", "{goals}");
    case.ok(&["goal", "confirm-goal", "--goal-id", &goal]);
    let goals = json(
        &case.ok(&["goal", "goals", "--format", "json"]),
        "the goals",
    );
    assert_eq!(goals.as_array().map(Vec::len), Some(1), "{goals}");
    assert_eq!(goals[0]["state"], "Confirmed", "{goals}");
}

// ---------------------------------------------------------------------------------------------
// 2. What the migration refuses, waits for and finishes.
// ---------------------------------------------------------------------------------------------

/// A refusal: exit 1, nothing on standard output, one line on standard error holding `names`.
fn assert_refused(output: &Output, names: &str, what: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(1)
            && output.stdout.is_empty()
            && stderr.lines().count() == 1
            && stderr.contains(names),
        "{what} is refused naming {names:?}: {}",
        describe(output)
    );
}

/// A second migration, and one where `state/tree/` is an empty directory, refuse and change
/// nothing.
#[test]
fn a_migration_refuses_where_the_tree_exists_and_changes_nothing() {
    let case = Case::with_file_store("again");
    case.ok(&["store", "migrate"]);
    let before = walk(&case.state);
    let again = case.run(&["store", "migrate"]);
    assert_refused(&again, "exists", "a second migration");
    assert_eq!(
        walk(&case.state),
        before,
        "a second migration changed files"
    );

    let empty = Case::with_file_store("empty-tree");
    fs::create_dir_all(empty.state.join("tree")).expect("create an empty tree directory");
    let before = walk(&empty.state);
    let output = empty.run(&["store", "migrate"]);
    assert_refused(&output, "exists", "a migration onto an empty tree/");
    assert_eq!(walk(&empty.state), before, "it changed files");
}

/// Without an eventlog-file store there is nothing to migrate, and nothing is created.
#[test]
fn a_migration_without_an_eventlog_file_store_refuses_and_creates_nothing() {
    let case = Case::new("nothing");
    let output = case.run(&["store", "migrate"]);
    assert_refused(&output, "no eventlog-file store", "a migration of nothing");
    assert!(!case.state.exists(), "it created {}", case.state.display());

    let output = case.run(&["store", "migrate", "--format", "yaml"]);
    assert_refused(&output, "yaml", "an unknown format");
}

/// While another holder has the eventlog-file store's writer lock, the migration waits and writes
/// no tree; once the lock is released, it migrates.
#[test]
fn a_migration_waits_for_the_eventlog_file_stores_writer_lock() {
    let case = Case::with_file_store("held-lock");
    let lock = File::options()
        .read(true)
        .write(true)
        .open(case.state.join("store/writer.lock"))
        .expect("open the eventlog-file store's writer lock");
    lock.lock().expect("hold the writer lock");

    let mut child = case
        .command(&["store", "migrate"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start the migration");
    thread::sleep(Duration::from_millis(1500));
    let waiting = child.try_wait().expect("ask after the migration");
    let planted: Vec<bool> = ["tree", "tree.next", "store.migrating"]
        .iter()
        .map(|name| case.state.join(name).exists())
        .collect();
    lock.unlock().expect("release the writer lock");
    let output = child.wait_with_output().expect("the migration ends");

    assert!(
        waiting.is_none(),
        "the migration ended while the lock was held: {waiting:?}"
    );
    assert_eq!(
        planted,
        [false, false, false],
        "it wrote tree, tree.next or store.migrating while the lock was held"
    );
    assert!(output.status.success(), "{}", describe(&output));
    assert!(case.state.join("tree/store.json").is_file());
}

/// A migration that stopped before its first rename leaves `tree.next/` and `store.migrating/`;
/// the next run removes them and migrates.
#[test]
fn a_migration_finishes_what_an_interrupted_one_left() {
    let case = Case::with_file_store("interrupted");
    for left in ["tree.next/tenants", "store.migrating"] {
        fs::create_dir_all(case.state.join(left)).expect("leave a directory behind");
    }
    fs::write(case.state.join("tree.next/store.json"), b"{}").expect("leave a file behind");
    case.ok(&["store", "migrate"]);
    assert!(!case.state.join("tree.next").exists());
    assert!(!case.state.join("store.migrating").exists());
    let goals = json(
        &case.ok(&["goal", "goals", "--format", "json"]),
        "the goals",
    );
    let source = json(
        &fs::read_to_string(fixture().join("views/goal-goals.json")).expect("read"),
        "the fixture's goals",
    );
    assert_eq!(goals, source);
}

// ---------------------------------------------------------------------------------------------
// 2b. A tree that holds observations, as the build before `story:observation-retention` migrated
//     it, is split.
// ---------------------------------------------------------------------------------------------

/// A case whose `state/tree/` holds every event of the fixture's eventlog-file store, the
/// observations included, in conductor's tenant, as `store migrate` of the build before
/// `story:observation-retention` wrote it; and no `state/store/`.
fn with_observations_in_the_tree(name: &str) -> Case {
    let case = Case::new(name);
    let source = case.state.join("source");
    copy_dir(&fixture().join("store"), &source);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    runtime.block_on(async {
        let file = FileEventStore::open_existing(&source)
            .await
            .expect("open the fixture's eventlog-file store");
        let tenant = TenantId::new("conductor").expect("the tenant");
        let mut events = Vec::new();
        let mut after = 0;
        loop {
            let page = file
                .read_feed(&tenant, after, MAX_READ_LIMIT)
                .await
                .expect("read the fixture's feed");
            let more = page.has_more && !page.events.is_empty();
            after = page.next_position;
            events.extend(page.events);
            if !more {
                break;
            }
        }
        drop(file);
        let tree = eventlog_tree::TreeEventStore::open(case.state.join("tree"))
            .await
            .expect("create the tree");
        for event in events {
            let stream = StreamId::new(tenant.clone(), event.stream_type, event.stream_id)
                .expect("the event's stream");
            let expected = match event.version {
                1 => Expected::NoStream,
                version => Expected::Exact(version - 1),
            };
            let appended =
                [NewEvent::new(event.name, event.schema_version, event.data).expect("an event")];
            let id = new_event_id();
            let meta = CommandMeta {
                idempotency_key: id.clone(),
                request_hash: request_hash(&appended).expect("a hash"),
                subject: event.subject,
                actor: event.actor,
                request_id: id.clone(),
                trace_id: id,
                causation_id: None,
                causation_depth: 0,
                occurred_at: event.occurred_at,
                claim: None,
            };
            tree.append(&stream, expected, &appended, &meta)
                .await
                .expect("append to the tree");
        }
    });
    fs::remove_dir_all(&source).expect("remove the copy");
    assert!(
        !observation_streams(&case.state.join("tree")).is_empty(),
        "the tree holds observations"
    );
    case
}

/// The one directory a split kept the tree it split in.
fn split_kept(case: &Case) -> PathBuf {
    let kept: Vec<PathBuf> = names(&case.state)
        .into_iter()
        .filter(|name| name.starts_with("tree.split-"))
        .map(|name| case.state.join(name))
        .collect();
    assert_eq!(kept.len(), 1, "one kept tree: {kept:?}");
    kept.into_iter().next().expect("one")
}

/// `store migrate` splits a `tree/` that holds observations: every view answers as the fixture
/// says, `tree/` holds no observation, each snapshot's observations are in its tenant of
/// `observations/`, the tree it split is kept whole, and a second run refuses and changes nothing.
#[test]
fn a_tree_that_holds_observations_is_split_and_answers_every_view_as_before() {
    let case = with_observations_in_the_tree("split");
    let printed = case.ok(&["store", "migrate"]);

    let kept = split_kept(&case);
    assert_eq!(
        printed.lines().last(),
        Some(
            format!(
                "moved {} events in {} streams from {} to {} and {}; the tree before the split \
                 is kept at {}",
                file_store_counts(&fixture().join("store"))
                    .values()
                    .map(|(_, events)| events)
                    .sum::<usize>(),
                file_store_counts(&fixture().join("store"))
                    .values()
                    .map(|(streams, _)| streams)
                    .sum::<usize>(),
                case.state.join("tree").display(),
                case.state.join("tree").display(),
                case.state.join("observations").display(),
                kept.display()
            )
            .as_str()
        ),
        "{printed}"
    );
    assert_eq!(
        observation_streams(&case.state.join("tree")),
        BTreeSet::new(),
        "state/tree/ keeps no observation after the split"
    );
    assert!(
        !observation_streams(&kept).is_empty(),
        "the tree it split is kept whole at {}",
        kept.display()
    );
    assert_eq!(
        names(&case.state.join("observations/tenants")),
        observed_snapshots()
    );
    assert!(!case.state.join("tree.next").exists());
    let differing = differing_views(&case);
    assert!(
        differing.is_empty(),
        "{} of {} views answer otherwise after the split:\n{}",
        differing.len(),
        VIEWS.len(),
        differing.join("\n")
    );

    let before = walk(&case.state);
    let again = case.run(&["store", "migrate"]);
    assert_refused(&again, "exists", "a second migration after a split");
    assert_eq!(walk(&case.state), before, "it changed files");
}

/// A split that stopped between its two renames leaves `tree.next/` and no `tree/`: every command
/// and view refuses, naming `conductor store migrate`, and writes nothing; the next
/// `store migrate` renames `tree.next/` `tree/`, and every view answers as the fixture says.
#[test]
fn a_split_stopped_between_its_renames_is_refused_and_then_finished() {
    let case = with_observations_in_the_tree("split-stopped");
    case.ok(&["store", "migrate"]);
    let kept = split_kept(&case);
    fs::rename(case.state.join("tree"), case.state.join("tree.next"))
        .expect("leave the split between its renames");

    let before = walk(&case.state);
    for args in [
        &[
            "goal",
            "propose-goal",
            "--goal-id",
            "G9",
            "--title",
            "t",
            "--exit-evidence",
            "e",
        ][..],
        &["goal", "goals", "--format", "json"][..],
        &["snapshot", "repositories", "--format", "json"][..],
    ] {
        let output = case.run(args);
        assert_refused(
            &output,
            "run `conductor store migrate`",
            &format!("`conductor {}`", args.join(" ")),
        );
        assert_eq!(walk(&case.state), before, "`{}` wrote", args.join(" "));
    }

    let printed = case.ok(&["store", "migrate"]);
    assert_eq!(
        printed,
        format!(
            "finished the split a run left between its renames: renamed {} to {}; the tree \
             before the split is kept at {}\n",
            case.state.join("tree.next").display(),
            case.state.join("tree").display(),
            kept.display()
        )
    );
    assert!(case.state.join("tree/store.json").is_file());
    assert!(!case.state.join("tree.next").exists());
    let differing = differing_views(&case);
    assert!(differing.is_empty(), "{}", differing.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// 3. Every other command refuses an unmigrated store, and a new state directory starts a tree.
// ---------------------------------------------------------------------------------------------

/// With `state/store/manifest.json` and no `state/tree/`, every command and view exits 1 with one
/// line that names `conductor store migrate`, and writes nothing.
#[test]
fn every_command_refuses_an_unmigrated_store_and_writes_nothing() {
    let case = Case::with_file_store("unmigrated");
    let mut runs: Vec<Vec<&str>> = VIEWS
        .iter()
        .map(|(_, args)| {
            let mut args = args.to_vec();
            args.extend_from_slice(&["--format", "json"]);
            args
        })
        .collect();
    let commands: [&[&str]; 9] = [
        &[
            "goal",
            "propose-goal",
            "--goal-id",
            "G9",
            "--title",
            "t",
            "--exit-evidence",
            "e",
        ],
        &[
            "repository",
            "mark-repository",
            "--repository",
            "alpha",
            "--activity",
            "Active",
            "--marked-by",
            "Operator",
            "--reason",
            "r",
        ],
        &[
            "decision",
            "raise-decision-request",
            "--request-id",
            "alpha-9",
            "--repository",
            "alpha",
            "--question",
            "q?",
            "--options",
            "A: a; B: b",
            "--recommendation",
            "A",
            "--raised-at",
            "2026-10-07T09:00:00Z",
        ],
        &[
            "controller",
            "start-controller",
            "--repository",
            "delta",
            "--harness",
            "Claude",
        ],
        &[
            "dispatch",
            "send-dispatch",
            "--dispatch-id",
            "DSP-20261007-09",
            "--repository",
            "alpha",
            "--goal-id",
            "G1",
            "--brief",
            "b",
            "--sent-at",
            "2026-10-07T11:00:00Z",
            "--priority",
            "1",
        ],
        &[
            "message",
            "receive-message",
            "--sender",
            "alpha",
            "--recipient",
            "conductor",
            "--first-line",
            "hello",
            "--transport",
            "SendMessage",
            "--received-at",
            "2026-10-07T12:00:00Z",
        ],
        &[
            "resource",
            "request-resource",
            "--repository",
            "alpha",
            "--resource",
            "Usage",
            "--amount",
            "1",
            "--requested-at",
            "2026-10-07T13:00:00Z",
        ],
        &[
            "snapshot",
            "start-snapshot",
            "--started-at",
            "2026-10-07T00:00:00Z",
            "--disk-free-bytes",
            "1",
        ],
        &["goal", "confirm-goal", "--goal-id", "G1"],
    ];
    runs.extend(commands.iter().map(|args| args.to_vec()));

    let before = walk(&case.state);
    let mut problems = Vec::new();
    for args in &runs {
        let output = case.run(args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.code() != Some(1)
            || !output.stdout.is_empty()
            || stderr.lines().count() != 1
            || !stderr.contains("run `conductor store migrate`")
        {
            problems.push(format!(
                "`conductor {}`: {}",
                args.join(" "),
                describe(&output)
            ));
        }
        if walk(&case.state) != before {
            problems.push(format!(
                "`conductor {}` wrote into the state directory",
                args.join(" ")
            ));
            break;
        }
    }
    assert!(
        problems.is_empty(),
        "{} of {} runs:\n{}",
        problems.len(),
        runs.len(),
        problems.join("\n")
    );

    // The guard's hook answers a denial whether or not it is recorded: exit 2, the reason, and a
    // second line saying it was not recorded, naming the migration.
    let home = case.work.join("home");
    fs::create_dir_all(home.join("example-org/alpha")).expect("create the hook's home");
    let payload = serde_json::json!({
        "session_id": "s",
        "cwd": home.join("example-org/alpha").display().to_string(),
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "git -C ../conductor status"},
    });
    let mut hook = case
        .command(&["guard", "record-guard-decision", "--from-pre-tool-use"])
        .env("HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start the hook");
    hook.stdin
        .take()
        .expect("the hook's standard input")
        .write_all(&serde_json::to_vec(&payload).expect("the payload"))
        .expect("write the payload");
    let output = hook.wait_with_output().expect("the hook ends");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() == Some(2)
            && stderr.lines().count() == 2
            && stderr.contains("the verdict was not recorded")
            && stderr.contains("run `conductor store migrate`"),
        "the hook denies and says the verdict was not recorded: {}",
        describe(&output)
    );
    assert_eq!(
        walk(&case.state),
        before,
        "the hook wrote into the state directory"
    );
}

/// A state directory with neither store: a command starts a tree store at `state/tree/`, and a
/// view reads it.
#[test]
fn a_state_directory_with_neither_store_starts_a_tree_store() {
    let case = Case::new("neither");
    case.ok(&[
        "goal",
        "propose-goal",
        "--goal-id",
        "G1",
        "--title",
        "On the tree",
        "--exit-evidence",
        "a view lists it",
    ]);
    assert!(case.state.join("tree/store.json").is_file());
    assert!(!case.state.join("store").exists());
    let goals = json(
        &case.ok(&["goal", "goals", "--format", "json"]),
        "the goals",
    );
    assert_eq!(goals[0]["goal_id"], "G1", "{goals}");
    assert_eq!(goals.as_array().map(Vec::len), Some(1), "{goals}");
}

// ---------------------------------------------------------------------------------------------
// 4. What an append costs as the tree grows.
// ---------------------------------------------------------------------------------------------

/// Puts one Goal snapshot per id in `ids` through the storage port: one event each.
fn put_goals(store: &mut Store, ids: impl Iterator<Item = usize>) {
    for n in ids {
        GoalStorage::put(
            store,
            GoalSnapshot {
                state: GoalState::Draft,
                data: GoalData {
                    goal_id: GoalId(format!("G{n:05}")),
                    title: format!("Goal {n}"),
                    exit_evidence: "measured".to_owned(),
                },
            },
        );
    }
    store.check().expect("the store kept every goal");
}

/// Batches of 100 appends measured at each size. Each batch fsyncs every file it writes, so its
/// time also carries whatever else the machine's disk is doing; the cheapest of three is the cost
/// of the appends themselves.
const BATCHES: usize = 3;

/// With `held` events stored under `state`: the time to open the store, the cheapest of
/// [`BATCHES`] batches of 100 appends through it, and the time to read every goal back.
fn measure(state: &Path, held: usize) -> (Duration, Duration, Duration) {
    let started = Instant::now();
    let mut store = store::open(state).expect("open the store");
    let opened = started.elapsed();

    let mut batches = Vec::new();
    for batch in 0..BATCHES {
        let first = held + batch * 100;
        let started = Instant::now();
        put_goals(&mut store, first..first + 100);
        batches.push(started.elapsed());
    }

    let started = Instant::now();
    let listed = GoalStorage::list(&store).len();
    let read = started.elapsed();
    store.check().expect("the full read succeeds");
    assert_eq!(
        listed,
        held + BATCHES * 100,
        "the full read answers every goal"
    );
    let appended = *batches.iter().min().expect("a batch");
    println!(
        "{held} events: open {opened:?}, 100 appends {appended:?} (batches {batches:?}), full \
         read of {listed} goals {read:?}"
    );
    (opened, appended, read)
}

/// With 5,000 events stored, 100 appends cost less than 3 times what 100 appends cost at 500
/// events, each the cheapest of [`BATCHES`] batches. Prints the time to open the store, to append
/// 100 events and to read every goal at both sizes; `--nocapture` shows them.
///
/// A wall-clock ratio: on a host under load the two sizes are measured under different load and
/// the ratio fails while the code is unchanged (`story:append-cost-test-under-load`). It runs
/// alone, with `task measure`.
#[test]
#[ignore = "a measurement: wall-clock ratio, run alone with `task measure`"]
fn appends_at_5000_events_cost_less_than_three_times_appends_at_500() {
    let case = Case::new("scaling");
    let mut store = store::open(&case.state).expect("create the store");
    put_goals(&mut store, 0..500);
    drop(store);
    let (_, at_500, _) = measure(&case.state, 500);

    let mut store = store::open(&case.state).expect("reopen the store");
    put_goals(&mut store, 500 + BATCHES * 100..5_000);
    drop(store);
    let (_, at_5000, _) = measure(&case.state, 5_000);

    assert!(
        at_5000 < at_500 * 3,
        "100 appends took {at_5000:?} at 5,000 events and {at_500:?} at 500"
    );
}
