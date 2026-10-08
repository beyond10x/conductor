//! `story:goal-and-resource-commands`: the `conductor goal` and `conductor resource` commands and
//! views, run as the built binary against the store under `--state-dir`.
//!
//! Each case has its own directory under this test target's temporary directory in `target/`: the
//! binary runs from `work/` in it and keeps its records in `state/` beside it. A command that is
//! refused exits 1 with one line on standard error naming the error the specification declares,
//! and writes nothing on standard output.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

/// Free bytes on `/` that clear the 30G floor for a build slot (`spec/domains/dispatch.yaml`).
const DISK_FREE: &str = "64424509440";

/// One case's directories: where the binary runs, and the state directory it is pointed at.
struct Case {
    work: PathBuf,
    state: PathBuf,
}

impl Case {
    fn new(case: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("goal_resource")
            .join(case);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case's directory");
        }
        let work = root.join("work");
        fs::create_dir_all(&work).expect("create the case's working directory");
        Self {
            work,
            state: root.join("state"),
        }
    }

    /// Runs `conductor --state-dir <state> <args>` from the case's working directory.
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_conductor"))
            .current_dir(&self.work)
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    /// Every command and view the case ran honoured `--state-dir`: the store is under it, and the
    /// working directory, where `state/` is looked for without the flag, holds nothing.
    fn kept_under_the_state_dir(&self) {
        assert!(
            self.state.join("tree").is_dir(),
            "no store under --state-dir {}",
            self.state.display()
        );
        let planted: Vec<PathBuf> = fs::read_dir(&self.work)
            .expect("read the working directory")
            .map(|entry| entry.expect("a directory entry").path())
            .collect();
        assert!(
            planted.is_empty(),
            "a handler ignored --state-dir and wrote under the working directory: {planted:?}"
        );
    }

    /// Runs a command that must succeed, and answers its standard output.
    fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "`conductor {}` exited {:?}; stderr: {}",
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("stdout is UTF-8")
    }

    /// Runs a command that the specification refuses with `error`: exit 1, nothing on standard
    /// output, and one line on standard error naming the command and `error`.
    fn refused(&self, args: &[&str], error: &str) {
        let output = self.run(args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let command = format!("conductor: {} {}: {error}: ", args[0], args[1]);
        assert!(
            output.status.code() == Some(1)
                && output.stdout.is_empty()
                && stderr.starts_with(&command)
                && stderr.ends_with('\n')
                && stderr.lines().count() == 1,
            "`conductor {}` should be refused with {error}: exit 1, no stdout, one stderr line \
             starting {command:?}; got exit {:?}, stdout {:?}, stderr {stderr:?}",
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
        );
    }

    /// The rows of a view, read as `--format jsonl`.
    fn rows(&self, group: &str, view: &str) -> Vec<Value> {
        self.ok(&[group, view, "--format", "jsonl"])
            .lines()
            .map(|line| serde_json::from_str(line).expect("each jsonl line is a JSON object"))
            .collect()
    }

    /// The state the `goals` view gives `goal`.
    fn goal_state(&self, goal: &str) -> String {
        let rows = self.rows("goal", "goals");
        let row = rows
            .iter()
            .find(|row| row["goal_id"] == goal)
            .unwrap_or_else(|| panic!("the goals view has no row for {goal}: {rows:?}"));
        row["state"].as_str().expect("state is text").to_owned()
    }

    /// The state the `resource-requests` view gives `request`.
    fn request_state(&self, request: &str) -> String {
        let rows = self.rows("resource", "resource-requests");
        let row = rows
            .iter()
            .find(|row| row["resource_request_id"] == request)
            .unwrap_or_else(|| panic!("the ledger has no row for {request}: {rows:?}"));
        row["state"].as_str().expect("state is text").to_owned()
    }

    fn propose(&self, goal: &str, title: &str) {
        self.ok(&[
            "goal",
            "propose-goal",
            "--goal-id",
            goal,
            "--title",
            title,
            "--exit-evidence",
            &format!("{goal} is listed as met in NORTHSTAR.md"),
        ]);
    }

    /// Requests `amount` of `resource` for cortex, and answers the id the command prints.
    fn request(&self, resource: &str, amount: &str) -> String {
        let printed = self.ok(&[
            "resource",
            "request-resource",
            "--repository",
            "cortex",
            "--resource",
            resource,
            "--amount",
            amount,
            "--requested-at",
            "2026-10-07T10:00:00Z",
        ]);
        let id = printed
            .strip_suffix('\n')
            .unwrap_or_else(|| panic!("request-resource prints one line, got {printed:?}"));
        assert!(
            !id.is_empty() && !id.contains('\n'),
            "request-resource prints the request's id on one line, got {printed:?}"
        );
        id.to_owned()
    }
}

/// Goal lifecycle: G9 is proposed, confirmed and met, and reads `Met`. Confirming it again is
/// refused with `GoalStateConflict`, and it still reads `Met`.
#[test]
fn g9_is_proposed_confirmed_and_met_and_confirming_it_again_is_refused() {
    let case = Case::new("goal-lifecycle");
    case.propose("G9", "Ship the pilot");
    assert_eq!(case.goal_state("G9"), "Draft");
    case.ok(&["goal", "confirm-goal", "--goal-id", "G9"]);
    assert_eq!(case.goal_state("G9"), "Confirmed");
    case.ok(&[
        "goal",
        "mark-goal-met",
        "--goal-id",
        "G9",
        "--evidence",
        "the pilot ran on 2026-10-07",
    ]);
    assert_eq!(case.goal_state("G9"), "Met");

    case.refused(
        &["goal", "confirm-goal", "--goal-id", "G9"],
        "GoalStateConflict",
    );
    assert_eq!(
        case.rows("goal", "goals"),
        vec![json!({
            "goal_id": "G9",
            "state": "Met",
            "title": "Ship the pilot",
            "exit_evidence": "G9 is listed as met in NORTHSTAR.md",
        })],
        "the refused confirmation moved nothing"
    );
    case.kept_under_the_state_dir();
}

/// Dropping a goal: G8 is proposed and dropped, and reads `Dropped`.
#[test]
fn g8_is_proposed_and_dropped_and_reads_dropped() {
    let case = Case::new("goal-drop");
    case.propose("G8", "Retire the mailbox");
    case.ok(&[
        "goal",
        "drop-goal",
        "--goal-id",
        "G8",
        "--reason",
        "superseded by G9",
    ]);
    assert_eq!(case.goal_state("G8"), "Dropped");
    case.kept_under_the_state_dir();
}

/// Serving repositories: cortex is added to G9 and then removed, and the `servings` view shows
/// each change. `goal goals --format jsonl` run twice gives byte-identical output.
#[test]
fn cortex_is_added_to_g9_and_removed_and_the_view_shows_each_change() {
    let case = Case::new("servings");
    case.propose("G9", "Ship the pilot");
    assert_eq!(case.rows("goal", "servings"), Vec::<Value>::new());

    let printed = case.ok(&[
        "goal",
        "add-serving",
        "--goal-id",
        "G9",
        "--repository",
        "cortex",
    ]);
    let rows = case.rows("goal", "servings");
    assert_eq!(rows.len(), 1, "one serving after the add: {rows:?}");
    let serving = rows[0]["serving_id"]
        .as_str()
        .expect("serving_id is text")
        .to_owned();
    assert_eq!(
        printed,
        format!("{serving}\n"),
        "add-serving prints the id it assigned"
    );
    assert_eq!(
        rows,
        vec![json!({
            "serving_id": serving,
            "state": "Serving",
            "goal_id": "G9",
            "repository": "cortex",
        })]
    );

    case.ok(&[
        "goal",
        "remove-serving",
        "--serving-id",
        &serving,
        "--reason",
        "cortex no longer serves G9",
    ]);
    assert_eq!(
        case.rows("goal", "servings"),
        vec![json!({
            "serving_id": serving,
            "state": "Removed",
            "goal_id": "G9",
            "repository": "cortex",
        })]
    );
    case.refused(
        &[
            "goal",
            "remove-serving",
            "--serving-id",
            &serving,
            "--reason",
            "twice",
        ],
        "ServingStateConflict",
    );

    case.propose("G1", "Run every repository from one board");
    let first = case.ok(&["goal", "goals", "--format", "jsonl"]);
    let second = case.ok(&["goal", "goals", "--format", "jsonl"]);
    assert_eq!(first.lines().count(), 2, "both goals are listed: {first:?}");
    assert_eq!(
        first, second,
        "the goals export is byte-identical run to run"
    );
    case.kept_under_the_state_dir();
}

/// `add-serving` checks the goal id against the bound `story:goal-id-bounds` set before the
/// generated behaviour reads the goal by it, and answers `InvalidGoalId` itself: empty, and over
/// 170 UTF-8 bytes. An id at the bound is one the store can keep, so it reaches the behaviour and
/// is answered `GoalNotFound`. None of them adds a serving, and the store keeps taking writes.
#[test]
fn add_serving_refuses_a_goal_id_no_goal_can_have_with_invalid_goal_id() {
    let case = Case::new("add-serving-bounds");
    case.propose("G9", "Ship the pilot");
    let over: [(&str, String); 3] = [
        ("the empty string", String::new()),
        ("171 percent signs", "%".repeat(171)),
        ("86 two-byte characters, 172 bytes", "é".repeat(86)),
    ];
    for (what, goal) in &over {
        assert!(
            goal.is_empty() || goal.len() > 170,
            "{what} is out of bounds"
        );
        case.refused(
            &[
                "goal",
                "add-serving",
                "--goal-id",
                goal,
                "--repository",
                "cortex",
            ],
            "InvalidGoalId",
        );
    }
    case.refused(
        &[
            "goal",
            "add-serving",
            "--goal-id",
            &"%".repeat(170),
            "--repository",
            "cortex",
        ],
        "GoalNotFound",
    );
    assert_eq!(case.rows("goal", "servings"), Vec::<Value>::new());

    case.ok(&[
        "goal",
        "add-serving",
        "--goal-id",
        "G9",
        "--repository",
        "cortex",
    ]);
    let rows = case.rows("goal", "servings");
    assert_eq!(
        rows.iter()
            .map(|row| (row["goal_id"].clone(), row["state"].clone()))
            .collect::<Vec<_>>(),
        vec![(json!("G9"), json!("Serving"))],
        "the store still takes writes after the refusals"
    );
    case.kept_under_the_state_dir();
}

/// Adding a serving to a met goal is refused with `GoalStateConflict`.
#[test]
fn add_serving_to_a_met_goal_is_refused() {
    let case = Case::new("add-serving-closed");
    case.propose("G9", "Ship the pilot");
    case.ok(&["goal", "confirm-goal", "--goal-id", "G9"]);
    case.ok(&[
        "goal",
        "mark-goal-met",
        "--goal-id",
        "G9",
        "--evidence",
        "done",
    ]);
    case.refused(
        &[
            "goal",
            "add-serving",
            "--goal-id",
            "G9",
            "--repository",
            "cortex",
        ],
        "GoalStateConflict",
    );
    assert_eq!(case.rows("goal", "servings"), Vec::<Value>::new());
    case.kept_under_the_state_dir();
}

/// Resource requests: amount 0 is refused with `InvalidAmount`. One request is granted and reads
/// `Granted`, then released and reads `Released`; another is refused and reads `Refused`.
/// Refusing the granted one is refused with `ResourceRequestStateConflict`.
#[test]
fn resource_requests_are_granted_released_and_refused_through_the_ledger() {
    let case = Case::new("resources");
    case.refused(
        &[
            "resource",
            "request-resource",
            "--repository",
            "cortex",
            "--resource",
            "Disk",
            "--amount",
            "0",
            "--requested-at",
            "2026-10-07T10:00:00Z",
        ],
        "InvalidAmount",
    );
    assert_eq!(
        case.rows("resource", "resource-requests"),
        Vec::<Value>::new(),
        "the refused request kept nothing"
    );

    let granted = case.request("BuildSlot", "1");
    let refused = case.request("Usage", "2");
    assert_ne!(granted, refused, "each request has its own id");
    assert_eq!(case.request_state(&granted), "Requested");

    case.ok(&[
        "resource",
        "grant-resource",
        "--resource-request-id",
        &granted,
        "--disk-free-bytes",
        DISK_FREE,
    ]);
    assert_eq!(case.request_state(&granted), "Granted");
    case.refused(
        &[
            "resource",
            "refuse-resource",
            "--resource-request-id",
            &granted,
            "--reason",
            "too late",
        ],
        "ResourceRequestStateConflict",
    );
    assert_eq!(case.request_state(&granted), "Granted");

    case.ok(&[
        "resource",
        "release-resource",
        "--resource-request-id",
        &granted,
    ]);
    assert_eq!(case.request_state(&granted), "Released");

    case.ok(&[
        "resource",
        "refuse-resource",
        "--resource-request-id",
        &refused,
        "--reason",
        "the usage budget is spent",
    ]);
    assert_eq!(
        case.rows("resource", "resource-requests"),
        vec![
            json!({
                "resource_request_id": granted,
                "state": "Released",
                "repository": "cortex",
                "resource": "BuildSlot",
                "amount": 1,
                "requested_at": "2026-10-07T10:00:00Z",
            }),
            json!({
                "resource_request_id": refused,
                "state": "Refused",
                "repository": "cortex",
                "resource": "Usage",
                "amount": 2,
                "requested_at": "2026-10-07T10:00:00Z",
            }),
        ]
    );
    case.kept_under_the_state_dir();
}

/// A build slot is not granted under the 30G disk floor; the request stays open.
#[test]
fn a_build_slot_under_the_disk_floor_is_refused_and_stays_requested() {
    let case = Case::new("disk-floor");
    let request = case.request("BuildSlot", "1");
    case.refused(
        &[
            "resource",
            "grant-resource",
            "--resource-request-id",
            &request,
            "--disk-free-bytes",
            "1073741824",
        ],
        "DiskBelowFloor",
    );
    assert_eq!(case.request_state(&request), "Requested");
    case.kept_under_the_state_dir();
}

/// `--state-dir` is honoured by both groups: a `goal` and a `resource` command create the store
/// under the directory the flag names, nothing appears under the working directory, and the views
/// read the records back from there.
#[test]
fn the_state_dir_flag_is_honoured_by_goal_and_resource_commands() {
    let case = Case::new("state-dir-flag");
    assert!(!case.state.exists(), "the state directory starts absent");
    case.propose("G9", "Ship the pilot");
    let request = case.request("Disk", "10");
    case.kept_under_the_state_dir();
    assert_eq!(case.goal_state("G9"), "Draft");
    assert_eq!(case.request_state(&request), "Requested");
    case.kept_under_the_state_dir();
}
