//! Adversary cases for `story:state-dir` (wave 04, U1, pass 1).
//!
//! Each case runs the built `conductor` from its own directory under this test target's temporary
//! directory in `target/`, never from the crate directory.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::direction::obligations::{GoalsQuery, ProposeGoalBehavior};
use conductor_model::direction::{GoalId, ProposeGoal};
use eventlog_core::{
    CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId, new_event_id, request_hash,
};
use eventlog_tree::TreeEventStore;
use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, Visitor};
use serde_json::{Value, json};
use time::OffsetDateTime;

fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_state_dir")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(&dir).expect("create the case's directory");
    dir
}

fn working_dir(root: &Path) -> PathBuf {
    let dir = root.join("work");
    fs::create_dir_all(&dir).expect("create the working directory");
    dir
}

fn propose(state: &Path, goal: &str, title: &str) {
    let mut generated = Generated::new(store::open(state).expect("open the store"));
    generated
        .propose_goal(ProposeGoal {
            goal_id: GoalId(goal.to_owned()),
            title: title.to_owned(),
            exit_evidence: format!("{goal} is listed"),
        })
        .expect("ProposeGoal decides an outcome");
    generated
        .ports
        .check()
        .expect("the store kept the proposal");
}

fn conductor(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_conductor"))
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

fn path(dir: &Path) -> &str {
    dir.to_str().expect("the case's path is UTF-8")
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read the directory")
        .map(|entry| {
            entry
                .expect("read an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

// ---------------------------------------------------------------------------------------------
// 1. A view is a read: it plants no store where none was.
// ---------------------------------------------------------------------------------------------

/// `conductor goal goals` run without the flag from a directory that has no `state/` (a
/// controller's checkout, an operator's shell anywhere but the conductor checkout) must not leave
/// a `state/tree/` behind in that directory. The story's acceptance demands this of the run with
/// the flag; a read without it is no more entitled to write.
#[test]
fn adv_a_view_without_the_flag_plants_no_state_dir_in_the_working_directory() {
    let root = case_dir("view-default");
    let work = working_dir(&root);

    let output = conductor(&work, &["goal", "goals", "--format", "json"]);

    assert_eq!(
        entries(&work),
        Vec::<String>::new(),
        "a read left files in the working directory: {:?} ({})",
        walk(&work),
        describe(&output)
    );
}

/// A mistyped `--state-dir` must not be answered as an empty store that the read then creates:
/// the caller cannot tell "no goals" from "wrong directory", and the typo is now a store.
#[test]
fn adv_a_view_plants_no_store_at_a_state_dir_that_holds_none() {
    let root = case_dir("view-missing");
    let work = working_dir(&root);
    let kept = root.join("state");
    propose(&kept, "G1", "Kept");
    let typo = root.join("stat");

    let output = conductor(
        &work,
        &[
            "--state-dir",
            path(&typo),
            "goal",
            "goals",
            "--format",
            "json",
        ],
    );

    assert!(
        !typo.exists(),
        "a read created {:?} at the mistyped state directory ({})",
        walk(&typo),
        describe(&output)
    );
}

/// Every file under `dir`, relative to it.
fn walk(dir: &Path) -> Vec<String> {
    fn visit(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(read) = fs::read_dir(dir) else { return };
        for entry in read.flatten() {
            let path = entry.path();
            out.push(
                path.strip_prefix(base)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
            if path.is_dir() {
                visit(base, &path, out);
            }
        }
    }
    let mut out = Vec::new();
    visit(dir, dir, &mut out);
    out.sort();
    out
}

// ---------------------------------------------------------------------------------------------
// 2. Two state directories in one command line.
// ---------------------------------------------------------------------------------------------

/// The flag repeated at one level is a usage error (clap refuses a second value). Repeated across
/// levels, before the group and after the leaf, the later value wins today: wave 04 adversary
/// pass 1, finding 2, judged INFEASIBLE (no caller builds such a line). This case holds that
/// behaviour, so a change to it is seen.
#[test]
fn adv_the_state_dir_given_before_and_after_the_subcommand_takes_the_later_value() {
    let root = case_dir("twice");
    let work = working_dir(&root);
    let first = root.join("first");
    let second = root.join("second");
    propose(&first, "G1", "In first");
    propose(&second, "G2", "In second");

    let same_level = conductor(
        &work,
        &[
            "--state-dir",
            path(&first),
            "--state-dir",
            path(&second),
            "goal",
            "goals",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        same_level.status.code(),
        Some(2),
        "the flag twice before the group is refused: {}",
        describe(&same_level)
    );

    let across = conductor(
        &work,
        &[
            "--state-dir",
            path(&first),
            "goal",
            "goals",
            "--state-dir",
            path(&second),
            "--format",
            "json",
        ],
    );
    let infeasible = "wave 04 adversary pass 1, finding 2, INFEASIBLE: the flag before the group \
                      and again after the leaf is answered from the later value with exit 0; \
                      refuse it when a caller builds such a line";
    assert_eq!(
        across.status.code(),
        Some(0),
        "{infeasible}: {}",
        describe(&across)
    );
    let rows: Value = serde_json::from_slice(&across.stdout)
        .unwrap_or_else(|error| panic!("{infeasible}: {error}: {}", describe(&across)));
    assert_eq!(
        rows,
        json!([{
            "goal_id": "G2",
            "state": "Draft",
            "title": "In second",
            "exit_evidence": "G2 is listed",
        }]),
        "{infeasible}: {}",
        describe(&across)
    );
}

// ---------------------------------------------------------------------------------------------
// 3. A stored goal the binary cannot read.
// ---------------------------------------------------------------------------------------------

/// Appends a `conductor.direction.Goal` snapshot whose state this binary does not know, as a
/// newer binary with a state the specification added would write it.
fn store_unknown_state(state: &Path, goal: &str) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    let log = runtime
        .block_on(TreeEventStore::open(state.join("tree")))
        .expect("open the log");
    let tenant = TenantId::new("conductor").expect("the tenant");
    let stream =
        StreamId::new(tenant, "conductor.direction.Goal", goal).expect("the goal's stream");
    let events = [NewEvent::new(
        "stored",
        1,
        json!({
            "state": "Archived",
            "goal_id": goal,
            "title": "From a newer binary",
            "exit_evidence": "none",
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
    runtime
        .block_on(log.append(&stream, Expected::NoStream, &events, &meta))
        .expect("append the record");
}

/// The generated query answers the rows that decode and drops the one that does not; only the
/// store handle after it reports the loss. So `goal goals` must not print a list that silently
/// lacks a goal: it lists the readable goals, names the one that does not decode on standard
/// error, and exits 1 (wave 05 U6: views list the readable rows, `store::show`).
#[test]
fn adv_goals_refuse_a_store_holding_a_goal_that_does_not_decode() {
    let root = case_dir("undecodable");
    let work = working_dir(&root);
    let state = root.join("state");
    propose(&state, "G1", "Readable");
    store_unknown_state(&state, "G2");

    // The mutation, expressed here rather than in the source: the query without the check.
    let generated = Generated::new(store::open(&state).expect("open the store"));
    let rows = generated.goals().expect("the generated query answers");
    assert_eq!(
        rows.iter()
            .map(|row| row.goal_id.0.as_str())
            .collect::<Vec<_>>(),
        ["G1"],
        "without check, the query alone answers the readable goal and drops G2"
    );
    assert!(generated.ports.check().is_err(), "check reports G2");
    drop(generated);

    let output = conductor(
        &work,
        &[
            "--state-dir",
            path(&state),
            "goal",
            "goals",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "a list that silently lacks G2 is not an answer: {}",
        describe(&output)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"G1\"") && !stdout.contains("\"G2\""),
        "the readable G1 is listed and the unreadable G2 is not: {}",
        describe(&output)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("G2") && stderr.contains("Archived"),
        "the refusal names the record: {stderr}"
    );
}

// ---------------------------------------------------------------------------------------------
// 4. The view's columns, driven from the specification.
// ---------------------------------------------------------------------------------------------

/// The keys of one JSON object, in the order they are written.
struct Keys(Vec<String>);

impl<'de> Deserialize<'de> for Keys {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Ordered;
        impl<'de> Visitor<'de> for Ordered {
            type Value = Keys;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Keys, A::Error> {
                let mut keys = Vec::new();
                while let Some((key, IgnoredAny)) = map.next_entry::<String, IgnoredAny>()? {
                    keys.push(key);
                }
                Ok(Keys(keys))
            }
        }
        deserializer.deserialize_map(Ordered)
    }
}

/// The field names of `conductor.direction.Goals`, in the order `ess specify compile` lists them.
fn spec_goals_fields() -> Vec<String> {
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
    model["views"]["conductor.direction.Goals"]["fields"]
        .as_array()
        .expect("the Goals view has fields")
        .iter()
        .map(|field| {
            field["name"]
                .as_str()
                .expect("a field has a name")
                .to_owned()
        })
        .collect()
}

/// `jsonl` rows and the `text` header carry the view's fields, named and ordered as the
/// specification declares them; `goal.rs` keeps a hand-written copy of that list.
#[test]
fn adv_goals_columns_are_the_spec_view_fields_in_order() {
    let fields = spec_goals_fields();
    assert!(!fields.is_empty(), "the spec declares no Goals field");
    let root = case_dir("columns");
    let work = working_dir(&root);
    let state = root.join("state");
    propose(&state, "G1", "One");

    let jsonl = conductor(
        &work,
        &[
            "--state-dir",
            path(&state),
            "goal",
            "goals",
            "--format",
            "jsonl",
        ],
    );
    assert!(jsonl.status.success(), "{}", describe(&jsonl));
    let line = String::from_utf8(jsonl.stdout).expect("UTF-8");
    let Keys(keys) = serde_json::from_str(line.trim_end()).expect("one JSON object");
    assert_eq!(keys, fields, "jsonl keys against the spec's Goals fields");

    let text = conductor(
        &work,
        &[
            "--state-dir",
            path(&state),
            "goal",
            "goals",
            "--format",
            "text",
        ],
    );
    assert!(text.status.success(), "{}", describe(&text));
    let header: Vec<String> = String::from_utf8(text.stdout)
        .expect("UTF-8")
        .lines()
        .next()
        .expect("a header line")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        header, fields,
        "text header against the spec's Goals fields"
    );
}
