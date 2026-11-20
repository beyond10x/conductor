---
format: aep.planning-md/3
id: story:person-wait-and-pause-hold-restarts
kind: story
status: implemented
title: A controller that waits only on persons, or is paused, is not restarted by the loop; the next input starts it
summary: The loop starts no controller whose every open dispatch waits on a person, and none of a paused one; a delivery or a moved dispatch starts it again
tags:
- from-issue
refs:
- provider: github
  reference: beyond10x/conductor#258
relations:
- decomposes: epic:routine-duties-in-loop
- informed_by: story:blocked-repository-yields-place-to-waiting-unit
- informed_by: story:stopped-controller-stays-stopped
- informed_by: story:loop-restarts-controller-at-handover
scope:
- confidence: cited
  path: crates/conductor/roles/controller.md
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: docs/design/conductor-run.md
- confidence: cited
  path: website/docs/concepts/decisions-and-messages.md
- confidence: cited
  path: website/docs/concepts/the-loop.md
- confidence: cited
  path: website/docs/guides/start-conductor.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T17:01:41Z", actor: "human:unknown", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-20T17:01:41Z", actor: "human:unknown", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-20T18:39:23Z", actor: "human:unknown", revision: 11, decided_on: {"recorded":{"test_result":2}}}
---
## Why

Issue #258 (filed against 1.19.2; unchanged on main at 1.21.0 plus wave 70). A repository's
controller had a dispatch `Blocked` waiting only on retests by persons. It reported
`blocked handover <file>` and ended; the supervisor then ran `controller pause-controller`; about
90 s later the loop started a new session anyway, which found nothing to do but wait, wrote a
hand-over again within a minute, and so on: hand-over/restart churn, each a full session start.

Read in the code on main:

- **What restarted it is the `session gone` handler, not the hand-over handler.** The hand-over
  handler already respects a pause: `handovers` reads only a `Running` controller and forgets the
  entry otherwise (`crates/conductor/src/run/handlers.rs:318-324`, `Tick::running` at
  `crates/conductor/src/run.rs:725-731`). But `controller pause-controller` stops the session
  (`crates/conductor/src/controller.rs:217-241`, `pause_here`), the watch reads `session gone: <name>`, and
  `gone` restarts any repository with an open dispatch, `Blocked` included
  (`run/handlers.rs:948-967`, `tick.restart` at `:965`; `Tick::open_dispatch`, `run.rs:773-782`).
- **A pause does not hold against a restart.** `Tick::restart_with` takes a `Paused` controller as
  held, stops it and starts the repository again (`run.rs:1258-1310`, the `Running | Paused` match
  at `:1266-1272`, the stop at `:1274-1286`, `start_again` at `:1290-1292`). Every watch-tick
  restart goes through it: `session gone`, `session stalled`/`silent` the second time
  (`run/handlers.rs:926`), a usage-limited hand-over (`:758`), and the hand-over restart
  (`hand_over`, `:273-287`). `session exited (blocked)` resumes the session from its transcript
  with no look at the controller's state (`run/handlers.rs:613-627`; `crate::spawn::resume_leased`
  reads none).
- **Nothing reads what a `Blocked` dispatch waits on.** `Tick::try_start` (`run.rs:822-880`), the
  one start path of `session gone`, a hand-over restart, the cycle's reconcile
  (`run/dispatch.rs:271`) and dispatch v1's start of an Active repository (`run/dispatch.rs:584`
  through `Tick::spawn`, `run.rs:808-812`), checks the default charter, `kept_for_unit`
  (`run.rs:899-975`), `make_room` and `at_limit`. A dispatch blocked on a person starts as one
  blocked on a hand-over. So even with the `session gone` restart withheld, dispatch v1 would start
  an Active repository's controller again at the next cycle.
- **The reason is in the log, readable as disk's is.** A `blocked <reason>` report writes one
  `blocked` line with its reason in the dispatches log; a `blocked` report on a dispatch already
  `Blocked` moves nothing and writes no line (`run/mailbox.rs:589-597`, `repeated`). So after
  `blocked person: …` then `blocked handover <path>`, the newest `blocked` line of the dispatch is
  still the person's. The loop already reads a block's kind this way for disk:
  `blocked_on_disk` and `on_disk` (`run/handlers.rs:1263-1309`, first word `disk`).
- **No report form says "waits on a person".** The controller profile names `blocked ci <pr>` and
  `blocked disk: <line>` (`crates/conductor/roles/controller.md:36-37`, `:97-98`) but no form for
  a wait on a person, and tells a controller to hand over "at your wave's close"
  (`controller.md:54-56`), which a controller that only waits reaches at once.
- **An input for a stopped controller waits for a start nothing makes.** A `[DECISION]` answer, a
  relayed `[NEED]`, a NEED answer or a mailbox reply for a repository with no running controller is
  kept in `<state>/run/pending.json` and made "once that repository has a running controller"
  (`deliver_or_keep`, `run/handlers.rs:1516-1525`; `deliveries`, `:1582-1592`); a watch line for a
  repository with none is dropped (`relay`, `:872-890`), but for a red `main`.

## Outcome

- **Waits on a person.** A dispatch waits on a person when it is `Blocked` and the newest
  `blocked` line of its id in the dispatches log (as `blocked_on_disk` reads it) has a reason whose
  first word is `person`, read as `on_disk` reads `disk`: case-insensitive, then the end or a
  character that is not alphanumeric (`person: …`, `Person …`; not `personal …`, not `persons …`).
  The word is followed by the end, a `:` or whitespace only (`person: …`, `Person …`; not
  `personal …`, `persons …`, `person-x`, `person_x`, `person's …`).
  A repository is **held for a person** when it has at least one open dispatch, every open
  dispatch waits on a person, and `<state>/run/pending.json` keeps no delivery for it that no
  controller refused for good and that is no watch line.
- **Read cheaply, parsed once.** The dispatches log is read only when the store shows at least one
  open dispatch of the repository and every one `Blocked`, and at most once per tick or cycle
  (`Tick::newest_blocked`, the newest `blocked` reason of every dispatch, dropped when the tick
  writes a `blocked` line); `person.json` is written only when an entry changes; the newest `blocked` reason of a
  dispatch is one helper (`newest_blocked_reason`) that `blocked_on_disk` uses too, and the first
  word test is one helper beside `on_disk`.
- **No start of a held repository.** `Tick::try_start` refuses the start of a repository held for
  a person, inside the `STARTS` lock and before `kept_for_unit` and `make_room`, so no idle
  controller is stopped for it. The refusal is its own variant, `NotStarted::Held(why)`, answered
  by `start_again` as `Again::Held(why)` (not `Refused`, not `Yielded`); `why` reads `<dispatch>
  waits on a person: <rest>` (the first such dispatch in store order), `<rest>` the reason with
  its leading `person` and the `:` and whitespace after it taken off, so it never reads
  `person: person:`. It holds for every start through `try_start`: the watch
  tick's restart after `session gone`, `session stalled` and a hand-over (the held controller
  stopped as today, its dispatch left `Blocked`), the cycle's reconcile restart, and dispatch v1's
  start of an Active repository. In the watch tick it is no error: `restart_with` writes one journal line `<repo> waits: <why>`,
  records the repository in `person.json`, forgets the session (`handlers::forget`), and the tick
  goes on. In the cycle it is the repository's `waiting` reason and
  its `<repo> waits: …` part of the `cycle` line, as any refusal of `Tick::spawn`.
- **Kept, so it is found again.** A refusal for a person, from any caller (`restart_with`,
  reconcile, `Tick::spawn`), writes the repository into `<state>/run/person.json` (its dispatch
  ids, the reason, when, and the last refusal said), a run-state file beside `handover.json`,
  `limited.json` and `pending.json`, named in `run/handlers.rs`'s module doc (`:18-30`) and in
  `docs/design/conductor-run.md`. The person pass reads it twice in each watch tick, each time
  followed by the kept deliveries: in `pending` after the hand-overs, and again after
  `mailbox::read` and `disk_recovered`, so an item the mailbox or a watch line kept in this tick
  starts its controller in this tick. The pass: an entry whose repository runs a
  `Running` or `Paused` controller or has no open dispatch is dropped; one whose repository is no
  longer held for a person is started (`start_again`, `held` false), its `controller-restarted`
  line's reason `person wait ended: <what>` (`a delivery kept: <event>` or `the dispatch moved`),
  and dropped. A start refused for any reason (`Again::Yielded`, `Again::Held`, a refusal at
  `max_working`) keeps the entry; its journal line is written only when the refusal differs
  from the one the entry keeps.
- **What ends the wait (the next input).** Decision (class C, controller, wave 72 review): a
  relayed watch line does not end it; each CI event restarting a controller that only waits again
  is the churn of #258. Each of these makes the repository no longer held:
  - a delivery kept for it in `pending.json` that no controller refused for good (an item with
    `refused_by` set does not end a wait: it waits for a controller other than the one that
    refused it, and would start the repository again only for that one to refuse it): a `[DECISION <id>]` the `answered` line delivers, a
    `[NEED]` another repository's controller sent to it, a `[REPORT <repo> -] need … opened …` or
    `not relayed` answer, a `[RESOURCE]` answer, a mailbox refusal; the started controller is
    offered it by the `deliveries` that follows the pass, as any kept delivery;
  - a mailbox reply or operator message addressed to it, kept as any delivery;
  - a new dispatch sent for it (then not every open dispatch is `Blocked`);
  - the dispatch leaving the wait: `dispatch report-unblocked` (a person's retest result, given by
    the operator), a newer `blocked` line with another reason, or the dispatch done, failed or
    cancelled (then no open dispatch: the entry is dropped and nothing starts).
  After an input, a controller still waiting only on persons reports `blocked person: …` again
  (from `started`, so the log has a new line) and ends its turn; its idle session keeps its place
  until `session silent` twice restarts it, which stops it into the hold. So a held controller is
  never stranded and never churns: every input the loop routes to a repository reaches a
  started controller, and the operator's `dispatch report-unblocked` or `conductor spawn
  --repository <repo>` (by hand, outside `try_start`) starts one at any time.
- **Watch lines are kept, not acted on.** A watch line `relay` would deliver (`PR checks done`,
  `PR merged`, `main CI after merge`, `CI red on main`, `CI green again on main`) for a held
  repository with no running controller (an entry in `person.json`; or, for a red `main`, a
  `red_ci` start refused as held, whose `red-ci` line then names the hold) is kept as `[REPORT
  <repo> -] watch: <line>` with its key (`watch`: the line's kind and subject, `PR checks done:
  alpha#7`, `CI red on main: alpha / check`), at most one item per repository and key, a newer
  line replacing the older. It ends no wait; the controller the next input starts is offered it.
- **Resumes are waits too.** A resume of a held repository's session (`session exited (blocked)`,
  a usage limit's reset, a withdrawn token, a resume on another pool token) resumes nothing: its
  running controller of that session is recorded stopped, the repository kept in `person.json`,
  one journal line `<repo> waits: <why>`.
- **Offers count ticks.** The second person pass of a watch tick (after the mailbox) offers the
  kept deliveries only to the repositories it started, so a delivery is offered to one controller
  at most once per tick and `OFFERS` counts ticks.
- **A pause holds.** No watch-tick handler starts, resumes or stops a session of a repository that
  has a `Paused` controller: `Tick::restart_with` answers before it stops anything (so `session
  gone` and the usage-limited hand-over; `session stalled`/`silent` and the hand-over restart
  already read only a `Running` controller), and the resumes of `exited`, `limited` and `withdrawn` (`run/handlers.rs:591-606`) do the same:
  the check is in `resume` and in `limited` before any resume. A pause keeps what is kept of the
  session: its `handover.json` request (`handovers` keeps the entry of a `Paused` repository) and
  its `limited.json` mark, for the controller `resume-controller` resumes. A `person.json` entry
  of a `Paused` repository is dropped. One journal line `<repo> is paused: <session> not
  restarted` (or `not resumed`), said once while the pause lasts (`<state>/run/paused.json`), the
  controller left `Paused`, no dispatch moved, no `controller-restarted` or `resumed` line. Only `controller resume-controller`
  (or `stop-controller`, then a start) ends a pause. The cycle already starts none for a `Paused`
  repository (`run/dispatch.rs:12-30`, the `controlled` set) and reconciles only `Running` ones.
- **The controller says so.** `crates/conductor/roles/controller.md` names the form `[REPORT
  <repo> <dispatch>] blocked person: waits on <who> for <what>` for a dispatch whose remaining
  work waits only on persons (the words `waits on` keep the supervisor watch's idle check from
  counting it idle), and says: when the dispatch is already blocked for another reason, report
  `unblocked` first, then `blocked person: …`; end the turn; write no hand-over for it (a
  hand-over is for the context threshold and a wave's close with work left); the loop starts the
  controller again on the next input, and one still waiting only on persons then reports
  `blocked person: …` again (after `started`) and ends its turn. A change to a profile is reported with its diff (AGENTS.md rule 9).
- Docs: `docs/design/conductor-run.md` (the watch-line table, § on starts, the run-state files),
  `website/docs/concepts/the-loop.md` and `website/docs/concepts/decisions-and-messages.md` say
  the person wait, what ends it, the idle-then-silent path, and that a pause holds against every
  loop restart; `website/docs/guides/start-conductor.md` (`:89-95`) gets one sentence that a
  pause holds against every loop restart until `resume-controller`. The module doc of
  `run/dispatch.rs` (`:10-12`, the reconcile restart) names the hold; no code there changes.

Decision (class C, planner, wave 72): "waits only on persons" is read from the dispatches log's
newest `blocked` reason, first word `person`, not from the hand-over file (free markdown), the
newest message (a `blocked handover` on a `Blocked` dispatch writes nothing) or a class-H decision
request (an escalated request ends in an `answered` line, which ends the wait as a kept
`[DECISION]`). Undone by reading another field; the hold is a refusal of `try_start`, so dropping
the check restores today's behaviour.

Spec: no change. The reason is the free-text reason `report-blocked` already records; the
`controller-restarted` line's reason and the `waiting` reason are free text
(`spec/domains/dispatch.yaml:586-589`); `person.json` is run state beside `handover.json`, not the
store. No new noun, field, command, view, event or error: `ess specify validate --path spec`
stays green and `crates/conductor-model` is not regenerated. Should the implementor find a new
`DispatchLogEvent` needed, it is modelled in `spec/` first and the story's scope extended, not
hand-written.

## Acceptance (tests that fail on the current code)

Fixture P: `alpha` spawned (`case.spawn("alpha", FIRST)`) with an open dispatch, mailed
`[REPORT alpha <id>] started`, then `[REPORT alpha <id>] blocked person: waits on a reviewer
for a retest of the sign-in flow`, then `[REPORT alpha <id>] blocked handover docs/handoff/2026-11-20-alpha.md`,
and a tick to take the mail. The `tests/run.rs` cases bind no socket, but they need a host where
`socket(AF_UNIX)` may be created: elsewhere the `none` provider becomes `child`
(`crates/conductor/src/confine/mod.rs:250`) and no `--bg` launch is made, so `launched()` reads
nothing. They run on CI; the unit test runs anywhere.

- `tests/run.rs` `a_gone_session_waiting_only_on_a_person_is_not_restarted`: fixture P; a watch
  tick with `session gone: ex-alpha …` writes no `controller-restarted`, launches no second
  `ex-alpha`, records the controller `Stopped`, leaves the dispatch `Blocked`, and its journal
  names `alpha waits: <id> waits on a person: waits on a reviewer for a retest of the sign-in flow`.
- `tests/run.rs` `a_handover_restart_of_a_controller_waiting_on_a_person_starts_none`: fixture P
  without the hand-over mail; the hand-over driven by `handed_over` (`tests/run.rs:2037`); the
  tick stops `alpha`'s controller, launches nothing, leaves the dispatch `Blocked`, writes no
  error, its journal names `alpha waits: <id> waits on a person: …`, and `person.json` holds
  `alpha`.
- `tests/run.rs` `a_cycle_starts_no_controller_for_a_repository_waiting_on_a_person`: fixture P,
  `alpha` Active in a complete snapshot, its controller stopped with `stop_alpha`
  (`tests/run.rs:1975`); the next
  cycle says no `spawned alpha`, its `cycle` line and the dispatch's `waiting` reason name `waits on
  a person`.
- (The next three start in the same tick through the person pass after `mailbox::read`.)
- `tests/run.rs` `a_decision_answer_starts_a_controller_held_for_a_person`: after the first
  case, a decision request of `alpha` raised and decided through the CLI and an `answered <id> …`
  line; that watch tick writes `controller-restarted` with the reason `person wait ended: a
  delivery kept: decision-sent`, launches `ex-alpha` again, and the delivery of `[DECISION <id>]`
  is offered to it.
- `tests/run.rs` `a_need_to_a_held_repository_starts_its_controller`: after the first case,
  `beta`'s controller mails `[NEED beta r1] alpha …`; the tick keeps it for `alpha` and starts
  `alpha` (`controller-restarted`), and the need is offered to it.
- `tests/run.rs` `an_unblocked_report_starts_a_controller_held_for_a_person`: after the first
  case, `dispatch report-unblocked --dispatch-id <id>`; the next watch tick writes
  `controller-restarted` with the reason `person wait ended: the dispatch moved` and launches
  `ex-alpha`.
- `tests/run.rs` `a_watch_line_for_a_held_repository_is_kept_and_starts_nothing`: after the
  first case, `PR checks done: alpha#7 …`; the tick and the next keep `[REPORT alpha -] watch: PR
  checks done: alpha#7 …` for `alpha`, start nothing, and keep the entry.
- `tests/run.rs` `a_decision_answer_after_a_kept_watch_line_starts_it_and_offers_the_line`: after
  the previous, a decision answered for `alpha`: the tick starts it (`person wait ended: a delivery
  kept: decision-sent`) and offers it the decision and the kept watch line.
- `tests/run.rs` `a_held_repository_keeps_one_watch_line_per_kind_and_subject`: `PR checks done`
  of `alpha#7` twice, of `alpha#8`, `CI red on main: alpha / check` twice: three items kept, each
  the newest of its kind and subject; nothing starts.
- `tests/run.rs` `a_kept_delivery_is_offered_once_per_tick_beside_a_held_repository`: a `[NEED]`
  for `beta`, whose live session refuses it and is asked again, beside held `alpha`: one offer per
  tick over two ticks (the second person pass offers only to repositories it started).
  `a_delivery_refused_for_a_live_session_is_offered_five_times_and_then_marked` passes unchanged.
- `tests/run.rs` `a_red_main_of_a_held_repository_with_no_entry_is_kept`: fixture P, `alpha`'s
  controller stopped by the operator, `CI red on main: alpha / check …`: the `red-ci` line's reason
  is the hold, the line is kept, nothing launched, `alpha` in `person.json`.
- `tests/run.rs` `a_held_repository_s_exited_session_is_not_resumed` and
  `a_held_repository_s_withdrawn_session_is_not_resumed`: fixture P, then `session exited
  (blocked): ex-alpha …`, or `ex-alpha` named in `tokens-withdrawn.json`: no resume, the
  controller recorded `Stopped`, `alpha` in `person.json`, one journal line `alpha waits: …`.
- `tests/run.rs` `a_paused_controller_s_withdrawn_session_is_not_resumed`: a paused `alpha`'s
  session named in `tokens-withdrawn.json`: no resume, still `Paused`, one journal line.
- `tests/run.rs` `a_paused_controller_s_hand_over_request_is_kept`: a hand-over asked of `alpha`,
  then a pause: the next tick keeps the `handover.json` entry.
- `tests/run.rs` `a_paused_controller_s_gone_session_is_not_restarted`: `alpha` spawned with a
  `Started` dispatch, `controller pause-controller --controller-id <id>`, then a watch tick with
  `session gone: ex-alpha …`: no launch, no `controller-restarted`, the controller still `Paused`,
  the journal names `alpha is paused: ex-alpha not restarted`.
- `tests/run.rs` `a_paused_controller_s_exited_session_is_not_resumed`: as the previous, with
  `session exited (blocked): ex-alpha …`: no `resumed` line, nothing launched, still `Paused`.
- `tests/run.rs` `a_paused_controller_s_usage_limit_reset_resumes_nothing`: `alpha`'s session
  marked by a `usage limit: …` line whose reset has passed, then its controller paused; the next
  watch tick writes no `resumed` and no `controller-restarted`, launches nothing, leaves it
  `Paused`, and keeps its `limited.json` mark.
- `tests/run.rs` `a_paused_repository_s_stopped_limit_mark_resumes_nothing`: a usage-limit mark
  the cycle recorded `stopped`, its reset passed, the controller recorded `Paused`; two watch
  ticks resume nothing, keep the mark in `limited.json`, and say `alpha is paused: ex-alpha not
  resumed` once.
- `tests/run.rs` `a_paused_repository_s_entry_is_dropped`: after the first case, a new controller
  of `alpha` started by hand and paused; the next tick drops `alpha` from `person.json` and starts
  nothing.
- `tests/run.rs` `a_repeated_person_refusal_writes_one_journal_line`: after the first case,
  `answered` for a request of `alpha` kept while `max_working` is reached by others; two ticks
  write one journal line for the refused start, and the entry stays.
- `tests/run.rs` `a_lift_refused_at_max_working_keeps_the_entry`: as the previous; once a place
  is free: the entry is kept while the start is refused; once a place is free, the next tick
  starts `alpha` and drops the entry.
- `tests/run.rs` `a_held_repository_s_red_main_is_kept_not_spawned`: after the first case, `CI red
  on main: alpha / check …`; no `red-ci` spawn, the `[REPORT alpha -] watch: …` delivery is kept,
  and nothing starts.
- `tests/run.rs` `a_held_dispatch_cancelled_drops_its_entry`: after the first case, `dispatch
  cancel-dispatch`; the next tick drops `alpha`'s entry and starts nothing.
- `tests/run.rs` `a_delivery_refused_for_good_does_not_end_a_person_wait`: a `[DECISION]` kept
  with `refused_by` the stopped controller; a later `session gone` writes the journal line and
  keeps `alpha` in `person.json`, and it and the next tick start nothing.
- `tests/run.rs` `a_person_wait_after_another_block_is_read`: `alpha` reports `blocked ci 7`,
  `unblocked`, `blocked person: waits on a reviewer for a retest`; `session gone` starts nothing.
- `tests/run.rs` `a_paused_controller_s_silence_restarts_nothing` (new; passes on today's code,
  `stalled` reads only a `Running` controller, `run/handlers.rs:917-923`): a check-in sent while it
  ran, then a pause, then `session silent: ex-alpha …`: no stop, no launch.
- `tests/run.rs` `a_blocked_handover_without_a_person_wait_restarts_as_before` (new; passes on
  today's code, it guards the rule): `alpha` reported `started` then `blocked handover …` only;
  `session gone: ex-alpha …` writes `controller-restarted`, reason `gone`.
- `tests/run.rs` `a_personal_reason_is_no_person_wait` (new; passes on today's code, it guards the
  parse): fixture P with `blocked personal token expired` for the person's line; `session gone`
  restarts `alpha`.
- `tests/run.rs` `a_gone_session_with_an_open_dispatch_is_respawned` and the
  `story:blocked-repository-yields-place-to-waiting-unit` cases pass unchanged.
- Unit test in `crates/conductor/src/run/handlers.rs` `a_person_reason_is_read_by_its_first_word`:
  `person: x`, `Person x`, `person`, `person:x`, `person<tab>x` are a person wait; `personal x`,
  `persons x`, `person-x`, `person_x`, `person's retest`, `person.x`, `disk: x`, `handover x` are
  not; the refusal's `<rest>` of `person: waits on x` is `waits on x`.
- `crates/conductor/roles/controller.md` names `blocked person:` and that no hand-over is written
  for a wait on persons (`tests/profiles.rs`
  `a_controller_waiting_only_on_persons_reports_blocked_person_and_writes_no_hand_over`); its line
  cap rises from 120 to 122 for it; the diff is in the pull request.

## Boundaries

- A controller stopped with `controller stop-controller`, and the cycle's start of a repository
  with no work, are `story:stopped-controller-stays-stopped` (draft; it also scopes
  `run/dispatch.rs`, so it is not in the same wave as this story). This story touches neither the
  `Stopped` state's meaning nor which repositories dispatch v1 visits.
- `story:blocked-repository-yields-place-to-waiting-unit` (on main) is kept: the person check
  runs before `kept_for_unit` and refuses for its own reason; a held repository holds no place.
- One unit, not two: both halves change the same functions (`Tick::restart_with`, `resume`,
  `resume_stopped`, the `pending` order) in `run.rs` and `run/handlers.rs` and the same test file;
  split, they could not run in one wave.
- `story:supervisor-watch-forge-triggers` scopes `crates/conductor/src/supervisor_watch/*`,
  `crates/conductor/src/watch.rs`, `crates/conductor/tests/supervisor_watch*.rs`,
  `crates/conductor/roles/supervisor.md` and `spec/domains/host.yaml`; this story edits none of
  them, and no other wave-72 unit scopes `run.rs`, `run/*.rs`, `tests/run.rs` or
  `roles/controller.md`. Reading `blocked person` in the supervisor watch's idle check (today held
  back by the words `waits on`) is a follow-up; so is the supervisor's by-hand restart rule
  (`roles/supervisor.md` around `:426-437`: `conductor spawn` bypasses `try_start`), unchanged
  here.
- Drafts that touch the same files, none in this wave: `story:handover-restart-by-envelope-id`
  (which session a hand-over restart acts on), `story:controllers-reconciled-with-live-sessions`
  (reconcile), `story:merge-wakes-idle-controller` (owns how a `PR merged` line reaches a
  controller in general; this story only keeps a relayed line for a repository held for a
  person).
- Pause race, named not fixed: `pause_here` ends the session (`controller.rs:226`) before it
  records `Paused` (`:233-238`), in another process than the loop, so the in-process `STARTS`
  lock cannot cover it; a watch pass that reads the session gone between the two still restarts
  it. A follow-up may record the pause first.
- `crate::watch` (`crates/conductor/src/watch.rs`) is used as it is; the watch lines are unchanged.
- `controller pause-controller`, `resume-controller` and `stop-controller` keep their commands,
  flags and records; the hand-over file is not read; no new state, event or command.
- The order of starts, `max_working`, `make_room` and the usage-limit marks are unchanged.
- A held repository whose only new events are watch lines stays held until a person acts (a
  decision answer, `dispatch report-unblocked`, a `[NEED]`, a new dispatch) or the operator
  starts it by hand: the watch lines wait for that controller (decision class C, wave 72 review).
- The `tests/profiles.rs` line cap of `controller.md` rises from 120 to 122; no other rule of the
  profile is shortened for the person wait.
