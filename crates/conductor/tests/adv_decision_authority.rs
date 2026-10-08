//! Adversary, wave 06 U1 (`story:decision-commands`, the decision group), pass 1: who may run the
//! operator's commands.
//!
//! `spec/domains/decision.yaml` grants `RecordOperatorDecision`, `AnswerEscalatedRequest` and
//! `ReverseOperatorDecision` to the `Operator` actor alone, and binds "who decided" to the command
//! rather than to an input field (the hardening of 2026-10-06: "an input `decided_by` let any
//! caller claim to be the operator"). Design § 5: "The operator records what was done
//! with `record-operator-decision`, class H"; "An escalated request is answered only by the
//! operator (`decision answer-escalated-request`)".
//!
//! The binary takes no caller identity, so the grant holds only where a session is stopped from
//! running these commands, and `conductor guard` is the hook every conductor and controller tool
//! call runs. Before this unit the three commands exited 2; now they write. These cases put the
//! Bash calls that use them to the guard.

use std::fs;
use std::path::{Path, PathBuf};

use conductor_cli::guard;
use conductor_model::dispatch::Verdict;
use serde_json::json;

fn home(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_decision_authority")
        .join(case)
        .join("home");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's home");
    }
    for repository in ["conductor", "cortex"] {
        fs::create_dir_all(dir.join("example-org").join(repository)).expect("create the checkout");
    }
    dir
}

/// The guard's verdict on a Bash call `command` made by a session whose cwd is
/// `~/example-org/<repository>`.
fn bash(home: &Path, repository: &str, command: &str) -> guard::Decision {
    let payload = json!({
        "session_id": "00000000-0000-4000-8000-000000000000",
        "cwd": home.join("example-org").join(repository).display().to_string(),
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": command, "description": "record a decision"},
    });
    guard::decide(&payload, home)
}

const OPERATOR_COMMANDS: [&str; 3] = [
    "decision record-operator-decision --decision-class O --question q --options o --choice A \
     --reason r --evidence e --decided-at 2026-10-07T10:00:00Z",
    "decision answer-escalated-request --request-id cortex-1 --decision-id DEC-20261007-01",
    "decision reverse-operator-decision --decision-id DEC-20261007-01 --reason r",
];

/// A repository controller may raise and withdraw a request, nothing else
/// (`conductor.decision.RepositoryController`). Pointed at conductor's state directory, the
/// operator's commands write into conductor's store, attributed to the operator. The guard's Bash
/// rule for a controller looks at `cd`, `git -C`, `gh` and `.claude/settings` only, and
/// `state/` is Git-ignored, so the cycle's dirty-file check (design § 7) does not see it either.
#[test]
fn adv_a_controller_cannot_record_or_reverse_as_the_operator() {
    let home = home("controller");
    let state = home.join("example-org/conductor/state");
    let allowed: Vec<String> = OPERATOR_COMMANDS
        .iter()
        .map(|command| format!("conductor --state-dir {} {command}", state.display()))
        .filter(|command| bash(&home, "cortex", command).verdict == Verdict::Allow)
        .collect();
    assert_eq!(
        allowed,
        Vec::<String>::new(),
        "a controller's session ran the operator's commands against conductor's store"
    );
}

/// Conductor is the `Conductor` actor: `RecordConductorDecision`, `AnswerRequest`,
/// `EscalateRequest`, `ReverseConductorDecision`. `ReservedForOperator` and
/// `ReversalReservedForOperator` refuse it a class-O decision and the reversal of an operator's
/// one, and the same session then records the class-O decision as the operator's with
/// `record-operator-decision`, unrefused.
///
/// As adopted by wave 06 U7, this case asserts the opposite of the adversary's: the coordinator
/// decided that conductor's own session runs every leaf, the three operator commands included,
/// because it records the operator's decisions on his word, with his words in `evidence`, as it
/// does by hand today (design § 5; `~/.cache/conductor-waves/w06/u7/brief.md`, decision 1). So
/// the guard allows all three for conductor's session.
#[test]
fn adv_conductor_may_run_the_operators_commands_on_his_word() {
    let home = home("conductor");
    let denied: Vec<String> = OPERATOR_COMMANDS
        .iter()
        .map(|command| format!("conductor {command}"))
        .filter(|command| bash(&home, "conductor", command).verdict != Verdict::Allow)
        .collect();
    assert_eq!(
        denied,
        Vec::<String>::new(),
        "conductor's session records the operator's decisions on his word, so the guard allows \
         the operator's commands there"
    );
}
