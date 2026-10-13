//! `story:profiles-from-review`: the generic profiles carry what a review of a live instance's
//! sessions found missing. Each case reads a profile for the sentence that holds a rule, so a
//! rewrite that drops the rule fails here.

use std::fs;
use std::path::PathBuf;

fn profile(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.agents")
        .join(name);
    let text = fs::read_to_string(&path).expect("read the profile");
    // Profiles wrap at 100 columns; compare on single spaces.
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn conductor_dispatches_cleanup_below_disk_low_and_never_asks_the_operator() {
    let text = profile("conductor.md");
    assert!(
        text.contains("dispatches cleanup (class C: dispatch it and report what was freed; never ask the operator to clear the instance's own trees, archives or caches)"),
        "conductor.md: cleanup below disk_low is class C"
    );
}

#[test]
fn conductor_relays_every_rules_commit_and_decisions_carry_their_id() {
    let text = profile("conductor.md");
    assert!(
        text.contains("`[DECISION …]` always carries the decision id it rests on"),
        "conductor.md: a DECISION carries its id"
    );
    assert!(
        text.contains("Relay every `rules.md` commit at once to every running controller"),
        "conductor.md: rules commits are relayed"
    );
}

#[test]
fn conductor_reads_a_redacted_session_row_as_by_design() {
    let text = profile("conductor.md");
    assert!(
        text.contains("redacted by design, not a defect"),
        "conductor.md: redacted session rows"
    );
}

/// `story:instance-session-names` and `story:controller-report-sendmessage`: a controller's
/// messages go to its instance's conductor by the session name `conductor config show` prints, and
/// every report is a `SendMessage` call, since text printed in the session reaches nobody.
#[test]
fn a_controller_messages_its_instance_s_conductor_by_send_message() {
    let text = profile("repo-controller.md");
    assert!(
        text.contains(
            "Messages go only to `<session_prefix>-conductor`, your instance's conductor: the \
             session name `conductor config show` prints as `conductor.session_name` \
             (`conductor` when the instance has no `session_prefix`). When your instance names \
             no `conductor` role, `conductor.served_by` names the `default` instance whose \
             conductor serves it, and `conductor.session_name` is that conductor's."
        ),
        "repo-controller.md: messages go to the instance's conductor"
    );
    assert!(
        text.contains(
            "Every `[REPORT]`, `[DECISION-REQUEST]`, `[NEED]` and `[RESOURCE]` is a `SendMessage` \
             call to that conductor: text printed in the session reaches nobody, and a turn that \
             ends on a report ends with that call."
        ),
        "repo-controller.md: every report is a SendMessage call"
    );
    assert!(
        !text.contains("Messages go only to `conductor`"),
        "repo-controller.md: the bare name is gone"
    );
}

#[test]
fn a_controller_reads_the_sections_for_all_sessions_and_reports_pr_ci_and_merge() {
    let text = profile("repo-controller.md");
    assert!(
        text.contains("follow its controller section and every section it marks for all sessions or for controllers"),
        "repo-controller.md: reads the shared sections"
    );
    assert!(
        text.contains("when a pull request opens, when its CI turns red, and on merge"),
        "repo-controller.md: reports PR, CI red and merge"
    );
}

/// `story:resumable-gates`: a gate over an hour reruns only what failed and what depends on it,
/// and one full run stays for the release commit; the controller profile and the charters
/// conductor writes both say so.
#[test]
fn a_long_gate_reruns_only_the_failed_steps_and_keeps_one_full_run_for_the_release() {
    let controller = profile("repo-controller.md");
    assert!(
        controller.contains(
            "A gate that runs over an hour resumes: after a failure, rerun only the failed steps \
             and the steps that depend on them, never the whole suite again. Keep one full run \
             for the release commit."
        ),
        "repo-controller.md: a long gate reruns only what failed"
    );
    let conductor = profile("conductor.md");
    assert!(
        conductor.contains(
            "A charter for a repository whose gate runs over an hour says: after a failure, rerun \
             only the failed steps and the steps that depend on them; keep one full run for the \
             release commit."
        ),
        "conductor.md: charters carry the resumable-gate rule"
    );
}

/// `story:work-beside-long-gate`: while a gate expected to run over an hour runs, conductor
/// dispatches the repository's next wave units beside it and the controller proposes them
/// instead of waiting.
#[test]
fn next_units_run_beside_a_gate_expected_to_run_over_an_hour() {
    let conductor = profile("conductor.md");
    assert!(
        conductor.contains(
            "While a controller's gate is expected to run over an hour, dispatch the repository's \
             next wave units beside it, in a separate tree, within the build-slot budget \
             (`thresholds.build_slot`)."
        ),
        "conductor.md: dispatch next units beside a long gate"
    );
    let controller = profile("repo-controller.md");
    assert!(
        controller.contains(
            "While a gate expected to run over an hour runs, propose your next wave units to \
             conductor and work on them in a separate tree, within the build-slot budget, instead \
             of waiting for it."
        ),
        "repo-controller.md: propose work beside a long gate"
    );
}

/// `story:claims-cite-source`: a news or status claim names the source it was read from and is
/// checked against it before it is sent.
#[test]
fn a_news_or_status_claim_cites_its_source_and_is_checked_against_it() {
    let text = profile("conductor.md");
    assert!(
        text.contains(
            "Every news or status claim about a release, an issue or a gate, in a report or the \
             daily brief, cites the source it was read from (the release page, the issue, the \
             runner's exit line) and is checked against that source before it is sent."
        ),
        "conductor.md: claims cite their source"
    );
}

/// `story:operator-items-age`: an operator item older than a day is re-sent with the default
/// conductor will take and its deadline, and the default is taken at the deadline when the step
/// can be undone.
#[test]
fn an_old_operator_item_is_re_sent_with_a_default_and_a_deadline() {
    let text = profile("conductor.md");
    assert!(
        text.contains("An operator item older than a day"),
        "conductor.md: the re-send applies after a day"
    );
    assert!(
        text.contains("is re-sent with the default you will take and its deadline."),
        "conductor.md: an old operator item is re-sent with its default and deadline"
    );
    assert!(
        text.contains("At the deadline, take that default when the step can be undone"),
        "conductor.md: the default is taken at the deadline"
    );
}

/// `story:gc-before-pause`: conductor runs `conductor gc`, which removes, before it pauses work
/// for disk.
#[test]
fn conductor_runs_gc_before_it_pauses_work_for_disk() {
    let text = profile("conductor.md");
    assert!(
        text.contains(
            "before you pause work for disk, run `conductor gc` (without `--dry-run` it removes \
             and strips archive build output)"
        ),
        "conductor.md: gc runs before work pauses for disk"
    );
}
