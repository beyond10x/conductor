//! `story:profiles-from-review`: the generic profiles carry what a review of a live instance's
//! sessions found missing. Each case reads a profile for the sentence that holds a rule, so a
//! rewrite that drops the rule fails here.
//!
//! `story:run-daemon` retired the conductor profile: `conductor run` does in code what it asked
//! of a session (`tests/run.rs`), so the cases that read it went with it.
//! `story:controller-profile-for-run` fits the two left to the daemon: a controller reports with
//! `conductor message send`, and each profile stays under its line cap.
//! `story:roles-in-the-binary` moved the profiles into the binary: `crates/conductor/roles/`,
//! one per role, with no Claude Code adapter committed beside them.

use std::fs;
use std::path::PathBuf;

/// The directory of the role profiles the binary carries, `crates/conductor/roles/`.
fn agents() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("roles")
}

fn profile(name: &str) -> String {
    let path = agents().join(name);
    let text = fs::read_to_string(&path).expect("read the profile");
    // Profiles wrap at 100 columns; compare on single spaces.
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every Markdown file of `dir`, relative to the repository root, with its text.
fn shipped(dir: &str) -> Vec<(PathBuf, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let entries =
        fs::read_dir(root.join(dir)).unwrap_or_else(|error| panic!("read {dir}: {error}"));
    let mut files = Vec::new();
    for entry in entries {
        let path = entry.expect("a profile").path();
        if path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read the profile");
        files.push((path, text));
    }
    files
}

/// The host's profiles, beside the instance roles' (`story:host-session-profiles`), and the host
/// role each one is (`story:host-roles-by-daemon`).
const HOST_PROFILES: [(&str, &str); 2] =
    [("supervisor.md", "supervisor"), ("teacher.md", "teacher")];

/// `story:host-roles-by-daemon`: the supervisor's and the teacher's profiles ship in the binary,
/// each stating that the daemon starts it from `globals.roles` through `conductor session start
/// --role <role>` and that its task asks the daemon, with no `claude --bg` start command. No
/// adapter is committed for either (`story:roles-in-the-binary`): the `claude-code` kind writes
/// the profile into the agent's state at start, and each profile names where it ships, not a copy
/// on the host. Replaces the case of `story:host-session-profiles`, whose profiles wrote a
/// `claude --bg --agent <name>` start command.
#[test]
fn the_host_profiles_state_the_daemon_starts_them_from_the_binary() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (file, role) in HOST_PROFILES {
        let name = format!("conductor-{role}");
        let text = fs::read_to_string(agents().join(file)).expect("read the profile");
        for stated in [
            format!("`conductor session start --role {role}`"),
            "`globals.roles`".to_owned(),
            format!("`conductor daemon start-role --role {role}`"),
            "`conductor_host.slice`".to_owned(),
        ] {
            assert!(text.contains(&stated), "{file} states {stated}");
        }
        assert!(
            !text.contains(&format!("`claude --bg --agent {name} ")),
            "{file} still writes a claude --bg start command"
        );
        assert!(
            !root
                .join(".claude/agents")
                .join(format!("{name}.md"))
                .exists(),
            "no adapter is committed for the {role}"
        );
        assert!(
            text.contains(&format!("`crates/conductor/roles/{file}`"))
                && text.contains(&format!("`conductor role show --role {role}`")),
            "{file} names where it ships and how to print it"
        );
        for gone in [
            "task profiles",
            "`profiles/",
            ".agents/",
            "~/.claude/agents/",
        ] {
            assert!(
                !text.contains(gone),
                "{file} still names {gone:?}, a copy on the host"
            );
        }
    }
}

/// `conductor run` is the loop, so the conductor profile and its Claude Code adapter are retired,
/// and no profile or adapter shipped asks a session to arm a timer, run the watch under `Monitor`,
/// message another role with `SendMessage`, or restart conductor.
#[test]
fn no_profile_arms_a_timer_watches_messages_by_send_message_or_restarts_conductor() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for retired in [".agents/conductor.md", ".claude/agents/conductor.md"] {
        assert!(!root.join(retired).exists(), "{retired} is retired");
    }
    let profiles = shipped("crates/conductor/roles");
    assert_eq!(
        profiles.len(),
        4,
        "the profiles shipped: maintainer.md, controller.md and the host's supervisor.md and \
         teacher.md"
    );
    assert!(
        !root.join(".claude").exists(),
        "no Claude Code adapter is committed (story:roles-in-the-binary)"
    );
    for (path, text) in &profiles {
        // The host's supervisor and teacher run outside every instance, where no mailbox of
        // conductor run's reaches them: they message each other and the maintainer by
        // `SendMessage` (`story:host-session-profiles`). The instance roles never do.
        let host = HOST_PROFILES
            .iter()
            .any(|(name, _)| path.file_name().is_some_and(|file| file == *name));
        for gone in [
            "CronCreate",
            "Monitor",
            "SendMessage",
            "watch run --follow",
            "[RESTART conductor]",
            "conductor:restart",
            "conductor:start",
        ] {
            if host && gone == "SendMessage" {
                continue;
            }
            assert!(
                !text.contains(gone),
                "{}: names {gone:?}, which conductor run replaced",
                path.display()
            );
        }
    }
}

/// `story:roles-in-the-binary`: a harness permission prompt is refused with `Escape` and the pane
/// checked; no key that selects an option answers it. On a three-option prompt `2` and `Down`
/// then `Enter` both select "Yes, and don't ask again", which approves the call. The prompt is
/// the harness's own: conductor's guard denies by exit 2 with a reason, never by a prompt.
#[test]
fn the_supervisor_refuses_a_permission_prompt_with_escape() {
    let text = profile("supervisor.md");
    for phrase in [
        "refuse it with `send-keys Escape`, check the pane that the prompt is gone",
        "Refuse a harness permission prompt with `send-keys Escape`, then check the pane that the \
         prompt is gone",
    ] {
        assert!(text.contains(phrase), "supervisor.md: holds {phrase:?}");
    }
    for gone in [
        "`send-keys 2`",
        "`send-keys Down`",
        "guard refusal",
        "the hint",
        "or `1` when",
    ] {
        assert!(!text.contains(gone), "supervisor.md: still holds {gone:?}");
    }
}

/// `story:controller-profile-for-run`, acceptance 1 and 3: the controller profile fits in 120
/// lines and the maintainer's in 60. The host supervisor's and teacher's are held at their size
/// when they became one source, so a later rule replaces a line instead of adding one. The
/// supervisor's cap rose from 458 to 460 with `story:roles-in-the-binary`: its two answers to a
/// permission prompt (`send-keys 2`, and `Down` then `Enter`) both approved the call on a
/// three-option prompt, and the rule that replaces them (refuse with `Escape`, then check the pane
/// that the prompt is gone) is two wrapped lines longer.
#[test]
fn each_profile_stays_under_its_line_cap() {
    for (name, cap) in [
        ("controller.md", 120),
        ("maintainer.md", 60),
        ("supervisor.md", 460),
        ("teacher.md", 178),
    ] {
        let path = agents().join(name);
        let lines = fs::read_to_string(&path)
            .expect("read the profile")
            .lines()
            .count();
        assert!(lines <= cap, "{name}: {lines} lines, over its cap of {cap}");
    }
}

/// `story:controller-profile-for-run`, acceptance 1: every report, decision request and need is a
/// `conductor message send` to conductor's mailbox, the message on standard input, since text
/// printed in the session reaches nobody; the session-name addressing of `SendMessage`, the usage
/// and resource words and their paragraphs are gone, as the daemon handles them.
#[test]
fn a_controller_reports_with_conductor_message_send() {
    let text = profile("controller.md");
    for sentence in [
        "Every `[REPORT]`, `[DECISION-REQUEST]` and `[NEED]` is sent as `conductor message send \
         --to conductor --from <session name> --text -`, the message on standard input: text \
         printed in the session reaches nobody, and a turn that ends on a report ends with that \
         call.",
        "Bodies at most 20 lines",
    ] {
        assert!(text.contains(sentence), "controller.md: holds {sentence:?}");
    }
    for gone in [
        "Messages go only to `<session_prefix>-conductor`",
        "conductor.session_name",
        "[RESOURCE",
        "Usage limit",
        "usage limit",
        "Disk and build slots are not asked for by message",
    ] {
        assert!(
            !text.contains(gone),
            "controller.md: no longer holds {gone:?}"
        );
    }
}

/// `story:controller-profile-for-run`, acceptance 1: on `[HANDOVER <repo>]` a controller finishes
/// its step, writes the hand-over file the daemon looks for, reports it and stops.
#[test]
fn a_controller_writes_its_hand_over_file_reports_it_and_stops() {
    let text = profile("controller.md");
    for phrase in [
        "on `[HANDOVER <repo>]`",
        "finish the step, write `<checkout>/.git/handoff/<UTC stamp>.md`",
        "report `[REPORT <repo> <dispatch>] blocked handover <path>` and stop.",
    ] {
        assert!(
            text.contains(phrase),
            "controller.md: the hand-over holds {phrase:?}"
        );
    }
}

#[test]
fn a_controller_reads_the_sections_for_all_sessions_and_reports_pr_ci_and_merge() {
    let text = profile("controller.md");
    assert!(
        text.contains("follow its controller section and every section it marks for all sessions or for controllers"),
        "controller.md: reads the shared sections"
    );
    assert!(
        text.contains("when a pull request opens, when its CI turns red, and on merge"),
        "controller.md: reports PR, CI red and merge"
    );
}

/// `story:controller-trees-by-path`: a controller works in a managed tree by its absolute path,
/// never through `EnterWorktree`, which the guard denies (`guard_rules.rs` holds the denial).
#[test]
fn a_controller_works_in_a_managed_tree_by_its_absolute_path() {
    let text = profile("controller.md");
    assert!(
        text.contains(
            "Work in a managed tree by its absolute path, never through `EnterWorktree`, which \
             the guard denies: file tools name `<tree>/…`, and commands run as `cd <tree> && …` \
             or `git -C <tree> …`."
        ),
        "controller.md: managed trees by absolute path, never EnterWorktree"
    );
}

/// `story:resumable-gates`: a gate over an hour reruns only what failed and what depends on it,
/// and one full run stays for the release commit.
#[test]
fn a_long_gate_reruns_only_the_failed_steps_and_keeps_one_full_run_for_the_release() {
    let controller = profile("controller.md");
    assert!(
        controller.contains(
            "A gate that runs over an hour resumes: after a failure, rerun only the failed steps \
             and the steps that depend on them, never the whole suite again. Keep one full run \
             for the release commit."
        ),
        "controller.md: a long gate reruns only what failed"
    );
}

/// `story:work-beside-long-gate`: while a gate expected to run over an hour runs, the controller
/// proposes the repository's next wave units instead of waiting.
#[test]
fn next_units_run_beside_a_gate_expected_to_run_over_an_hour() {
    let controller = profile("controller.md");
    assert!(
        controller.contains(
            "While a gate expected to run over an hour runs, propose your next wave units to \
             conductor and work on them in a separate tree, within the build-slot budget, instead \
             of waiting for it."
        ),
        "controller.md: propose work beside a long gate"
    );
}

/// A profile as written, line by line.
fn profile_lines(name: &str) -> Vec<String> {
    let path = agents().join(name);
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
    let name = "controller.md";
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
/// first. Once in conductor-dev's profile, and twice in the controller's: its own paragraph and the
/// brief text it copies to a worker.
#[test]
fn every_profile_says_check_for_ess_specifications_and_get_them_green_first() {
    for (name, expected) in [("maintainer.md", 1), ("controller.md", 2)] {
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

/// `story:merge-human`: a charter's `merge: human` makes the controller open the pull request and
/// never merge it; the cycle's dispatch v1 holds such a repository at its review limit
/// (`tests/run.rs`).
#[test]
fn a_charter_s_merge_human_leaves_the_merge_to_a_person() {
    let controller = profile("controller.md");
    for phrase in [
        "`merge: human`",
        "never merges",
        "never enables auto-merge",
        "`[REPORT <repo> <dispatch>] review <url>`",
        "the dispatch stays open until",
    ] {
        assert!(
            controller.contains(phrase),
            "controller.md: the merge: human rule holds {phrase:?}"
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
    let text = profile("controller.md");
    assert!(
        text.contains(
            "When `worktree create` refuses with `workspace-not-activated`, report `[NEED \
             conductor] worktree workspace not activated: <root>` and, until it is answered, work \
             in a plain `git worktree add <trees>/<repo>/<name>` tree, which you remove when done."
        ),
        "controller.md: the worktree-activation fallback"
    );
}

/// `story:controller-without-store`: a controller delivers in a repository with no planning
/// store, offers one once in its `started` report and creates none itself.
#[test]
fn a_controller_delivers_in_a_repository_with_no_planning_store() {
    let text = profile("controller.md");
    for phrase in [
        "no planning store",
        "does not create",
        "`[REPORT <repo> <dispatch-id>] started; no planning store: /aep:init would add one`",
        "the dispatch's brief is the plan, written into the pull request's description",
    ] {
        assert!(
            text.contains(phrase),
            "controller.md: the no-store path holds {phrase:?}"
        );
    }
    assert!(
        site_page("concepts/roles.md")
            .contains("A repository needs no planning store to be dispatched to"),
        "website/docs/concepts/roles.md: a repository needs no planning store"
    );
}

/// `story:disk-escalation` and `story:guard-requires-build-run`: a controller and conductor-dev
/// run every build, test or gate under `conductor build run`, and report a refusal instead of
/// retrying it.
#[test]
fn every_build_runs_under_conductor_build_run_and_a_refusal_is_reported() {
    for name in ["controller.md", "maintainer.md"] {
        let text = profile(name);
        for sentence in [
            "build, test or gate runs as `conductor build run [--reserve auto|<size>] -- <command>`",
            "refusal names the shortfall and is reported, never retried in a loop",
        ] {
            assert!(text.contains(sentence), "{name}: holds {sentence:?}");
        }
    }
    let controller = profile("controller.md");
    // `story:reserve-from-peak`: the size is the ledger's recorded peak, not a guess.
    let flat = controller.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("builds use `--reserve auto` unless a size is known to be larger"),
        "controller.md: builds use `--reserve auto`"
    );
    assert!(
        !controller.contains("`--reserve 8G` for a cargo"),
        "controller.md: no fixed size is the default"
    );
    // `story:controller-profile-for-run` removed the resource paragraph that said disk and build
    // slots are not asked for by message; `a_controller_reports_with_conductor_message_send` holds
    // that no `[RESOURCE` form is left.
    let sentence = "The guard denies a plain `cargo build|test|check|clippy|nextest|install|run`, \
                    `task check|gate|install|ui|regen|docs-generate`, `npm run build|test` or `npm \
                    ci`, and its denial names the form to use; `cargo fmt` and `rustfmt` run as \
                    they are.";
    assert!(
        controller.contains(sentence),
        "controller.md: holds {sentence:?}"
    );
    assert!(
        !controller.contains("kill -STOP"),
        "controller.md: the hand-written watchdog is gone"
    );
}

/// `story:build-jobs-per-slot`, acceptance 3: `build run` sets each build's job count, so the
/// profiles' command forms pass none, and the profiles that run builds say why.
#[test]
fn the_profiles_pass_no_job_count_to_build_run() {
    for name in ["maintainer.md", "controller.md"] {
        let text = profile(name);
        assert!(
            !text.contains("CARGO_BUILD_JOBS="),
            "{name}: a command form passes no CARGO_BUILD_JOBS"
        );
    }
    for name in ["maintainer.md", "controller.md"] {
        let text = profile(name);
        let sentence = "`build run` sets `CARGO_BUILD_JOBS` and `MAKEFLAGS` to the build's share \
                        of the cores (the cores less `thresholds.cpu_reserve`, over \
                        `thresholds.build_slots`) and lowers a higher `CARGO_BUILD_JOBS` to it: \
                        pass neither.";
        assert!(text.contains(sentence), "{name}: holds {sentence:?}");
    }
}

/// `story:rules-show`: a worker brief cites the `rules.md` items it carries with the output of
/// `conductor rules show --item …`, never with an excerpt cut by hand from the file.
#[test]
fn a_worker_brief_cites_rules_items_through_rules_show() {
    let text = profile("controller.md");
    let sentence = "A worker brief carries the `rules.md` items it cites as the output of \
                    `conductor rules show --item <n>,<n>,…`, pasted whole, never an excerpt cut \
                    from the file by hand (`awk`, `sed` or line numbers); after a `rules.md` \
                    change, run it again for each running worker's items.";
    assert!(text.contains(sentence), "controller.md: holds {sentence:?}");
}

/// The design, `docs/design/conductor.md`, on single spaces.
fn design() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/conductor.md");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `story:conductor-start-reading`, as `story:run-daemon` leaves it: restarts are batched with the
/// next wave and at most one an hour (design § 12); conductor-dev restarts the loop, `conductor
/// run`, after a release that changed it, never conductor by a message, and a `rules.md` change
/// needs no restart.
#[test]
fn restarts_are_rare_and_a_rules_change_needs_none() {
    let design = design();
    assert!(
        design.contains(
            "Restarts are batched with the next wave, at most one an hour unless one corrects a \
             live fault."
        ),
        "design § 12: restarts are batched"
    );

    let dev = profile("maintainer.md");
    for sentence in [
        "Restarts are batched with the next wave, at most one an hour unless one corrects a \
         live fault.",
        "`conductor daemon serve --uninstall`, then `conductor daemon serve --install`",
        "A `rules.md` change needs no restart of the loop: commit it, and each session reads it \
         at its next start.",
    ] {
        assert!(dev.contains(sentence), "maintainer.md: holds {sentence:?}");
    }
    for gone in [
        "`rules.md`, design §§ 2–12 or the conductor role's `settings`",
        "[RESTART conductor]",
        "task conductor:restart",
    ] {
        assert!(
            !dev.contains(gone),
            "maintainer.md: no longer holds {gone:?}"
        );
    }
}

/// `story:controller-profile-for-run`, acceptance 3: conductor-dev observes through `conductor
/// status`, the dispatch log and `metric kpi-readings`, not a conductor session's transcripts or
/// its NEEDs (`story:conductor-dev-report-form` answered those; the daemon routes a NEED to a
/// controller only); it hands over at 150k context into the state directory, and every wave names
/// the metric it moves with a reading before and after.
#[test]
fn conductor_dev_observes_the_daemon_s_records_and_measures_each_wave() {
    let dev = profile("maintainer.md");
    for phrase in [
        "`conductor status`",
        "`dispatches/`",
        "`conductor metric kpi-readings`",
        "150k",
        "`<state>/conductor-dev/handoff/`",
        "Every wave names the metric it moves and records its reading before and after",
        "report it as taken",
    ] {
        assert!(dev.contains(phrase), "maintainer.md: holds {phrase:?}");
    }
    for gone in [
        "conductor's transcripts",
        "[NEED conductor",
        "[REPORT conductor-dev",
    ] {
        assert!(
            !dev.contains(gone),
            "maintainer.md: no longer holds {gone:?}"
        );
    }
}

/// Waves are wide and concurrent (the operator, 2026-10-14): every ready unit with disjoint scope,
/// all implementors started at once, the next wave planned while a gate runs.
#[test]
fn a_controller_runs_wide_concurrent_waves() {
    let text = profile("controller.md");
    for sentence in [
        "A wave takes every ready unit with disjoint scope, up to `controllers.max_subagents`; a \
         narrower wave records its reason on its wave page, as a line `Width: <reason>`.",
        "All of a wave's implementors start in one message, as background sub-agents,",
        "The next wave is planned and started while a gate or a CI run runs.",
        "Adversary and security reviews run alongside the building units, not after them.",
    ] {
        assert!(text.contains(sentence), "controller.md: holds {sentence:?}");
    }
}

/// `story:profile-zsh-shell`: the sessions' shell may be zsh, where an unquoted word that starts
/// with `=` expands as a command path and an unmatched glob aborts the command.
#[test]
fn controllers_quote_for_zsh() {
    let sentence = "The shell may be zsh: quote a word that starts with `=`, and guard a glob that \
                    may match nothing (`(N)` or an `ls` test).";
    assert!(
        profile("controller.md").contains(sentence),
        "controller.md: holds {sentence:?}"
    );
}

/// `story:guard-no-pattern-kill`: every profile that starts sessions holding a shell states the
/// rule the guard enforces, so a rewrite that drops it fails here.
#[test]
fn profiles_forbid_killing_by_pattern() {
    for name in ["maintainer.md", "controller.md"] {
        let text = profile(name);
        assert!(
            text.contains(
                "Never `pkill`, `killall` or `kill` by pattern, name, `-1`, a negative group or `%job`: it kills other sessions' processes (rules.md item 147). Stop what you started with TaskStop or `kill <pid>`."
            ),
            "{name}: the no-pattern-kill sentence"
        );
    }
}

/// `story:watch-pr-checks`: a controller never waits on its pull request's checks, the watch
/// reads them and `conductor run` relays its line to the repository's controller (`tests/run.rs`).
#[test]
fn the_watch_reads_pull_request_checks_and_the_run_daemon_relays_them() {
    let controller = profile("controller.md");
    let sentence = "After `progress PR opened`, work the next unit or report `blocked ci <pr>` and \
                    end the turn; conductor relays the watch line.";
    assert!(
        controller.contains(sentence),
        "controller.md: holds {sentence:?}"
    );
}

/// `story:build-run-lean-debuginfo`, acceptance 4: `build run` keeps debug information to line
/// tables for a cargo-based command (`thresholds.lean_debuginfo`), so the profiles say so and
/// none tells a session to pass the variables.
#[test]
fn no_profile_tells_a_session_to_pass_the_debug_information_variables() {
    for name in ["controller.md", "maintainer.md"] {
        let text = profile(name);
        for variable in [
            "CARGO_PROFILE_DEV_DEBUG",
            "CARGO_PROFILE_TEST_DEBUG",
            "line-tables-only",
        ] {
            assert!(
                !text.contains(variable),
                "{name}: `build run` sets {variable}; a session does not pass it"
            );
        }
    }
    for name in ["controller.md", "maintainer.md"] {
        assert!(
            profile(name)
                .contains("`thresholds.lean_debuginfo`): set no debug variable of your own"),
            "{name}: leaves the debug variables to `build run`"
        );
    }
}

/// `story:rules-into-code` item 8: each profile has its session read its role file with `rules
/// show --role`, and none asks a session to remember what the binary enforces: a decision id on
/// every `[DECISION …]`, no `timeout` around `conductor snapshot`, `nice` and `ionice` on a
/// build.
#[test]
fn profiles_read_the_role_file_and_ask_nothing_the_binary_enforces() {
    for (name, role) in [
        ("maintainer.md", "maintainer"),
        ("controller.md", "controller"),
    ] {
        let text = profile(name);
        assert!(
            text.contains(&format!("`conductor rules show --role {role}`")),
            "{name}: reads its role file with rules show --role {role}"
        );
        for remembered in [
            "carries the decision id it rests on",
            "nice -n 19",
            "ionice",
            "timeout",
        ] {
            assert!(
                !text.contains(remembered),
                "{name}: asks a session to remember what the binary enforces: {remembered:?}"
            );
        }
    }
}

/// `story:spawn-without-dispatch-reports`, acceptance 1: a controller that holds no dispatch id
/// reports with `-`, never with an id it made up.
#[test]
fn a_controller_with_no_dispatch_reports_with_a_dash() {
    let text = profile("controller.md");
    for phrase in ["`[REPORT <repo> -]`", "never make one up"] {
        assert!(
            text.contains(phrase),
            "controller.md: the report form without a dispatch holds {phrase:?}"
        );
    }
}

/// `story:conductor-context-budget`, applied to the maintainer (measured 2026-11-08: three
/// conductor-dev hand-overs in 70 minutes, each session at about 45k tokens before any reading):
/// the start reads the hand-over first and `rules.md` by item titles, and leaves the design
/// sections and the items in full to the step that needs them.
#[test]
fn conductor_dev_starts_on_the_hand_over_and_the_rules_titles() {
    let dev = profile("maintainer.md");
    let start = dev
        .split("At session start read")
        .nth(1)
        .and_then(|rest| rest.split("You own,").next())
        .expect("maintainer.md: a start list");
    for phrase in [
        "your newest hand-over in `<state>/conductor-dev/handoff/`",
        "`conductor rules show --section conductor-dev --brief`",
        "--section 'All sessions' --brief`",
        "(`--item <n>`)",
        "`conductor rules show --role maintainer`",
        "No file is read twice",
    ] {
        assert!(
            start.contains(phrase),
            "maintainer.md: the start list holds {phrase:?}"
        );
    }
    assert!(
        start.find("hand-over") < start.find("rules show"),
        "maintainer.md: the hand-over is read before the rules"
    );
    assert!(
        !start.contains("design § 12 and §§ 2–6;"),
        "maintainer.md: the design sections are not start reading"
    );
}
