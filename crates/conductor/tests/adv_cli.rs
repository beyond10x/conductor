//! Adversary cases for `story:cli-from-spec` (wave 02, U1, pass 1).
//!
//! Each case is driven from the specification (`ess specify compile --path spec --format json`)
//! or from a literal the specification states, never from `src/cli.rs`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command as Process, Stdio};

use clap::Parser;
use clap::error::ErrorKind;
use conductor_cli::cli::Cli;
use serde_json::Value;

fn compile() -> Value {
    let spec = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec");
    let output = Process::new("ess")
        .args(["specify", "compile", "--path"])
        .arg(&spec)
        .args(["--format", "json"])
        .output()
        .expect("`ess` runs");
    assert!(output.status.success(), "`ess specify compile` failed");
    serde_json::from_slice(&output.stdout).expect("`ess specify compile` prints JSON")
}

fn cli_block(model: &Value) -> Value {
    model["components"]
        .as_object()
        .expect("components")
        .values()
        .find(|component| !component["cli"].is_null())
        .expect("a component with a `cli:` block")["cli"]
        .clone()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_str().expect("a string").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Every placed leaf: (group, wire, qualified name, is a command).
fn leaves(model: &Value) -> Vec<(String, String, String, bool)> {
    let cli = cli_block(model);
    let mut out = Vec::new();
    for group in cli["groups"].as_array().expect("groups") {
        let name = group["name"].as_str().expect("group name").to_owned();
        for qualified in strings(&group["commands"]) {
            let wire = model["commands"][&qualified]["naming"]["wire"]
                .as_str()
                .expect("command wire")
                .to_owned();
            out.push((name.clone(), wire, qualified, true));
        }
        for qualified in strings(&group["views"]) {
            let wire = model["views"][&qualified]["naming"]["wire"]
                .as_str()
                .expect("view wire")
                .to_owned();
            out.push((name.clone(), wire, qualified, false));
        }
    }
    out
}

fn parse(argv: &[&str]) -> Result<(), ErrorKind> {
    Cli::try_parse_from(argv)
        .map(drop)
        .map_err(|error| error.kind())
}

// ---------------------------------------------------------------------------------------------
// 1. A flag accepts what its field's type holds: timestamps, UUIDs and prefixed identities.
// ---------------------------------------------------------------------------------------------

/// What a field's type admits, beyond "any text".
#[derive(Debug)]
enum Typed {
    /// `timestamp`: RFC 3339 (generated `primitives::Timestamp`).
    Timestamp,
    /// `uuid` (generated `primitives::Uuid`).
    Uuid,
    /// A newtype with `prefix:`: "Every value starts with `<prefix>`" (generated docs).
    Prefix(String),
    Other,
}

fn typed(model: &Value, type_ref: &Value) -> Typed {
    match type_ref["kind"].as_str() {
        Some("optional") => typed(model, &type_ref["of"]),
        Some("primitive") => match type_ref["name"].as_str() {
            Some("timestamp") => Typed::Timestamp,
            Some("uuid") => Typed::Uuid,
            _ => Typed::Other,
        },
        Some("declared") => {
            let name = type_ref["name"].as_str().expect("declared name");
            let body = &model["types"][name]["body"];
            if body["kind"].as_str() != Some("newtype") {
                return Typed::Other;
            }
            match body["prefix"].as_str() {
                Some(prefix) => Typed::Prefix(prefix.to_owned()),
                None => typed(model, &body["of"]),
            }
        }
        _ => Typed::Other,
    }
}

#[test]
fn adv_timestamp_flag_refuses_a_value_that_is_not_rfc3339() {
    // `conductor.observation.StartSnapshot.started_at` is `Timestamp`; its help says "RFC 3339".
    assert_eq!(
        parse(&[
            "conductor",
            "snapshot",
            "start-snapshot",
            "--started-at",
            "yesterday"
        ]),
        Err(ErrorKind::ValueValidation),
        "`--started-at yesterday` is not an RFC 3339 instant and must be refused at parse time"
    );
}

#[test]
fn adv_uuid_flag_refuses_a_value_that_is_not_a_uuid() {
    // `conductor.dispatch.ControllerId` is a newtype over `uuid`.
    assert_eq!(
        parse(&[
            "conductor",
            "controller",
            "pause-controller",
            "--controller-id",
            "not-a-uuid"
        ]),
        Err(ErrorKind::ValueValidation),
        "`--controller-id not-a-uuid` is not a UUID and must be refused at parse time"
    );
}

#[test]
fn adv_prefixed_id_flag_refuses_a_value_without_its_prefix() {
    // `conductor.dispatch.DispatchId` declares `prefix: DSP-`; the generated type documents
    // "Every value starts with `DSP-`".
    assert_eq!(
        parse(&[
            "conductor",
            "dispatch",
            "report-started",
            "--dispatch-id",
            "42"
        ]),
        Err(ErrorKind::ValueValidation),
        "`--dispatch-id 42` lacks the `DSP-` prefix and must be refused at parse time"
    );
}

#[test]
fn adv_every_typed_flag_refuses_values_outside_its_type() {
    let model = compile();
    let mut problems = Vec::new();
    let mut checked = 0;
    for (group, wire, qualified, is_command) in leaves(&model) {
        if !is_command {
            continue;
        }
        for field in model["commands"][&qualified]["input"]
            .as_array()
            .expect("input")
        {
            let name = field["name"].as_str().expect("field name");
            let flag = format!(
                "--{}",
                field["naming"]["wire"]
                    .as_str()
                    .map_or_else(|| name.replace('_', "-"), str::to_owned)
            );
            let (valid, invalid) = match typed(&model, &field["type_ref"]) {
                Typed::Timestamp => ("2026-10-06T17:00:00Z".to_owned(), "yesterday"),
                Typed::Uuid => (
                    "123e4567-e89b-12d3-a456-426614174000".to_owned(),
                    "not-a-uuid",
                ),
                Typed::Prefix(prefix) => (format!("{prefix}20261006-01"), "20261006-01"),
                Typed::Other => continue,
            };
            checked += 1;
            let base = ["conductor", group.as_str(), wire.as_str(), flag.as_str()];
            let with = |value: &str| {
                let mut argv = base.to_vec();
                argv.push(value);
                parse(&argv)
            };
            if let Err(kind) = with(&valid) {
                problems.push(format!(
                    "`conductor {group} {wire} {flag} {valid}` is refused ({kind:?})"
                ));
            }
            match with(invalid) {
                Err(ErrorKind::InvalidValue | ErrorKind::ValueValidation) => {}
                other => problems.push(format!(
                    "`conductor {group} {wire} {flag} {invalid}` gives {other:?}; \
                     `{qualified}.{name}` does not hold that value"
                )),
            }
        }
    }
    assert!(checked > 0, "no typed field was checked");
    assert!(
        problems.is_empty(),
        "{} of {checked} typed flags accept a value outside their type:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------------------------
// 2. Every leaf whose story has not landed answers "not implemented" with exit status 2, through
//    its own handler.
// ---------------------------------------------------------------------------------------------

/// The leaves whose handlers are written, as `group wire`. Each is decided by its own story's
/// tests: the `snapshot` leaves by `tests/snapshot.rs` (`story:snapshot-driver`), the other `goal`
/// leaves and the `resource` leaves by `tests/goal_resource.rs`
/// (`story:goal-and-resource-commands`), the `controller` leaves by `tests/controller.rs`
/// (`story:controller-commands`), `goal goals` by `tests/state_dir.rs` (`story:state-dir`),
/// `guard guard-decisions` by `tests/guard_hook.rs` (`story:guard-hook`), the `decision` leaves by
/// `tests/decision.rs` and the `dispatch` and `message` leaves by `tests/dispatch.rs`
/// (`story:decision-commands`). In the order the leaves are placed.
const IMPLEMENTED: [&str; 69] = [
    "snapshot complete-snapshot",
    "snapshot fail-snapshot",
    "snapshot record-blocker",
    "snapshot record-merged-pull-request",
    "snapshot record-pull-request",
    "snapshot record-release",
    "snapshot record-repository",
    "snapshot record-session",
    "snapshot record-specification",
    "snapshot record-workflow-run",
    "snapshot start-snapshot",
    "snapshot blockers",
    "snapshot merged-pull-requests",
    "snapshot pull-requests",
    "snapshot releases",
    "snapshot repositories",
    "snapshot sessions",
    "snapshot snapshots",
    "snapshot specifications",
    "snapshot workflow-runs",
    "goal add-serving",
    "goal confirm-goal",
    "goal drop-goal",
    "goal mark-goal-met",
    "goal propose-goal",
    "goal remove-serving",
    "goal servings",
    "goal goals",
    "repository activate-repository",
    "repository deactivate-repository",
    "repository mark-repository",
    "repository repository-marks",
    "decision answer-escalated-request",
    "decision answer-request",
    "decision escalate-request",
    "decision raise-decision-request",
    "decision record-conductor-decision",
    "decision record-operator-decision",
    "decision reverse-conductor-decision",
    "decision reverse-operator-decision",
    "decision withdraw-request",
    "decision decisions",
    "decision hands-todo",
    "decision requests",
    "controller pause-controller",
    "controller resume-controller",
    "controller revise-charter",
    "controller start-controller",
    "controller stop-controller",
    "controller controllers",
    "dispatch cancel-dispatch",
    "dispatch report-blocked",
    "dispatch report-done",
    "dispatch report-failed",
    "dispatch report-started",
    "dispatch report-unblocked",
    "dispatch send-dispatch",
    "dispatch dispatches",
    "message handle-message",
    "message receive-message",
    "message reject-message",
    "message route-need",
    "message messages",
    "resource grant-resource",
    "resource refuse-resource",
    "resource release-resource",
    "resource request-resource",
    "resource resource-requests",
    "guard guard-decisions",
];

#[test]
fn adv_every_leaf_answers_not_implemented_with_exit_2() {
    let model = compile();
    // A leaf that opens the store creates `state/` under its working directory, and `state/` is
    // ignored at any depth: run every leaf from this target's temporary directory, never from
    // the crate directory cargo starts tests in.
    let cwd = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("adv_cli");
    fs::create_dir_all(&cwd).expect("create the leaves' working directory");
    let mut problems = Vec::new();
    let mut ran = 0;
    let mut skipped = Vec::new();
    for (group, wire, qualified, _) in leaves(&model) {
        let path = format!("{group} {wire}");
        if IMPLEMENTED.contains(&path.as_str()) {
            skipped.push(path);
            continue;
        }
        let output = Process::new(env!("CARGO_BIN_EXE_conductor"))
            .args([group.as_str(), wire.as_str()])
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        ran += 1;
        let stderr = String::from_utf8_lossy(&output.stderr);
        let expected = format!("conductor: {group} {wire}: not implemented\n");
        if output.status.code() != Some(2) || stderr != expected || !output.stdout.is_empty() {
            problems.push(format!(
                "`conductor {group} {wire}` ({qualified}) exited {:?} with stderr {stderr:?}; \
                 expected exit 2 and {expected:?}",
                output.status.code()
            ));
        }
    }
    assert!(ran > 0, "no leaf ran");
    assert_eq!(
        skipped, IMPLEMENTED,
        "every leaf listed as implemented is a placed leaf of the cli block"
    );
    assert!(
        problems.is_empty(),
        "{} of {ran} leaves do not answer through their own stub:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------------------------
// 3. No word outside the `cli:` block parses: no catch-all, no inferred prefix, no `help` word.
// ---------------------------------------------------------------------------------------------

#[test]
fn adv_no_word_outside_the_cli_block_parses() {
    let model = compile();
    let cli = cli_block(&model);
    let mut problems = Vec::new();
    let mut expect = |argv: &[&str], wanted: ErrorKind| {
        let got = parse(argv);
        if got != Err(wanted) {
            problems.push(format!(
                "`{}` gives {got:?}, expected Err({wanted:?})",
                argv.join(" ")
            ));
        }
    };
    expect(
        &["conductor", "no-such-group"],
        ErrorKind::InvalidSubcommand,
    );
    expect(&["conductor", "help"], ErrorKind::InvalidSubcommand);
    expect(
        &["conductor", "snap", "start-snapshot"],
        ErrorKind::InvalidSubcommand,
    );
    expect(
        &[
            "conductor",
            "snapshot",
            "start-snapshot",
            "--started",
            "2026-10-06T17:00:00Z",
        ],
        ErrorKind::UnknownArgument,
    );
    expect(
        &["conductor", "snapshot", "start-snapshot", "positional"],
        ErrorKind::UnknownArgument,
    );
    for group in cli["groups"].as_array().expect("groups") {
        let name = group["name"].as_str().expect("group name");
        expect(
            &["conductor", name, "no-such-command"],
            ErrorKind::InvalidSubcommand,
        );
        expect(&["conductor", name, "help"], ErrorKind::InvalidSubcommand);
    }
    assert!(
        problems.is_empty(),
        "{} word(s) outside the cli block parse:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}
