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

/// `story:rules-section-names`: conductor reads the shared section at start only when the file
/// has it; a refusal of it (which names the sections the file has) is skipped, not a failed start.
#[test]
fn conductor_reads_the_all_sessions_section_only_when_the_file_has_it() {
    let text = profile("conductor.md");
    assert!(
        text.contains(
            "and, when the file has that section, `conductor rules show --section \"all \
             sessions\" --brief` (the rules every session follows; when it is refused, skip it: \
             the refusal names the sections the file has)"
        ),
        "conductor.md: the shared section is read only when the file has it"
    );
}

/// `story:controller-trees-by-path`: a controller works in a managed tree by its absolute path,
/// never through `EnterWorktree`, which the guard denies (`guard_rules.rs` holds the denial).
#[test]
fn a_controller_works_in_a_managed_tree_by_its_absolute_path() {
    let text = profile("repo-controller.md");
    assert!(
        text.contains(
            "Work in a managed tree by its absolute path, never through `EnterWorktree`, which \
             the guard denies: file tools name `<tree>/…`, and commands run as `cd <tree> && …` \
             or `git -C <tree> …`."
        ),
        "repo-controller.md: managed trees by absolute path, never EnterWorktree"
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

/// `story:hand-item-dry-run`: an operator script is dry-run before it is handed over, and the
/// class-H item carries what the dry run printed or why there is none.
#[test]
fn an_operator_script_is_dry_run_before_it_is_handed_over() {
    let text = profile("conductor.md");
    assert!(
        text.contains("An operator script is dry-run before it is handed over."),
        "conductor.md: an operator script is dry-run first"
    );
    assert!(
        text.contains("`--dry-run <file>`") && text.contains("`--no-dry-run <reason>`"),
        "conductor.md: the item carries the dry run or why there is none"
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

/// `story:decisions-notify`: conductor sends the operator a push notification on each escalation
/// line and each unanswered line the watch prints.
#[test]
fn conductor_pushes_a_notification_on_each_escalated_and_unanswered_line() {
    let text = profile("conductor.md");
    for line in [
        "`escalated <O|H> <request-id> <question>`",
        "`unanswered <O|H> <request-id> <minutes>`",
    ] {
        assert!(
            text.contains(line),
            "conductor.md: names the watch line {line}"
        );
    }
    assert!(
        text.contains(
            "On each `escalated` or `unanswered` line, send the operator a `PushNotification` \
             naming the request id, its class and its question or how long it has waited."
        ),
        "conductor.md: each escalated and unanswered line is pushed to the operator"
    );
}

/// `story:decisions-page`: conductor gives every escalation what the Decisions page shows, by
/// the flags `decision escalate-request` reads, and knows the watch's line at a deadline.
#[test]
fn conductor_gives_every_escalation_its_kind_options_costs_recommendation_default_and_deadline() {
    let text = profile("conductor.md");
    assert!(
        text.contains(
            "Give every escalation its kind, options, costs, recommendation, default and \
             deadline"
        ),
        "conductor.md: every escalation names what the operator chooses from"
    );
    for flag in [
        "`--kind`",
        "`--lettered-options`",
        "`--recommended-letters`",
        "`--default-letters`",
        "`--deadline`",
    ] {
        assert!(text.contains(flag), "conductor.md: names {flag}");
    }
    assert!(
        text.contains("or at its deadline"),
        "conductor.md: the watch's unanswered line at a deadline"
    );
}

/// `story:request-amend-escalated`: conductor completes an escalated item in place with
/// `decision amend-escalated-request`, not by sending it again as text.
#[test]
fn conductor_completes_an_escalated_item_with_amend_escalated_request() {
    let text = profile("conductor.md");
    assert!(
        text.contains(
            "Complete an escalated item with `conductor decision amend-escalated-request \
             --request-id <id> --reason <r>`, not by text"
        ),
        "conductor.md: an escalated item is completed with the command"
    );
    for flag in [
        "`--lettered-options`",
        "`--recommended-letters`",
        "`--default-letters`",
        "`--deadline`",
        "`--dry-run <text>`",
    ] {
        assert!(text.contains(flag), "conductor.md: names {flag}");
    }
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

/// `story:disk-escalation`: conductor grants no build slot by message; it reads the host ledger
/// and answers a disk or build `[RESOURCE …]` with the command that reserves one, and a `rules.md`
/// rule to free disk is carried by the protocol.
#[test]
fn conductor_grants_no_build_slot_by_message_and_reads_the_host_ledger() {
    let text = profile("conductor.md");
    for sentence in [
        "Build slots are no longer granted by message.",
        "Every build, test or gate runs as `conductor build run --reserve <size> -- <command>`",
        "Read the live reservations with `conductor host grants`.",
        "Answer a `[RESOURCE <repo>] disk` or `[RESOURCE <repo>] build` with \"use conductor build \
         run\", and grant nothing.",
        "A `rules.md` rule that asks you to free disk is carried by this protocol: the binary \
         reclaims (`conductor host reclaim`), and a shortfall it cannot cover is one operator item \
         in `decision hands-todo`",
    ] {
        assert!(text.contains(sentence), "conductor.md: holds {sentence:?}");
    }
    assert!(
        !text.contains("`[RESOURCE <repo>] build-slot` is refused with the number"),
        "conductor.md: the build slot granted by message is gone"
    );
}

/// `story:disk-escalation` and `story:guard-requires-build-run`: a controller and conductor-dev
/// run every build, test or gate under `conductor build run`, and report a refusal instead of
/// retrying it.
#[test]
fn every_build_runs_under_conductor_build_run_and_a_refusal_is_reported() {
    for name in ["repo-controller.md", "conductor-dev.md"] {
        let text = profile(name);
        for sentence in [
            "build, test or gate runs as `conductor build run --reserve <size> -- <command>`",
            "refusal names the shortfall and is reported, never retried in a loop",
        ] {
            assert!(text.contains(sentence), "{name}: holds {sentence:?}");
        }
    }
    let controller = profile("repo-controller.md");
    for sentence in [
        "The guard denies a plain `cargo build|test|check|clippy|nextest|install|run`, \
         `task check|gate|install|ui|regen|docs-generate`, `npm run build|test` or `npm ci`, and \
         its denial names the form to use; `cargo fmt` and `rustfmt` run as they are.",
        "Disk and build slots are not asked for by message: `conductor build run` reserves them",
    ] {
        assert!(
            controller.contains(sentence),
            "repo-controller.md: holds {sentence:?}"
        );
    }
    assert!(
        !controller.contains("kill -STOP"),
        "repo-controller.md: the hand-written watchdog is gone"
    );
}

/// `story:rules-show`: a worker brief cites the `rules.md` items it carries with the output of
/// `conductor rules show --item …`, never with an excerpt cut by hand from the file.
#[test]
fn a_worker_brief_cites_rules_items_through_rules_show() {
    let text = profile("repo-controller.md");
    let sentence = "A worker brief carries the `rules.md` items it cites as the output of \
                    `conductor rules show --item <n>,<n>,…`, pasted whole, never an excerpt cut \
                    from the file by hand (`awk`, `sed` or line numbers); after a `rules.md` \
                    change, run it again for each running worker's items.";
    assert!(
        text.contains(sentence),
        "repo-controller.md: holds {sentence:?}"
    );
}

/// The design, `docs/design/conductor.md`, on single spaces.
fn design() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/conductor.md");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `story:conductor-start-reading`: a `rules.md` change reaches a running conductor as a relayed
/// diff and no restart; a restart is for the profile, the adapter, the controller profile or the
/// design, batched with the next wave and at most one an hour; conductor starts on design §§ 4-8
/// and its own section of `rules.md` only; and when conductor-dev is unreachable, conductor
/// restarts itself.
#[test]
fn conductor_restarts_rarely_and_starts_on_less_reading() {
    let design = design();
    for sentence in [
        "A `rules.md` change reaches a running conductor as a relayed diff, like the \
         controllers', and a restart is only for the profile, the adapter, the controller \
         profile or the design.",
        "Restarts are batched with the next wave, at most one an hour unless one corrects a \
         live fault.",
    ] {
        assert!(design.contains(sentence), "design § 12: holds {sentence:?}");
    }

    let dev = profile("conductor-dev.md");
    for sentence in [
        "Restarts are batched with the next wave, at most one an hour unless one corrects a \
         live fault.",
        "A `rules.md` change reaches a running conductor as a relayed diff, like the \
         controllers', and needs no restart:",
    ] {
        assert!(
            dev.contains(sentence),
            "conductor-dev.md: holds {sentence:?}"
        );
    }
    assert!(
        !dev.contains("`rules.md`, design §§ 2–12 or the conductor role's `settings`"),
        "conductor-dev.md: a rules.md change no longer restarts conductor"
    );

    let conductor = profile("conductor.md");
    for sentence in [
        "At session start read, in this order, and no file twice:",
        "the titles of the rules for your role: `conductor rules show --section conductor \
         --brief` and, when the file has that section, `conductor rules show --section \"all \
         sessions\" --brief` (the rules every session follows; when it is refused, skip it: the \
         refusal names the sections the file has). Not the whole `rules.md`: read an item whole \
         with `conductor rules show --item <n>,…` when a cycle needs it;",
        "the newest hand-over, the one committed last",
        "only the sections the hand-over points to of §§ 4–8 of `docs/design/conductor.md` in \
         the product repository, `NORTHSTAR.md` and the newest `STATUS.md`, and no design \
         section outside §§ 4–8; with no hand-over, those three in full;",
        "the lines of `decisions/` and `dispatches/` whose `at` is later than the hand-over's \
         commit time, not the last 50 lines.",
        "When a restart you need cannot be answered because conductor-dev is unreachable",
        "write your hand-over as for `[RESTART conductor]`, commit it, and run `task \
         conductor:restart` yourself",
        "A `rules.md` change reaches you as a relayed diff, not a restart:",
    ] {
        assert!(
            conductor.contains(sentence),
            "conductor.md: holds {sentence:?}"
        );
    }
    assert!(
        !conductor.contains("§§ 2–12 of `docs/design/conductor.md`"),
        "conductor.md: the start reads design §§ 4-8 only"
    );
    // `story:conductor-context-budget`: the start reads rule titles, not the rules in full, and
    // the log since the hand-over, not a fixed tail of it.
    for gone in [
        "the last 50 lines of `decisions/` and `dispatches/`",
        "`conductor rules show --section conductor` and",
    ] {
        assert!(
            !conductor.contains(gone),
            "conductor.md: no longer holds {gone:?}"
        );
    }
}

/// `story:conductor-dev-report-form`: conductor-dev answers a NEED in the report form conductor
/// parses, the NEED's id in the line and `-` in the dispatch-id slot.
#[test]
fn conductor_dev_answers_a_need_in_the_report_form() {
    let dev = profile("conductor-dev.md");
    let sentence = "Answer each NEED as `[REPORT conductor -] \
                    <started|progress|blocked|unblocked|done|failed> <need id>: <one line>`: the \
                    dispatch-id slot is `-`, the NEED's id goes in the line.";
    assert!(
        dev.contains(sentence),
        "conductor-dev.md: holds {sentence:?}"
    );
    assert!(
        !dev.contains("[REPORT conductor-dev"),
        "conductor-dev.md: no report names conductor-dev as its repository"
    );
}

/// `story:need-id-unique`: conductor sends a NEED only with an id `conductor need new` printed, so
/// no two NEEDs share one.
#[test]
fn conductor_sends_a_need_only_with_a_minted_id() {
    let conductor = profile("conductor.md");
    let sentence = "Send a `[NEED conductor <id>]` only with an id `conductor need new` printed, \
                    run for that NEED; never write an id by hand or reuse one.";
    assert!(
        conductor.contains(sentence),
        "conductor.md: holds {sentence:?}"
    );
}

/// Waves are wide and concurrent (the operator, 2026-10-14): every ready unit with disjoint scope,
/// all implementors started at once, the next wave planned while a gate runs.
#[test]
fn a_controller_runs_wide_concurrent_waves() {
    let text = profile("repo-controller.md");
    for sentence in [
        "A wave takes every ready unit with disjoint scope, up to `controllers.max_subagents`; a \
         narrower wave records its reason on its wave page, as a line `Width: <reason>`.",
        "All of a wave's implementors start in one message, as background sub-agents,",
        "The next wave is planned and started while a gate or a CI run runs.",
        "Adversary and security reviews run alongside the building units, not after them.",
    ] {
        assert!(
            text.contains(sentence),
            "repo-controller.md: holds {sentence:?}"
        );
    }
}

/// `story:decision-reads-charter`: a decision about a repository is checked against its charter's
/// "Do not touch" lines first, which `record-conductor-decision --repository` prints.
#[test]
fn conductor_checks_a_decision_against_the_charter_s_do_not_touch_lines() {
    let text = profile("conductor.md");
    let sentence = "A decision about a repository is checked against its charter's \"Do not \
                    touch\" lines first:";
    assert!(text.contains(sentence), "conductor.md: holds {sentence:?}");
}

/// `story:profile-zsh-shell`: the sessions' shell may be zsh, where an unquoted word that starts
/// with `=` expands as a command path and an unmatched glob aborts the command.
#[test]
fn conductor_and_controllers_quote_for_zsh() {
    let sentence = "The shell may be zsh: quote a word that starts with `=`, and guard a glob that \
                    may match nothing (`(N)` or an `ls` test).";
    for name in ["conductor.md", "repo-controller.md"] {
        assert!(
            profile(name).contains(sentence),
            "{name}: holds {sentence:?}"
        );
    }
}

/// `story:conductor-no-poll`: conductor never waits inside its own session.
#[test]
fn conductor_never_sleeps_or_polls_in_its_session() {
    let text = profile("conductor.md");
    let sentence = "Never sleep or poll on a pull request, a suite or a CI run in your session: \
                    your watch or the controller's report wakes you.";
    assert!(text.contains(sentence), "conductor.md: holds {sentence:?}");
}

/// `story:status-command`: conductor answers an operator's status question with `conductor
/// status`, one call, rather than assembling it from the views.
#[test]
fn conductor_answers_a_status_question_with_conductor_status() {
    let text = profile("conductor.md");
    assert!(
        text.contains(
            "Answer an operator's status question with `conductor status`: the newest complete \
             snapshot and its age, its deviations, live sessions, open dispatches, registered \
             waits and pending requests, read from the store in one call."
        ),
        "conductor.md: a status question is answered with `conductor status`"
    );
    assert!(
        !text.contains("`conductor status` once it exists"),
        "conductor.md: `conductor status` exists"
    );
}

/// A class-H call runs unattended only when `rules.md` names it as the operator's grant.
#[test]
fn a_class_h_call_runs_only_under_a_named_grant() {
    let text = profile("conductor.md");
    let sentence = "The one exception is a command the instance's `rules.md` names as granted by the \
                    operator for unattended runs, quoting them and stating its scope: run exactly \
                    that command within that scope, nothing wider, and report only its failure.";
    assert!(text.contains(sentence), "conductor.md: holds {sentence:?}");
}

/// `story:session-envelope-seam`: conductor's profile starts no session with `claude --bg`;
/// `conductor spawn` and `conductor session start` are the only start forms it names.
#[test]
fn conductor_starts_sessions_only_through_spawn_and_session_start() {
    let text = profile("conductor.md");
    assert!(
        !text.contains("claude --bg"),
        "conductor.md: no session starts with claude --bg"
    );
    assert!(
        text.contains(
            "`conductor spawn --repository <repo>` and `conductor session start --role <role>` \
             are the only start forms of a session"
        ),
        "conductor.md: spawn and session start are the only start forms"
    );
}

/// `story:policy-secret-drift`: a stale secret is published by its sync's own command only under
/// a grant in `rules.md`; otherwise it is the operator's hands, handed over with the dry run.
#[test]
fn conductor_publishes_a_stale_secret_only_under_a_grant_or_hands_it_over() {
    let text = profile("conductor.md");
    let sentence = "On a board row `secret stale: <sync>`, run that `secret_syncs` entry's \
                    `command` when `rules.md` grants it; else hand it over as class H with the \
                    output of its dry run (`dry_run`)";
    assert!(text.contains(sentence), "conductor.md: holds {sentence:?}");
}

/// `story:pr-check-names`: a report on a pull request names every red check, not one of them.
#[test]
fn a_report_on_a_pull_request_names_every_red_check() {
    let text = profile("conductor.md");
    let sentence = "A report on a pull request names every red check, as `conductor status` and the \
                    board list them under `failing checks`, never one of them for all.";
    assert!(text.contains(sentence), "conductor.md: holds {sentence:?}");
}

/// `story:instance-isolation`, item 7: the section on other instances applies only to an instance
/// that names no `conductor` role, which the `default` instance's conductor serves; an instance
/// with its own conductor is never acted for by another, so no conductor is a second writer of
/// its records.
#[test]
fn conductor_acts_for_another_instance_only_when_it_names_no_conductor() {
    let text = profile("conductor.md");
    let sentence = "This section applies only to an instance that names no `conductor` role, which \
                    your instance serves as the file's `default` (`conductor.served_by` in \
                    `conductor config show` names it). An instance that names a `conductor` role \
                    has its own conductor: never run a command for it, write its records, \
                    dispatch to its controllers or message its sessions.";
    assert!(text.contains(sentence), "conductor.md: holds {sentence:?}");
}
