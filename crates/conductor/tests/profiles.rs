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
