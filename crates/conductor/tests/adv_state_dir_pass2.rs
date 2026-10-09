//! Adversary cases for `story:state-dir` (wave 04, U1, pass 2): correction 1's `open_existing`
//! against store directories that exist but hold no store, every view against a missing store,
//! the view's state names against the specification, and the binding's own words against the
//! binary's.
//!
//! Each case runs the built `conductor` from its own directory under this test target's temporary
//! directory in `target/`, never from the crate directory.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use clap::CommandFactory as _;
use conductor_cli::cli::Cli;
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::direction::obligations::{
    ConfirmGoalBehavior, DropGoalBehavior, MarkGoalMetBehavior, ProposeGoalBehavior,
};
use conductor_model::direction::{
    ConfirmGoal, ConfirmGoalOutcome, DropGoal, DropGoalOutcome, GoalId, MarkGoalMet,
    MarkGoalMetOutcome, ProposeGoal, ProposeGoalOutcome,
};
use serde_json::Value;

fn case_dir(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_state_dir_pass2")
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

fn path(dir: &Path) -> &str {
    dir.to_str().expect("the case's path is UTF-8")
}

/// Every entry under `dir`, relative to it, sorted.
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
            if path.is_dir() && !path.is_symlink() {
                visit(base, &path, out);
            }
        }
    }
    let mut out = Vec::new();
    visit(dir, dir, &mut out);
    out.sort();
    out
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Runs `ess specify <args> --path spec --format json` and answers the JSON it prints.
fn ess_specify(verb: &str, extra: &[&str]) -> Value {
    let spec = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec");
    let output = Command::new("ess")
        .args(["specify", verb, "--path"])
        .arg(&spec)
        .args(extra)
        .args(["--format", "json"])
        .output()
        .expect("`ess` is on PATH");
    assert!(
        output.status.success(),
        "`ess specify {verb}` exited {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("`ess specify` prints JSON")
}

fn strings(value: &Value, what: &str) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{what} is a list"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("{what} holds strings"))
                .to_owned()
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// 1. A store directory that is there and holds no store.
// ---------------------------------------------------------------------------------------------

/// The refusal a view gives where the store directory holds no store: exit 1, no rows, one
/// stderr line naming the directory, and the directory left exactly as it was found.
fn assert_refused_untouched(output: &Output, store: &Path, before: &[String], what: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "{what}: a view on a store directory that holds no store is refused: {}",
        describe(output)
    );
    assert!(
        output.stdout.is_empty(),
        "{what}: no rows are printed: {}",
        describe(output)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "{what}: one line: {stderr:?}");
    assert!(
        stderr.contains(path(store)),
        "{what}: the refusal names {}: {stderr:?}",
        store.display()
    );
    assert_eq!(
        walk(store),
        before,
        "{what}: a view wrote into the store directory ({})",
        describe(output)
    );
}

/// `state/tree/` exists and is empty: what a command leaves between creating the directory and
/// creating the tree in it (`store::open`, then eventlog's `TreeEventStore::open`), or what an
/// operator's `mkdir -p` leaves. `store::open_existing` refuses a directory with no `store.json`;
/// `TreeEventStore::open` would mint a description there and answer `[]`. Nothing else in the
/// suite runs a view on such a directory, so a handler or a `store::open_existing` that reached
/// for `open` would pass every other case.
#[test]
fn adv2_a_view_on_an_empty_store_directory_refuses_and_writes_nothing() {
    let root = case_dir("empty-store-dir");
    let work = working_dir(&root);
    let state = root.join("state");
    let store_dir = state.join("tree");
    fs::create_dir_all(&store_dir).expect("create an empty store directory");

    // The mutation, expressed here rather than in the source: opening it with `open`.
    let probe = root.join("probe");
    fs::create_dir_all(probe.join("tree")).expect("create the probe's store directory");
    drop(store::open(&probe).expect("`store::open` mints a store in an empty directory"));
    assert_eq!(
        walk(&probe.join("tree")),
        [".lock", "store.json"],
        "what `open` leaves behind, and what the case below must not find"
    );

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
    assert_refused_untouched(&output, &store_dir, &[], "an empty tree/");
    assert_eq!(walk(&work), Vec::<String>::new());
}

/// `state/tree/.lock` and nothing else: the state a first command is in after it took the tree's
/// writer lock to create the tree and before eventlog wrote its `store.json`. A view must refuse
/// there and must not finish the creation.
#[test]
fn adv2_a_view_on_a_store_with_a_lock_and_no_manifest_refuses_and_writes_nothing() {
    let root = case_dir("lock-only");
    let work = working_dir(&root);
    let state = root.join("state");
    let store_dir = state.join("tree");
    fs::create_dir_all(&store_dir).expect("create the store directory");
    fs::write(store_dir.join(".lock"), b"").expect("create the writer lock");

    let output = conductor(
        &work,
        &[
            "goal",
            "goals",
            "--state-dir",
            path(&state),
            "--format",
            "json",
        ],
    );
    assert_refused_untouched(
        &output,
        &store_dir,
        &[".lock".to_owned()],
        "a lock without a manifest",
    );
    assert_eq!(walk(&work), Vec::<String>::new());
}

// ---------------------------------------------------------------------------------------------
// 2. Every view, against a missing store, plants none.
// ---------------------------------------------------------------------------------------------

/// Every view the `cli:` block places, as `(group, wire)`.
fn views() -> Vec<(String, String)> {
    let model = ess_specify("compile", &[]);
    let component = model["components"]
        .as_object()
        .expect("components")
        .values()
        .find(|component| !component["cli"].is_null())
        .expect("a component declares a cli block");
    let mut views = Vec::new();
    for group in component["cli"]["groups"].as_array().expect("cli groups") {
        let name = group["name"].as_str().expect("a group's name").to_owned();
        for qualified in strings(&group["views"], "a group's views") {
            let wire = model["views"][&qualified]["naming"]["wire"]
                .as_str()
                .unwrap_or_else(|| panic!("view `{qualified}` has naming.wire"))
                .to_owned();
            views.push((name.clone(), wire));
        }
    }
    views
}

/// `src/lib.rs` tells the next story that "a handler that reads or writes records opens the store
/// through `state::open`", which creates one. Correction 1 routed `goal goals` through
/// `state::open_existing`; this holds every view to it, so a view filled by a later story that
/// follows the crate document fails here rather than planting `state/tree/` in a controller's
/// checkout. A view that is not written yet answers "not implemented", which plants nothing either.
#[test]
fn adv2_no_view_plants_a_store_with_or_without_the_flag() {
    let views = views();
    assert!(
        views.len() > 1,
        "the cli block places {} view(s)",
        views.len()
    );
    let root = case_dir("every-view");
    let mut problems = Vec::new();
    for (group, wire) in &views {
        let work = root.join(format!("{group}-{wire}"));
        fs::create_dir_all(&work).expect("create the view's working directory");
        let missing = work.join("missing");

        let with = conductor(
            &work,
            &[
                "--state-dir",
                path(&missing),
                group.as_str(),
                wire.as_str(),
                "--format",
                "json",
            ],
        );
        if with.status.success() || missing.exists() {
            problems.push(format!(
                "`{group} {wire} --state-dir <missing>`: {:?} created, {}",
                walk(&work),
                describe(&with)
            ));
        }

        let without = conductor(&work, &[group.as_str(), wire.as_str(), "--format", "json"]);
        if without.status.success() || !walk(&work).is_empty() {
            problems.push(format!(
                "`{group} {wire}` with no state/: {:?} created, {}",
                walk(&work),
                describe(&without)
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "{} of {} view runs answered from, or planted, a store that was not there:\n  - {}",
        problems.len(),
        views.len() * 2,
        problems.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------------------------
// 3. The view's state column, against the specification's lifecycle.
// ---------------------------------------------------------------------------------------------

fn id(goal: &str) -> GoalId {
    GoalId(goal.to_owned())
}

/// `goal.rs` spells each `GoalState` by hand (`state_name`), because the generated enum has no
/// rendering. Every other binary case stores only proposed goals, so only `Draft` is ever
/// printed: swap any two of the other three names and the suite stays green. Here a goal is put
/// in each lifecycle state through the generated behaviours, and the printed names are held to
/// `lifecycle.states` of `conductor.direction.Goal` in `ess specify compile`.
#[test]
fn adv2_goals_name_every_lifecycle_state_as_the_spec_does() {
    let model = ess_specify("compile", &[]);
    let mut spec_states = strings(
        &model["entities"]["conductor.direction.Goal"]["lifecycle"]["states"],
        "the Goal lifecycle's states",
    );
    spec_states.sort();

    let root = case_dir("states");
    let work = working_dir(&root);
    let state = root.join("state");
    let mut generated = Generated::new(store::open(&state).expect("open the store"));
    for goal in ["G1", "G2", "G3", "G4"] {
        let outcome = generated
            .propose_goal(ProposeGoal {
                goal_id: id(goal),
                title: goal.to_owned(),
                exit_evidence: format!("{goal} is listed"),
            })
            .expect("ProposeGoal decides an outcome");
        assert!(matches!(outcome, ProposeGoalOutcome::Proposed { .. }));
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
    assert!(output.status.success(), "{}", describe(&output));
    let rows: Value = serde_json::from_slice(&output.stdout).expect("json is one JSON value");
    let printed: Vec<(String, String)> = rows
        .as_array()
        .expect("json is a list of rows")
        .iter()
        .map(|row| {
            (
                row["goal_id"].as_str().expect("a goal_id").to_owned(),
                row["state"].as_str().expect("a state").to_owned(),
            )
        })
        .collect();
    assert_eq!(
        printed,
        [
            ("G1".to_owned(), "Met".to_owned()),
            ("G2".to_owned(), "Dropped".to_owned()),
            ("G3".to_owned(), "Confirmed".to_owned()),
            ("G4".to_owned(), "Draft".to_owned()),
        ],
        "each goal is printed in the state the behaviours put it in"
    );
    let mut printed_states: Vec<String> = printed.into_iter().map(|(_, state)| state).collect();
    printed_states.sort();
    assert_eq!(
        printed_states, spec_states,
        "the printed states are the Goal lifecycle's, each once"
    );
}

// ---------------------------------------------------------------------------------------------
// 4. The binding's own words against the binary's.
// ---------------------------------------------------------------------------------------------

/// The `ess-cli/1` binding is the presentation contract of this binary: `ess generate cli` and any
/// reader of `ess specify cli` take the binary's description from its `about`. `tests/cli_tree.rs`
/// compares the binding's binary name, `globals.state` and each command's `about`, and not the
/// root `about`, so the two may say different things and every gate stays green.
#[test]
fn adv2_the_binding_describes_the_binary_as_its_help_does() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cli.yaml");
    let plan = ess_specify("cli", &["--binding", path(&file)]);
    let binding_about = plan["about"].as_str().expect("the binding has an about");
    let binary_about = Cli::command()
        .get_about()
        .map(ToString::to_string)
        .expect("the binary has an about");
    assert_eq!(
        binding_about, binary_about,
        "crates/conductor/cli.yaml `about` against `conductor --help`"
    );
}
