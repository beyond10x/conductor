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

/// `story:watch-disk-floors`: the watch reports each disk floor crossed, and conductor knows what
/// each line asks of it.
#[test]
fn conductor_acts_on_each_disk_floor_the_watch_reports() {
    let text = profile("conductor.md");
    for line in [
        "`disk low: <n>G free on /`",
        "`disk admit: <n>G free on / (builds wait)`",
        "`disk gate stop: <n>G free on / (gates stop)`",
    ] {
        assert!(
            text.contains(line),
            "conductor.md: names the watch line {line}"
        );
    }
    assert!(
        text.contains(
            "On `disk admit`, pause new builds: start nothing, grant nothing until free disk is \
             back at `thresholds.disk_admit`."
        ),
        "conductor.md: disk admit pauses new builds"
    );
    assert!(
        text.contains(
            "On `disk gate stop`, run `conductor gc` at once and tell every controller running a \
             gate that it is stopped until free disk is back at `thresholds.gate_resume`."
        ),
        "conductor.md: disk gate stop runs gc and tells the controllers"
    );
}

/// `story:self-arming-restart`: the restart prompt arms the jobs and the watch, so a hand-over
/// keeps only open work.
#[test]
fn a_hand_over_keeps_only_open_work_and_lists_no_jobs_or_watch() {
    let text = profile("conductor.md");
    assert!(
        text.contains(
            "A hand-over keeps only open work: it does not list the jobs or the watch, which the \
             start prompt of `task conductor:restart` names."
        ),
        "conductor.md: a hand-over lists no jobs or watch"
    );
}

/// A profile as written, line by line.
fn profile_lines(name: &str) -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.agents")
        .join(name);
    let text = fs::read_to_string(&path).expect("read the profile");
    text.lines().map(str::to_owned).collect()
}

/// How many commands of `program` the line names, each as a code span that is the program or
/// starts with it and a space.
fn named(line: &str, program: &str) -> usize {
    let alone = format!("`{program}`");
    let first = format!("`{program} ");
    line.matches(&alone).count() + line.matches(&first).count()
}

/// `story:profiles-forge-neutral`: each profile says what GitHub's words are on GitLab, and every
/// line of it that names a `gh` command names its `glab` counterpart on the same line.
#[test]
fn the_profiles_name_glab_beside_every_gh_command() {
    for name in ["conductor.md", "repo-controller.md"] {
        assert!(
            profile(name).contains(
                "On GitLab, a pull request is a merge request, `gh` is `glab` and a workflow run \
                 is a pipeline."
            ),
            "{name}: says what GitHub's words are on GitLab"
        );
        let mut with_gh = 0;
        for line in profile_lines(name) {
            let gh = named(&line, "gh");
            if gh == 0 {
                continue;
            }
            with_gh += 1;
            assert!(
                named(&line, "glab") >= gh,
                "{name}: the line names a `gh` command without its `glab` counterpart: {line}"
            );
        }
        assert!(with_gh > 0, "{name}: names no `gh` command");
    }
}

/// The sentence every profile carries for `story:spec-first-wording`.
const SPEC_FIRST: &str = "Before any change, check whether the repository has ESS specifications; \
                          if it does, modify them first and get them green (`ess specify validate` \
                          and the conformance suite they drive) before implementing.";

/// A profile on single spaces with each line's blockquote marker dropped, so a sentence quoted
/// across lines reads as it does in prose.
fn profile_unquoted(name: &str) -> String {
    profile_lines(name)
        .iter()
        .map(|line| {
            let line = line.trim_start();
            line.strip_prefix('>').unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// `story:spec-first-wording`: before any change, check for ESS specifications and get them green
/// first. Once in conductor-dev's profile, once in the paragraph conductor's `[DISPATCH …]` briefs
/// carry, and twice in the controller's: its own paragraph and the brief text conductor copies.
#[test]
fn every_profile_says_check_for_ess_specifications_and_get_them_green_first() {
    for (name, expected) in [
        ("conductor-dev.md", 1),
        ("conductor.md", 1),
        ("repo-controller.md", 2),
    ] {
        assert_eq!(
            profile_unquoted(name).matches(SPEC_FIRST).count(),
            expected,
            "{name}: the spec-first sentence"
        );
    }
}

/// A page of the documentation site on single spaces, read from `website/docs/<relative>`.
fn site_page(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../website/docs")
        .join(relative);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `story:records-from-commands`: once a command records an event, conductor writes no line for
/// it by hand, and the profile names the commands that write the records' logs.
#[test]
fn conductor_writes_no_hand_written_line_for_an_event_a_command_records() {
    let text = profile("conductor.md");
    assert!(
        text.contains("Once a command records an event, no hand-written line for it"),
        "conductor.md: no hand-written line for an event a command records"
    );
    for command in [
        "`dispatch send-dispatch`",
        "`report-*`",
        "`cancel-dispatch`",
        "`message receive-message`",
        "`resource grant-resource`",
        "`refuse-resource`",
        "`controller revise-charter`",
        "`decision record-conductor-decision`",
        "`raise-decision-request`",
        "`answer-request`",
        "`reverse-conductor-decision`",
    ] {
        assert!(
            text.contains(command),
            "conductor.md: names {command} among the commands that write the records' logs"
        );
    }
    assert!(
        !text.contains("Log each dispatch to `dispatches/YYYY-MM.jsonl`."),
        "conductor.md: the hand-logging instruction is gone"
    );
}

/// `story:review-limit`: conductor sends no new dispatch to a repository the board marks review
/// full, and reports it in the cycle instead.
#[test]
fn conductor_holds_dispatch_to_a_repository_whose_review_queue_is_full() {
    let text = profile("conductor.md");
    assert!(
        text.contains(
            "Send no new dispatch to a repository the board marks review full; report it in the \
             cycle instead."
        ),
        "conductor.md: a full review queue holds dispatch"
    );
    assert!(
        text.contains("`controllers.max_open_reviews`"),
        "conductor.md: names the limit's config key"
    );
}

/// `story:merge-human`: a charter's `merge: human` makes the controller open the pull request and
/// never merge it; conductor's charter section names the line and answers a request to merge.
#[test]
fn a_charter_s_merge_human_leaves_the_merge_to_a_person() {
    let controller = profile("repo-controller.md");
    for phrase in [
        "`merge: human`",
        "never merges",
        "never enables auto-merge",
        "`[REPORT <repo> <dispatch>] review <url>`",
        "the dispatch stays open until",
    ] {
        assert!(
            controller.contains(phrase),
            "repo-controller.md: the merge: human rule holds {phrase:?}"
        );
    }
    let conductor = profile("conductor.md");
    for phrase in [
        "`merge: human`",
        "`merge: controller`",
        "\"a person merges here\"",
    ] {
        assert!(
            conductor.contains(phrase),
            "conductor.md: the charter section holds {phrase:?}"
        );
    }
    assert!(
        site_page("guides/start-conductor.md").contains("`merge: human`"),
        "website/docs/guides/start-conductor.md: names `merge: human`"
    );
}

/// `story:worktree-activation`: a controller whose `worktree create` is refused for a workspace
/// that is not activated asks conductor and, until answered, works in a plain git worktree.
#[test]
fn a_controller_falls_back_to_a_plain_worktree_until_the_workspace_is_activated() {
    let text = profile("repo-controller.md");
    assert!(
        text.contains(
            "When `worktree create` refuses with `workspace-not-activated`, report `[NEED \
             conductor] worktree workspace not activated: <root>` and, until it is answered, work \
             in a plain `git worktree add <trees>/<repo>/<name>` tree, which you remove when done."
        ),
        "repo-controller.md: the worktree-activation fallback"
    );
}

/// `story:controller-without-store`: a controller delivers in a repository with no planning
/// store, offers one once in its `started` report and creates none itself.
#[test]
fn a_controller_delivers_in_a_repository_with_no_planning_store() {
    let text = profile("repo-controller.md");
    for phrase in [
        "no planning store",
        "does not create",
        "`[REPORT <repo> <dispatch-id>] started; no planning store: /aep:init would add one`",
        "the dispatch's brief is the plan, written into the pull request's description",
    ] {
        assert!(
            text.contains(phrase),
            "repo-controller.md: the no-store path holds {phrase:?}"
        );
    }
    assert!(
        site_page("concepts/roles.md")
            .contains("A repository needs no planning store to be dispatched to"),
        "website/docs/concepts/roles.md: a repository needs no planning store"
    );
}
