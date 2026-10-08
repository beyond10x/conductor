//! Adversary cases for `story:local-store`, wave 03, pass 1: the store under reopen, two handles,
//! racing threads, a failed write, damaged history, the identities and text the specification
//! admits, one record the store cannot read, and a caller already inside a tokio runtime.
//!
//! Each case keeps its state directory under this test target's own temporary directory in
//! `target/`.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use conductor_cli::store::{self, Store, StoreError};
use conductor_model::behaviour::{Generated, GoalStorage};
use conductor_model::direction::obligations::{
    ConfirmGoalBehavior, DropGoalBehavior, GoalsQuery, MarkGoalMetBehavior, ProposeGoalBehavior,
};
use conductor_model::direction::{
    ConfirmGoal, ConfirmGoalOutcome, DropGoal, DropGoalOutcome, GoalId, GoalState, MarkGoalMet,
    MarkGoalMetOutcome, ProposeGoal, ProposeGoalOutcome,
};
use eventlog_core::{
    CommandMeta, EventStore as _, Expected, NewEvent, StreamId, TenantId, new_event_id,
    request_hash,
};
use eventlog_tree::TreeEventStore;
use serde_json::json;
use time::OffsetDateTime;

/// One `Goals` row, as `(goal_id, state, title)`.
type Row = (String, GoalState, String);

/// An empty state directory for one case.
fn state_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_store")
        .join(case)
        .join("state");
    if dir.exists() {
        let store = dir.join("tree");
        if store.exists() {
            set_mode(&store, 0o755);
        }
        fs::remove_dir_all(&dir).expect("clear the case's state directory");
    }
    fs::create_dir_all(&dir).expect("create the case's state directory");
    dir
}

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("set the directory's mode");
}

fn id(text: &str) -> GoalId {
    GoalId(text.to_owned())
}

fn open(state: &Path) -> Generated<Store> {
    Generated::new(store::open(state).expect("open the store"))
}

fn propose_with(
    generated: &mut Generated<Store>,
    goal: &str,
    title: &str,
    exit_evidence: &str,
) -> ProposeGoalOutcome {
    generated
        .propose_goal(ProposeGoal {
            goal_id: id(goal),
            title: title.to_owned(),
            exit_evidence: exit_evidence.to_owned(),
        })
        .expect("ProposeGoal decides an outcome")
}

fn propose(generated: &mut Generated<Store>, goal: &str, title: &str) -> ProposeGoalOutcome {
    propose_with(generated, goal, title, &format!("evidence for {goal}"))
}

fn rows(generated: &Generated<Store>) -> Vec<Row> {
    generated
        .goals()
        .expect("Goals answers")
        .into_iter()
        .map(|row| (row.goal_id.0, row.state, row.title))
        .collect()
}

fn row(goal: &str, state: GoalState, title: &str) -> Row {
    (goal.to_owned(), state, title.to_owned())
}

/// A runtime for a caller that is itself inside tokio: an async handler, a `#[tokio::main]` binary.
fn callers_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("the caller's runtime")
}

/// A sync caller that is itself inside a tokio runtime reaches the store's own runtime, and gets
/// an answer, not a panic (`story:store-hardening`): a handle opened outside answers the Goals
/// view inside, and a handle opened, written, read and dropped inside keeps its proposal.
#[test]
fn adv_the_store_answers_a_caller_inside_a_tokio_runtime() {
    let state = state_dir("inside-runtime");
    let mut generated = open(&state);
    propose(&mut generated, "G1", "Inside a runtime");
    generated.ports.check().expect("the proposal is kept");

    let caller = callers_runtime();
    let goals = caller.block_on(async { rows(&generated) });
    assert_eq!(
        goals,
        vec![row("G1", GoalState::Draft, "Inside a runtime")],
        "a handle opened outside answers inside the caller's runtime"
    );
    generated
        .ports
        .check()
        .expect("the read inside the caller's runtime succeeds");
    drop(generated);

    let inside = caller.block_on(async {
        let mut inside = open(&state);
        let outcome = propose(&mut inside, "G2", "Opened inside a runtime");
        inside
            .ports
            .check()
            .expect("the proposal made inside is kept");
        let listed = rows(&inside);
        drop(inside);
        (outcome, listed)
    });
    let expected = vec![
        row("G1", GoalState::Draft, "Inside a runtime"),
        row("G2", GoalState::Draft, "Opened inside a runtime"),
    ];
    assert!(
        matches!(inside.0, ProposeGoalOutcome::Proposed { .. }),
        "{inside:?}"
    );
    assert_eq!(inside.1, expected, "a handle opened inside reads its write");
    assert_eq!(
        rows(&open(&state)),
        expected,
        "the write made inside is kept"
    );
}

/// A handle dropped on a thread inside another tokio runtime shuts its own runtime down without
/// a panic (`story:store-hardening`).
#[test]
fn adv_a_store_dropped_inside_a_tokio_runtime_does_not_panic() {
    let state = state_dir("dropped-inside-runtime");
    let mut generated = open(&state);
    propose(&mut generated, "G1", "Dropped inside a runtime");
    generated.ports.check().expect("the proposal is kept");

    callers_runtime().block_on(async move { drop(generated) });
    assert_eq!(
        rows(&open(&state)),
        vec![row("G1", GoalState::Draft, "Dropped inside a runtime")]
    );
}

/// An open the log refuses, on a thread inside another tokio runtime, is an error and not a panic
/// (`story:store-hardening`): the store's runtime is dropped on the error path too.
#[test]
fn adv_an_open_refused_inside_a_tokio_runtime_is_an_error() {
    let state = state_dir("refused-inside-runtime");
    fs::create_dir_all(state.join("tree")).expect("an empty store directory");

    let refused = callers_runtime().block_on(async { store::open_existing(&state).map(drop) });
    assert!(
        refused.is_err(),
        "an empty store directory has no log to open: {refused:?}"
    );

    // A tree whose description does not read is refused by eventlog itself, after the store's
    // runtime has started, so that runtime is dropped inside the caller's.
    fs::write(state.join("tree/store.json"), b"not json").expect("damage the description");
    for (what, refused) in [
        (
            "open_existing",
            callers_runtime().block_on(async { store::open_existing(&state).map(drop) }),
        ),
        (
            "open",
            callers_runtime().block_on(async { store::open(&state).map(drop) }),
        ),
    ] {
        assert!(
            refused.is_err(),
            "{what}: a tree whose description does not read is refused: {refused:?}"
        );
    }
}

/// Each read takes a goal's newest snapshot: a goal written several times reopens in the state of
/// its last write, and all four lifecycle states survive the JSON mapping.
#[test]
fn adv_a_reopened_store_answers_each_goals_latest_state() {
    let state = state_dir("latest");
    let mut generated = open(&state);
    for (goal, title) in [
        ("G1", "Met goal"),
        ("G2", "Dropped goal"),
        ("G3", "Confirmed goal"),
        ("G4", "Draft goal"),
    ] {
        assert!(matches!(
            propose(&mut generated, goal, title),
            ProposeGoalOutcome::Proposed { .. }
        ));
    }
    for goal in ["G1", "G3"] {
        let outcome = generated
            .confirm_goal(ConfirmGoal { goal_id: id(goal) })
            .expect("ConfirmGoal decides an outcome");
        assert!(matches!(outcome, ConfirmGoalOutcome::Confirmed { .. }));
    }
    let met = generated
        .mark_goal_met(MarkGoalMet {
            goal_id: id("G1"),
            evidence: "it holds".to_owned(),
        })
        .expect("MarkGoalMet decides an outcome");
    assert!(matches!(met, MarkGoalMetOutcome::Met { .. }));
    let dropped = generated
        .drop_goal(DropGoal {
            goal_id: id("G2"),
            reason: "not needed".to_owned(),
        })
        .expect("DropGoal decides an outcome");
    assert!(matches!(dropped, DropGoalOutcome::Dropped { .. }));
    generated.ports.check().expect("every write is kept");
    drop(generated);

    let mut reopened = open(&state);
    assert_eq!(
        rows(&reopened),
        vec![
            row("G1", GoalState::Met, "Met goal"),
            row("G2", GoalState::Dropped, "Dropped goal"),
            row("G3", GoalState::Confirmed, "Confirmed goal"),
            row("G4", GoalState::Draft, "Draft goal"),
        ]
    );
    // An earlier snapshot is never answered again: G1 is Met, so confirming it is refused.
    let again = reopened
        .confirm_goal(ConfirmGoal { goal_id: id("G1") })
        .expect("ConfirmGoal decides an outcome");
    assert!(
        matches!(again, ConfirmGoalOutcome::WrongState { .. }),
        "G1 reads as Met, not as an earlier state: {again:?}"
    );
    reopened.ports.check().expect("a refusal is not a failure");
}

/// The Goals view answers goals in the order they were first stored, before and after a reopen,
/// and an update does not move a goal.
#[test]
fn adv_goals_keep_the_order_they_were_proposed_in() {
    let state = state_dir("order");
    let mut generated = open(&state);
    for goal in ["G3", "G1", "G2"] {
        propose(&mut generated, goal, goal);
    }
    generated
        .confirm_goal(ConfirmGoal { goal_id: id("G1") })
        .expect("ConfirmGoal decides an outcome");
    let expected = vec![
        row("G3", GoalState::Draft, "G3"),
        row("G1", GoalState::Confirmed, "G1"),
        row("G2", GoalState::Draft, "G2"),
    ];
    assert_eq!(rows(&generated), expected);
    drop(generated);
    assert_eq!(rows(&open(&state)), expected);
}

/// Text and identities the specification admits survive a reopen unchanged, and two identities
/// whose stream ids could be confused stay two goals.
#[test]
fn adv_text_and_identities_round_trip_through_a_reopen() {
    let state = state_dir("round-trip");
    let long = "x".repeat(64 * 1024);
    let cases = [
        ("G 9", "space in the id", "e1"),
        ("G%209", "the escaped spelling of the id above", "e2"),
        ("G%", "a bare percent", "e3"),
        (
            "Ziel-\u{e4}",
            "Umlaute \u{e4}\u{f6}\u{fc} \u{df} \u{76ee}\u{6807} \u{1f3af}",
            "e4",
        ),
        (
            "G\"q\\b",
            "quote \" backslash \\ newline\n tab\t nul\u{0}",
            "line\r\nbreak",
        ),
        ("G-empty", "", ""),
        ("G-long", long.as_str(), long.as_str()),
    ];
    let mut generated = open(&state);
    for (goal, title, evidence) in cases {
        let outcome = propose_with(&mut generated, goal, title, evidence);
        assert!(
            matches!(outcome, ProposeGoalOutcome::Proposed { .. }),
            "{goal:?} is a new goal: {outcome:?}"
        );
    }
    generated.ports.check().expect("every proposal is kept");
    drop(generated);

    let reopened = open(&state);
    let listed: Vec<(String, String, String)> = reopened
        .goals()
        .expect("Goals answers")
        .into_iter()
        .map(|row| (row.goal_id.0, row.title, row.exit_evidence))
        .collect();
    let expected: Vec<(String, String, String)> = cases
        .iter()
        .map(|(goal, title, evidence)| {
            (
                (*goal).to_owned(),
                (*title).to_owned(),
                (*evidence).to_owned(),
            )
        })
        .collect();
    assert_eq!(listed, expected);
    for (goal, title, _) in cases {
        let held = GoalStorage::get(&reopened.ports, &id(goal)).expect("each goal reads back");
        assert_eq!(held.data.goal_id, id(goal));
        assert_eq!(held.data.title, title);
    }
    reopened.ports.check().expect("every read succeeds");
}

/// The longest goal id the specification admits, in UTF-8 bytes: the `invalid-goal-id` refusals
/// in `spec/domains/direction.yaml`. The store writes every byte that is not ASCII graphic, and
/// `%`, as `%XX` (`stream_id` in `src/store.rs`), and eventlog keeps a stream id of 1 to 512 bytes,
/// so this is the most bytes whose worst case, 3 bytes each, still fits.
const MAX_GOAL_ID_BYTES: usize = 170;

/// Every goal id `story:goal-id-bounds` admits is proposed, kept and read back after a reopen, at
/// the bound and with the bytes that escape widest. Every id it refuses (empty, or one byte past
/// the bound, whether or not eventlog would take it) gets the declared `invalid-goal-id` answer
/// from each command that addresses a goal by id, and never reaches the store: a store asked to
/// read or write an id it cannot hold latches a failure in `check()`, and this handle reports none
/// and keeps the next proposal.
#[test]
fn adv_every_goal_id_the_specification_admits_is_kept() {
    let state = state_dir("identities");
    let at_bound = |unit: &str, fill: &str| {
        let mut goal = unit.repeat(MAX_GOAL_ID_BYTES / unit.len());
        goal.push_str(fill);
        assert_eq!(goal.len(), MAX_GOAL_ID_BYTES, "{goal:?} sits at the bound");
        goal
    };
    let past_bound = |goal: String, extra: &str| {
        let goal = goal + extra;
        assert!(goal.len() > MAX_GOAL_ID_BYTES, "{goal:?} is past the bound");
        goal
    };
    let admitted: Vec<(&str, String)> = vec![
        ("one byte", "G".to_owned()),
        ("G1", "G1".to_owned()),
        ("G2", "G2".to_owned()),
        ("G3", "G3".to_owned()),
        ("G4", "G4".to_owned()),
        ("G5", "G5".to_owned()),
        ("170 ASCII letters", at_bound("e", "")),
        ("170 percent signs, 510 bytes escaped", at_bound("%", "")),
        (
            "170 spaces and controls, 510 bytes escaped",
            at_bound(" \t\n\u{0}", " \t"),
        ),
        ("85 two-byte characters", at_bound("\u{e9}", "")),
        (
            "56 three-byte characters and 2 ASCII",
            at_bound("\u{76ee}", "ab"),
        ),
        (
            "42 four-byte characters and a two-byte one",
            at_bound("\u{1f3af}", "\u{e9}"),
        ),
    ];
    let refused: Vec<(&str, String)> = vec![
        ("the empty string", String::new()),
        ("171 ASCII letters", past_bound(at_bound("e", ""), "e")),
        (
            "171 percent signs, 513 bytes escaped",
            past_bound(at_bound("%", ""), "%"),
        ),
        (
            "85 two-byte characters and 1 ASCII",
            past_bound(at_bound("\u{e9}", ""), "e"),
        ),
        ("43 four-byte characters", "\u{1f3af}".repeat(43)),
        (
            "513 ASCII letters, past eventlog's own bound",
            "e".repeat(513),
        ),
    ];

    let mut generated = open(&state);
    for (what, goal) in &admitted {
        let outcome = propose(&mut generated, goal, what);
        assert!(
            matches!(outcome, ProposeGoalOutcome::Proposed { .. }),
            "{what}: the specification admits it: {outcome:?}"
        );
        generated
            .ports
            .check()
            .unwrap_or_else(|failure| panic!("{what}: the store keeps it: {failure}"));
    }
    for (what, goal) in &refused {
        let outcome = propose(&mut generated, goal, what);
        assert!(
            matches!(&outcome, ProposeGoalOutcome::InvalidGoalId { error } if error.goal_id == id(goal)),
            "{what}: ProposeGoal refuses it: {outcome:?}"
        );
        let confirmed = generated
            .confirm_goal(ConfirmGoal { goal_id: id(goal) })
            .expect("ConfirmGoal decides an outcome");
        assert!(
            matches!(&confirmed, ConfirmGoalOutcome::InvalidGoalId { error } if error.goal_id == id(goal)),
            "{what}: ConfirmGoal refuses it: {confirmed:?}"
        );
        let met = generated
            .mark_goal_met(MarkGoalMet {
                goal_id: id(goal),
                evidence: "evidence".to_owned(),
            })
            .expect("MarkGoalMet decides an outcome");
        assert!(
            matches!(&met, MarkGoalMetOutcome::InvalidGoalId { error } if error.goal_id == id(goal)),
            "{what}: MarkGoalMet refuses it: {met:?}"
        );
        let dropped = generated
            .drop_goal(DropGoal {
                goal_id: id(goal),
                reason: "reason".to_owned(),
            })
            .expect("DropGoal decides an outcome");
        assert!(
            matches!(&dropped, DropGoalOutcome::InvalidGoalId { error } if error.goal_id == id(goal)),
            "{what}: DropGoal refuses it: {dropped:?}"
        );
        generated
            .ports
            .check()
            .unwrap_or_else(|failure| panic!("{what}: a refused id reached the store: {failure}"));
    }
    let after = propose(&mut generated, "G-after", "after the refusals");
    assert!(
        matches!(after, ProposeGoalOutcome::Proposed { .. }),
        "a proposal after the refusals: {after:?}"
    );
    generated
        .ports
        .check()
        .expect("the handle keeps writing after the refusals");
    drop(generated);

    let reopened = open(&state);
    let mut expected: Vec<Row> = admitted
        .iter()
        .map(|(what, goal)| row(goal, GoalState::Draft, what))
        .collect();
    expected.push(row("G-after", GoalState::Draft, "after the refusals"));
    assert_eq!(
        rows(&reopened),
        expected,
        "the reopened store holds every admitted id, and only those"
    );
    for (what, goal) in &admitted {
        let held = GoalStorage::get(&reopened.ports, &id(goal))
            .unwrap_or_else(|| panic!("{what}: reads back after a reopen"));
        assert_eq!(held.data.goal_id, id(goal), "{what}");
    }
    reopened
        .ports
        .check()
        .expect("every read after the reopen succeeds");
}

/// A handle opened before another handle wrote sees that write in the view, refuses a second
/// proposal of it, and an update it decides on a read the other handle has since moved past is
/// refused as `Moved` rather than overwriting it.
#[test]
fn adv_a_long_lived_handle_sees_and_respects_another_handles_writes() {
    let state = state_dir("two-handles");
    let mut early = open(&state);
    assert_eq!(rows(&early), Vec::<Row>::new());

    let mut other = open(&state);
    propose(&mut other, "G1", "From the other handle");
    other
        .ports
        .check()
        .expect("the other handle's proposal is kept");

    assert_eq!(
        rows(&early),
        vec![row("G1", GoalState::Draft, "From the other handle")]
    );
    assert!(matches!(
        propose(&mut early, "G1", "Again"),
        ProposeGoalOutcome::AlreadyProposed { .. }
    ));

    let held = GoalStorage::get(&early.ports, &id("G1")).expect("early reads G1");
    let confirmed = other
        .confirm_goal(ConfirmGoal { goal_id: id("G1") })
        .expect("ConfirmGoal decides an outcome");
    assert!(matches!(confirmed, ConfirmGoalOutcome::Confirmed { .. }));
    other.ports.check().expect("the confirmation is kept");

    let mut stale = held;
    stale.state = GoalState::Dropped;
    GoalStorage::put(&mut early.ports, stale);
    match early.ports.check() {
        Err(StoreError::Moved {
            identity,
            read,
            held,
            ..
        }) => {
            assert_eq!(identity, "G1");
            assert_eq!((read, held), (1, 2));
        }
        other => panic!("expected the stale update to be refused as Moved, got {other:?}"),
    }
    drop(early);
    drop(other);
    assert_eq!(
        rows(&open(&state)),
        vec![row("G1", GoalState::Confirmed, "From the other handle")]
    );
}

/// Eight handles race one proposal of `G9`: exactly one is kept, every other one is either refused
/// by the generated code or reported by its own handle as `Exists`, and the store holds the
/// winner's title.
#[test]
fn adv_racing_handles_keep_exactly_one_proposal() {
    const HANDLES: usize = 8;
    let state = state_dir("race-one");
    drop(store::open(&state).expect("create the store"));
    let barrier = Arc::new(Barrier::new(HANDLES));
    let racers: Vec<_> = (0..HANDLES)
        .map(|racer| {
            let state = state.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let mut generated = open(&state);
                barrier.wait();
                let outcome = propose(&mut generated, "G9", &format!("racer {racer}"));
                (racer, outcome, generated.ports.check())
            })
        })
        .collect();
    let results: Vec<_> = racers
        .into_iter()
        .map(|racer| racer.join().expect("a racer finishes"))
        .collect();

    let kept: Vec<usize> = results
        .iter()
        .filter(|(_, outcome, check)| {
            matches!(outcome, ProposeGoalOutcome::Proposed { .. }) && check.is_ok()
        })
        .map(|(racer, ..)| *racer)
        .collect();
    assert_eq!(kept.len(), 1, "exactly one proposal is kept: {results:?}");
    for (racer, outcome, check) in &results {
        if *racer == kept[0] {
            continue;
        }
        let by_the_generated_code =
            matches!(outcome, ProposeGoalOutcome::AlreadyProposed { .. }) && check.is_ok();
        let by_the_store = matches!(outcome, ProposeGoalOutcome::Proposed { .. })
            && matches!(check, Err(StoreError::Exists { identity, .. }) if identity == "G9");
        let refused = by_the_generated_code || by_the_store;
        assert!(refused, "racer {racer}: {outcome:?}, {check:?}");
    }
    assert_eq!(
        rows(&open(&state)),
        vec![row("G9", GoalState::Draft, &format!("racer {}", kept[0]))]
    );
}

/// Eight handles create the store and propose ten goals each at the same time: no proposal is lost.
#[test]
fn adv_racing_handles_lose_no_proposal() {
    const HANDLES: usize = 8;
    const GOALS: usize = 10;
    let state = state_dir("race-many");
    let barrier = Arc::new(Barrier::new(HANDLES));
    let racers: Vec<_> = (0..HANDLES)
        .map(|racer| {
            let state = state.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let mut generated = open(&state);
                for goal in 0..GOALS {
                    let goal = format!("R{racer}-G{goal}");
                    propose(&mut generated, &goal, &goal);
                }
                generated.ports.check()
            })
        })
        .collect();
    for racer in racers {
        racer
            .join()
            .expect("a racer finishes")
            .expect("every proposal of a racer is kept");
    }
    let mut listed: Vec<String> = rows(&open(&state)).into_iter().map(|row| row.0).collect();
    listed.sort();
    let mut expected: Vec<String> = (0..HANDLES)
        .flat_map(|racer| (0..GOALS).map(move |goal| format!("R{racer}-G{goal}")))
        .collect();
    expected.sort();
    assert_eq!(listed, expected);
}

/// A write the disk refuses is reported by `check`, and every later write on that handle is
/// refused even after the disk accepts writes again; a fresh handle writes.
#[test]
fn adv_a_failed_write_is_reported_and_stops_every_later_write() {
    let state = state_dir("failed-write");
    let root = state.join("tree");
    let mut generated = open(&state);

    set_mode(&root, 0o555);
    let first = propose(&mut generated, "G1", "Refused by the disk");
    set_mode(&root, 0o755);
    assert!(matches!(first, ProposeGoalOutcome::Proposed { .. }));
    let failure = generated
        .ports
        .check()
        .expect_err("the refused write is reported");
    assert!(
        matches!(&failure, StoreError::Log { identity: Some(goal), .. } if goal == "G1"),
        "{failure:?}"
    );

    propose(&mut generated, "G2", "After the failure");
    assert_eq!(
        generated.ports.check(),
        Err(failure),
        "the first failure is the one reported"
    );
    drop(generated);

    let mut reopened = open(&state);
    assert_eq!(
        rows(&reopened),
        Vec::<Row>::new(),
        "neither proposal was kept"
    );
    propose(&mut reopened, "G2", "A fresh handle");
    reopened.ports.check().expect("a fresh handle writes");
    assert_eq!(
        rows(&reopened),
        vec![row("G2", GoalState::Draft, "A fresh handle")]
    );
}

/// The event files of goal `goal`'s stream in the tree under `state`.
fn goal_event_files(state: &Path, goal: &str) -> Vec<PathBuf> {
    let stream = state
        .join("tree/tenants/conductor/streams/conductor.direction.Goal")
        .join(goal);
    let mut files: Vec<PathBuf> = fs::read_dir(&stream)
        .expect("read the goal's stream directory")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
}

/// Every regular file under `dir`, at any depth.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).expect("read a directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// Half an event file left beside the committed history, as a write cut off mid-file leaves it,
/// is never read as a record: the store either refuses to open or answers exactly the committed
/// goal.
#[test]
fn adv_a_torn_tail_is_not_read_as_a_record() {
    let state = state_dir("torn-tail");
    let mut generated = open(&state);
    propose(&mut generated, "G9", "Torn tail goal");
    generated.ports.check().expect("the proposal is kept");
    drop(generated);

    let events = goal_event_files(&state, "G9");
    assert_eq!(events.len(), 1, "one event file: {events:?}");
    let committed = fs::read(&events[0]).expect("read the event file");
    let torn = events[0].with_file_name(format!("{}.json", "0".repeat(64)));
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&torn)
        .and_then(|mut file| file.write_all(&committed[..committed.len() / 2]))
        .expect("tear a copy of the event file beside it");

    if let Ok(store) = store::open(&state) {
        let reopened = Generated::new(store);
        assert_eq!(
            rows(&reopened),
            vec![row("G9", GoalState::Draft, "Torn tail goal")]
        );
        reopened.ports.check().expect("the committed goal reads");
    }
}

/// A committed record changed on disk is refused, not answered with the changed text.
#[test]
fn adv_a_changed_committed_record_is_refused() {
    let state = state_dir("changed-record");
    let mut generated = open(&state);
    propose(&mut generated, "G9", "Original title");
    generated.ports.check().expect("the proposal is kept");
    drop(generated);

    let holding: Vec<PathBuf> = files_under(&state.join("tree/tenants"))
        .into_iter()
        .filter(|file| fs::read_to_string(file).is_ok_and(|text| text.contains("Original title")))
        .collect();
    assert_eq!(
        holding,
        goal_event_files(&state, "G9"),
        "the title is in G9's one event file"
    );
    let text = fs::read_to_string(&holding[0]).expect("read the event file");
    assert_eq!(text.matches("Original title").count(), 1);
    fs::write(
        &holding[0],
        text.replace("Original title", "Changed title!"),
    )
    .expect("change the event file");

    match store::open(&state) {
        Err(_) => {}
        Ok(store) => {
            let reopened = Generated::new(store);
            panic!(
                "the store opened a changed history and answers {:?}, check {:?}",
                rows(&reopened),
                reopened.ports.check()
            );
        }
    }
}

/// More records than one page of the log's feed (1000): the view still answers every goal.
#[test]
fn adv_the_view_reads_past_one_page_of_the_log() {
    const GOALS: usize = 1001;
    let state = state_dir("past-one-page");
    let mut generated = open(&state);
    for goal in 0..GOALS {
        let goal = format!("G{goal:04}");
        propose(&mut generated, &goal, &goal);
    }
    generated.ports.check().expect("every proposal is kept");
    drop(generated);
    let listed = rows(&open(&state));
    assert_eq!(listed.len(), GOALS);
    assert_eq!(
        listed.last(),
        Some(&row("G1000", GoalState::Draft, "G1000"))
    );
}

/// Goal `goal`'s stream in the raw log.
fn goal_stream(goal: &str) -> StreamId {
    let tenant = TenantId::new("conductor").expect("tenant");
    StreamId::new(tenant, "conductor.direction.Goal", goal).expect("stream")
}

/// Appends one event named `name` at schema `version` to goal `goal`'s new stream with the raw
/// log, as another build would write it.
fn append_raw(state: &Path, goal: &str, name: &str, version: u32) {
    callers_runtime().block_on(async {
        let log = TreeEventStore::open(state.join("tree"))
            .await
            .expect("open the raw log");
        let events = [NewEvent::new(
            name,
            version,
            json!({
                "state": "Draft",
                "goal_id": goal,
                "title": "Written by another build",
                "exit_evidence": "e",
                "owner": "a field another build added",
            }),
        )
        .expect("event")];
        let key = new_event_id();
        let meta = CommandMeta {
            idempotency_key: key.clone(),
            request_hash: request_hash(&events).expect("hash"),
            subject: "conductor".to_owned(),
            actor: "conductor".to_owned(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: OffsetDateTime::now_utc(),
            claim: None,
        };
        log.append(&goal_stream(goal), Expected::NoStream, &events, &meta)
            .await
            .expect("append the other build's record");
    });
}

/// Redacts the first event of goal `goal`'s stream with the raw log.
fn redact_raw(state: &Path, goal: &str) {
    callers_runtime().block_on(async {
        let log = TreeEventStore::open(state.join("tree"))
            .await
            .expect("open the raw log");
        log.redact(&goal_stream(goal), 1, "erased on request")
            .await
            .expect("redact the record");
    });
}

/// Leaves goal G2's newest record one the store cannot read, in the state directory given.
type MakeUnreadable = fn(&Path);

/// One Goal record the store cannot read hides only its own goal (`story:store-hardening`): for a
/// later schema version, a redacted record and an event name this build does not know, G1 and G3
/// are still listed beside the unreadable G2, and the handle reports G2 as the record that did not
/// decode.
#[test]
fn adv_one_record_the_store_cannot_read_hides_only_itself() {
    let unreadable: [(&str, MakeUnreadable); 3] = [
        ("later-schema", |state| append_raw(state, "G2", "stored", 2)),
        ("redacted", |state| {
            let mut generated = open(state);
            propose(&mut generated, "G2", "Redacted later");
            generated.ports.check().expect("G2 is kept");
            drop(generated);
            redact_raw(state, "G2");
        }),
        ("unknown-event", |state| {
            append_raw(state, "G2", "renamed", 1);
        }),
    ];
    for (kind, make_unreadable) in unreadable {
        let state = state_dir(&format!("unreadable-record-{kind}"));
        let mut generated = open(&state);
        propose(&mut generated, "G1", "Readable");
        generated.ports.check().expect("the proposal is kept");
        drop(generated);
        make_unreadable(&state);
        let mut generated = open(&state);
        propose(&mut generated, "G3", "Readable after");
        generated.ports.check().expect("the proposal is kept");
        drop(generated);

        let reopened = open(&state);
        let listed = rows(&reopened);
        let check = reopened.ports.check();
        assert_eq!(
            listed,
            vec![
                row("G1", GoalState::Draft, "Readable"),
                row("G3", GoalState::Draft, "Readable after"),
            ],
            "{kind}: the Goals view answers every goal beside the unreadable G2 ({check:?})"
        );
        assert!(
            matches!(
                &check,
                Err(StoreError::Undecodable { entity, stream, .. })
                    if *entity == "conductor.direction.Goal" && stream == "G2"
            ),
            "{kind}: the unreadable G2 is reported: {check:?}"
        );
    }
}

/// Measurement probe, not a case: the cost of one proposal, of a reopen and of the Goals view as
/// the log grows. Run with `--ignored --nocapture`.
#[test]
#[ignore = "measurement probe"]
fn adv_probe_cost_by_history_size() {
    use std::time::Instant;
    const BLOCK: usize = 100;
    const BLOCKS: usize = 20;
    let state = state_dir("probe-cost");
    let mut generated = open(&state);
    for block in 0..BLOCKS {
        let started = Instant::now();
        for goal in 0..BLOCK {
            let goal = format!("G{:05}", block * BLOCK + goal);
            propose(&mut generated, &goal, &goal);
        }
        let per = started.elapsed() / u32::try_from(BLOCK).expect("block");
        println!(
            "goals {:5}..{:5}: {per:?} per proposal",
            block * BLOCK,
            (block + 1) * BLOCK
        );
    }
    generated.ports.check().expect("every proposal is kept");
    drop(generated);
    let started = Instant::now();
    let reopened = open(&state);
    println!(
        "open with {} goals: {:?}",
        BLOCK * BLOCKS,
        started.elapsed()
    );
    let started = Instant::now();
    let listed = rows(&reopened).len();
    println!("Goals view of {listed}: {:?}", started.elapsed());
    let files = files_under(&state.join("tree/tenants"));
    let size: u64 = files
        .iter()
        .map(|file| fs::metadata(file).expect("a history file").len())
        .sum();
    println!("tree history: {} files, {size} bytes", files.len());
}
