//! `story:state-dir`: the built `conductor` finds its store through `--state-dir`, the global flag
//! the `ess-cli/1` binding `crates/conductor/cli.yaml` names, and without it under `state/` in the
//! working directory.
//!
//! Each case runs the binary from its own directory under this test target's temporary directory
//! in `target/`, and seeds the store through the library, as `tests/store.rs` does.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::direction::obligations::ProposeGoalBehavior;
use conductor_model::direction::{GoalId, ProposeGoal};
use serde_json::{Value, json};

/// An empty directory for one case.
fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("state_dir")
        .join(case);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(&dir).expect("create the case's directory");
    dir
}

/// An empty directory under `root`, to run the binary from.
fn working_dir(root: &Path) -> PathBuf {
    let dir = root.join("work");
    fs::create_dir_all(&dir).expect("create the working directory");
    dir
}

/// Proposes goal `goal` into the store kept under `state`.
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

/// The `Goals` row a proposed goal reads as.
fn draft(goal: &str, title: &str) -> Value {
    json!({
        "goal_id": goal,
        "state": "Draft",
        "title": title,
        "exit_evidence": format!("{goal} is listed"),
    })
}

/// Runs conductor in `cwd`, with `cwd` as the home directory too, so the operator's own config file
/// under the real home directory is never read.
fn conductor(cwd: &Path, args: &[&str]) -> Output {
    common::conductor(cwd)
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

fn stdout(output: &Output, what: &str) -> String {
    assert!(
        output.status.success(),
        "{what} exited {:?}; stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn json_rows(output: &Output, what: &str) -> Value {
    let text = stdout(output, what);
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{what} printed {text:?}: {error}"))
}

fn path(dir: &Path) -> &str {
    dir.to_str().expect("the case's path is UTF-8")
}

/// The names in `dir`, sorted.
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

#[test]
fn goals_answer_from_the_state_dir_flag_and_leave_the_working_directory_alone() {
    let root = case_dir("flag");
    let work = working_dir(&root);
    let elsewhere = root.join("elsewhere");
    propose(&elsewhere, "G1", "Kept elsewhere");

    let output = conductor(
        &work,
        &[
            "--state-dir",
            path(&elsewhere),
            "goal",
            "goals",
            "--format",
            "json",
        ],
    );

    assert_eq!(
        json_rows(
            &output,
            "`conductor --state-dir <elsewhere> goal goals --format json`"
        ),
        json!([draft("G1", "Kept elsewhere")]),
        "the view answers from <elsewhere>/tree"
    );
    assert!(
        !work.join("state").exists(),
        "no state/ is created in the working directory"
    );
    assert_eq!(
        entries(&work),
        Vec::<String>::new(),
        "the working directory is left empty"
    );
    assert_eq!(
        entries(&elsewhere),
        ["tree"],
        "the store is <elsewhere>/tree"
    );
}

#[test]
fn the_state_dir_flag_is_taken_after_the_subcommand_too() {
    let root = case_dir("after");
    let work = working_dir(&root);
    let elsewhere = root.join("elsewhere");
    propose(&elsewhere, "G1", "Kept elsewhere");

    let output = conductor(
        &work,
        &[
            "goal",
            "goals",
            "--state-dir",
            path(&elsewhere),
            "--format",
            "json",
        ],
    );

    assert_eq!(
        json_rows(
            &output,
            "`conductor goal goals --state-dir <elsewhere> --format json`"
        ),
        json!([draft("G1", "Kept elsewhere")])
    );
    assert_eq!(entries(&work), Vec::<String>::new());
}

/// Both branches of the one resolution: without the flag the store is `state/tree/` under the
/// working directory; with it, the flag's directory alone, even where the working directory has a
/// `state/` of its own (a controller's checkout running `conductor guard`).
#[test]
fn without_the_flag_the_store_is_state_under_the_working_directory_and_the_flag_wins() {
    let root = case_dir("default");
    let work = working_dir(&root);
    propose(&work.join("state"), "G2", "Kept in the working directory");
    let elsewhere = root.join("elsewhere");
    propose(&elsewhere, "G1", "Kept elsewhere");

    let without = conductor(&work, &["goal", "goals", "--format", "json"]);
    assert_eq!(
        json_rows(&without, "`conductor goal goals --format json`"),
        json!([draft("G2", "Kept in the working directory")]),
        "without the flag the view answers from ./state/tree"
    );

    let with = conductor(
        &work,
        &[
            "--state-dir",
            path(&elsewhere),
            "goal",
            "goals",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        json_rows(
            &with,
            "`conductor --state-dir <elsewhere> goal goals --format json`"
        ),
        json!([draft("G1", "Kept elsewhere")]),
        "the flag's store alone answers"
    );
}

/// An empty value would be read as the working directory itself and put `tree/` there.
#[test]
fn an_empty_state_dir_is_refused_before_anything_is_written() {
    let root = case_dir("empty");
    let work = working_dir(&root);

    let output = conductor(&work, &["--state-dir", "", "goal", "goals"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "an empty --state-dir is a usage error; stderr: {stderr}"
    );
    assert!(
        stderr.contains("'--state-dir <DIR>'") && !stderr.contains("unexpected argument"),
        "the refusal names the flag's value, not an unknown flag: {stderr}"
    );
    assert_eq!(entries(&work), Vec::<String>::new());
}

#[test]
fn goals_render_in_every_format_the_view_offers() {
    let root = case_dir("formats");
    let work = working_dir(&root);
    let elsewhere = root.join("elsewhere");
    propose(&elsewhere, "G1", "First");
    propose(&elsewhere, "G10", "Second | piped\nline two");
    let rows = json!([
        draft("G1", "First"),
        draft("G10", "Second | piped\nline two"),
    ]);
    let goals = |format: &str| {
        let output = conductor(
            &work,
            &[
                "--state-dir",
                path(&elsewhere),
                "goal",
                "goals",
                "--format",
                format,
            ],
        );
        stdout(
            &output,
            &format!("`conductor goal goals --format {format}`"),
        )
    };

    let json = goals("json");
    assert_eq!(
        serde_json::from_str::<Value>(&json).expect("json is one JSON value"),
        rows
    );
    assert_eq!(json.lines().count(), 1, "json is one line: {json:?}");

    let jsonl = goals("jsonl");
    let lines: Vec<Value> = jsonl
        .lines()
        .map(|line| serde_json::from_str(line).expect("each jsonl line is one JSON value"))
        .collect();
    assert_eq!(Value::Array(lines), rows);

    assert_eq!(
        goals("text"),
        "goal_id  state  title                    exit_evidence\n\
         G1       Draft  First                    G1 is listed\n\
         G10      Draft  Second | piped line two  G10 is listed\n"
    );
    assert_eq!(goals("text"), goals_default(&work, &elsewhere));

    assert_eq!(
        goals("markdown"),
        "| goal_id | state | title | exit_evidence |\n\
         |---|---|---|---|\n\
         | G1 | Draft | First | G1 is listed |\n\
         | G10 | Draft | Second \\| piped<br>line two | G10 is listed |\n"
    );
}

/// `goal goals` without `--format`: text.
fn goals_default(work: &Path, state: &Path) -> String {
    let output = conductor(work, &["--state-dir", path(state), "goal", "goals"]);
    stdout(&output, "`conductor goal goals`")
}

#[test]
fn an_empty_store_lists_no_goals() {
    let root = case_dir("no-goals");
    let work = working_dir(&root);
    let elsewhere = root.join("elsewhere");
    drop(store::open(&elsewhere).expect("create an empty store, as a command would"));

    let output = conductor(
        &work,
        &[
            "--state-dir",
            path(&elsewhere),
            "goal",
            "goals",
            "--format",
            "json",
        ],
    );

    assert_eq!(
        json_rows(
            &output,
            "`conductor --state-dir <empty> goal goals --format json`"
        ),
        json!([])
    );
    assert_eq!(entries(&work), Vec::<String>::new());
}

/// A view opens an existing store only. Where none is, with the flag or without it, it answers
/// exit 1 and one line naming the store it looked for, and creates nothing: "no goals" and "wrong
/// directory" must not look alike, and a read must not plant a store.
#[test]
fn a_view_on_a_missing_store_is_exit_1_and_creates_nothing() {
    let root = case_dir("missing");
    let work = working_dir(&root);
    let missing = root.join("missing");

    let refused = |output: &Output, looked_for: &Path, what: &str| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{what} on a missing store exits 1; stderr: {stderr}"
        );
        assert!(
            output.stdout.is_empty(),
            "{what} prints no rows: {:?}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(
            stderr.lines().count(),
            1,
            "{what} answers one stderr line: {stderr:?}"
        );
        assert!(
            stderr.contains(path(looked_for)),
            "{what} names the store it looked for, {}: {stderr:?}",
            looked_for.display()
        );
    };

    let with = conductor(
        &work,
        &[
            "--state-dir",
            path(&missing),
            "goal",
            "goals",
            "--format",
            "json",
        ],
    );
    refused(
        &with,
        &missing.join("tree"),
        "`conductor --state-dir <missing> goal goals`",
    );
    assert!(
        !missing.exists(),
        "the missing state directory is not created"
    );

    let without = conductor(&work, &["goal", "goals", "--format", "json"]);
    let looked_for = work
        .canonicalize()
        .expect("the working directory resolves")
        .join("state")
        .join("tree");
    refused(&without, &looked_for, "`conductor goal goals`");
    assert_eq!(
        entries(&work),
        Vec::<String>::new(),
        "no state/ is created in the working directory"
    );
}
