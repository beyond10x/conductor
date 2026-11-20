---
title: The loop
sidebar_position: 2
description: conductor run and its three ticks; what a snapshot is and its eight collectors; the watch pass and its lines; which repositories are Active; where the loop keeps its state.
lede: The loop is one process per instance, conductor run, with no model of its own. It keeps everything in the store and its state files, so a restart loses nothing.
source: docs/design/conductor-run.md section 3; docs/design/conductor.md sections 4, 8 and 10; crates/conductor/src/run.rs, run/dispatch.rs, run/handlers.rs, run/mailbox.rs, snapshot.rs, collect/, watch.rs, repository.rs; conductor run --help, conductor watch run --help
---

## Three ticks

`conductor run` holds `<state>/run/lock`: a second one refuses.

| tick | when | does |
|---|---|---|
| watch | every `cadence.watch` (180 s) | the watch pass, `main`'s CI every `cadence.ci` (900 s); one handler per watch line; the mailbox |
| cycle | every `cadence.cycle` (2 h), first at start | `snapshot start-snapshot`, `snapshot publish-board`, `records check`, dispatch v1 |
| daily | at `cadence.daily` (08:07 UTC) | `metric compute` for the previous day, `<records>/REPORT.md`, a records commit |

Before dispatch v1 the cycle checks each `Running` controller against the session list. A
controller whose session is gone, neither live nor waiting at a usage limit (after a runtime
restart or an OOM kill, say), is recorded stopped, and the `cycle` line says why; it no longer
counts against `controllers.max_working`. One whose repository has an open dispatch starts again,
as after `session gone`.

Dispatch v1 runs `conductor spawn` for each Active repository with a charter and no running or
paused controller, the one that has waited longest for a place first, while fewer than
`controllers.max_working` run. A repository whose charter has `merge: human` waits while
`controllers.max_open_reviews` of its pull requests wait for review. A repository is sent a goal it
serves once: after a `done` dispatch for that goal it gets no other from the loop. When every place
is taken, the controller of a repository with no open dispatch whose session is idle is stopped
for the waiting repository, and the `cycle` line names it. Each open dispatch that gets no
controller carries why (no complete snapshot, Inactive, no charter, `max_working` reached, a
refused start), shown by `dispatch dispatches` and written once per change as a `waiting` line.

A repository whose every open dispatch is `Blocked` gives way to the units of work that wait for a
place, while places are short: the free places do not exceed the repositories those units wait in.
A unit waits when its last start was refused at `max_working`, not for a lasting reason. The
repository is not started while a unit its own `[NEED]` opened waits; and when no controller of it
ran as the start began (its controller was stopped, by the loop or the operator), it is not started
again while any unit waits. A routine hand-over restart keeps its place but for the first rule. A
start that gives way says `max_working <n>: the place is kept for <dispatch> of <repo>`: in the
watch tick as one journal line, in the cycle as the repository's `waits:` part of the `cycle` line.

A controller whose work is left only to persons (a review, a retest) reports `blocked person:
waits on <who> for <what>` and ends its turn, with no hand-over. A repository whose every open
dispatch is blocked that way, with nothing kept for it, is held for a person: the loop starts no
controller for it, neither after `session gone`, a second `session silent` or a hand-over, nor in
the cycle, nor resumes its session, and says `<repo> waits: <dispatch> waits on a person: …` once.
The next input ends the wait: a decision answer, a `[NEED]` or another delivery kept for the
repository, `dispatch report-unblocked`, a new dispatch, or the dispatch blocked for another reason.
A watch line for it, such as `PR checks done` or `CI red on main`, does not: it is kept, the newest
of each kind and subject, for the controller the next input starts. The watch tick that finds the input starts the controller, its
`controller-restarted` line's reason `person wait ended: …`, and offers it what was kept. A
controller that still waits only on persons reports `blocked person: …` again and ends its turn;
once its idle session is read `session silent` twice it is stopped into the hold again. A
dispatch done, failed or cancelled ends the hold and starts nothing.

A pause holds against every loop restart: no watch-tick handler starts, resumes or stops a session
of a repository whose controller is `Paused` (`session gone`, a session that exited, a usage
limit's reset, a hand-over), and the cycle starts none for it. Each is said once in the journal,
`<repo> is paused: <session> not restarted` or `not resumed`; a hand-over request or a usage-limit
mark of the session is kept. Only `controller resume-controller` ends a pause.

A unit of work's controller starts on conductor's default charter where the repository has none.
`controllers.unit_merge` chooses its form: `human`, the default, ends `merge: human`, and the
controller reports the request as `review <url>` and never merges it; `controller` ends `merge:
controller`, and the controller merges the request itself once its full gate passed and its checks
are green, one request at a time and never by auto-merge, then reports `done`. A charter equal byte
for byte to either form is rewritten to the form `unit_merge` names at the next unit start, so a
changed setting reaches a repository still on the default. Any other charter is the operator's and
is never rewritten. Either form gets a controller only for a dispatch.

Each tick is recorded (`session run-ticks`). A handler that fails prints `conductor run: <what>:
<why>` and the tick goes on. A tick that cannot be recorded fails the run. `--once` runs one watch
tick and one cycle tick and exits.

The loop's one model call: `claude -p`, with no tools, answers a class C `[DECISION-REQUEST]`
([decisions and messages](./decisions-and-messages.md#handled-in-code)).

The runtime conductord starts runs the loop; `conductor run` runs it by hand when no runtime
holds the lock ([the host](./the-host.md#the-loop)).

## A snapshot

`conductor snapshot start-snapshot` runs the collectors in order and completes when all
answered. The first that does not fails it with `<collector>: <error>`. A failed snapshot is
kept, never read as current
([troubleshoot](../guides/troubleshoot.md#a-failed-snapshot)).

| collector | records |
|---|---|
| repositories | one row per repository, the fields of `snapshot record-repository` ([CLI](../reference/cli.md#conductor-snapshot-record-repository)) |
| github | open and merged pull requests, the newest `main` run per workflow, releases |
| gitlab | the same through `glab api --hostname <host>` ([GitLab](../guides/gitlab.md#what-conductor-reads-from-gitlab)) |
| blockers | open artifacts whose kind ends in `blocker`, read at each workspace member's `origin/main` |
| specifications | per repository: present, opted out or missing; valid or refused; scenario count |
| sessions | live Claude Code and Codex sessions and the repository each works in |
| secrets | per secret sync, when its source and its secrets last changed |
| usage | each window of the accounts `usage.probe` prints |

The workspace members are those of `<records>/.engineering/workspace.yaml`
([config](./config.md#workspace-and-sources)). A member a collector cannot read is a member failure
(`snapshot member-failures`), and the snapshot goes on. Under `--budget <seconds>` a collector
past it is stopped, `OverBudget` in `snapshot collector-runs`, and the next one runs.

The views (`snapshot repositories`, `pull-requests`, …) list every kept snapshot's rows: filter on
the newest `snapshot_id`.

## Active repositories

Conductor processes issues and pull requests only for Active repositories. `conductor repository
activity` decides each one when read, and stores nothing:

- A mark (`mark-repository`, `activate-repository`, `deactivate-repository`), set with a reason,
  wins. No snapshot overwrites it.
- Without a mark, a repository is Active when the newest complete snapshot counts a real commit in
  7 days and it is not archived.

A commit is not real when it is a merge, its normalised subject is in 3 or more repositories in
the window, its subject is maintenance, or it touches only docs, CI, planning, markdown or lock
files ([add a repository rule](../guides/add-a-repository-rule.md)).
`conductor repository shipped --since <RFC 3339>` lists what was released or merged since an instant.

## The watch

The watch runs cheap probes, one pass per watch tick. `conductor watch run` runs passes by hand
until one finds a change, or with `--follow` prints each change and goes on.

| probe | lines |
|---|---|
| sessions of the instance | `session exited (blocked)`, `session gone`, `conductors:` for two live conductors |
| transcripts | `usage limit`, `context` at `thresholds.context_handover`, `session silent` after `thresholds.silent_session` |
| dispatch log and commits | `session stalled` after `thresholds.stalled_session` |
| host ledger | `build queued` after `thresholds.build_queued` |
| requests linked to open dispatches | `PR merged`, relayed to the repository's running controller |
| default-branch pipelines of those merges, any repository | `main CI after merge` once the pipeline of the merge commit is green or red, relayed to the repository's running controller |
| CI of Active repositories | `CI red on main`, `CI green again on main`, `PR checks done` |
| newest complete snapshot | `review full`, `review open`, `gates hooks stale`, `workspace stale` |
| free disk on `/` | `disk low`, `disk admit`, `disk gate stop` ([disk](./disk.md)) |
| store | `usage:` forecast, `escalated`, `unanswered`, `answered` |

A session start is no change, and an exit a dispatch log line explains is not reported. Each
line's handler delivers to a controller, restarts it or starts `gc`, and logs a line such as
`relayed` in the dispatch log.

## Where the state is

The loop's records are in the store, one database per deployment
([records and store](./records-and-store.md#the-store)). The watch keeps its state files under
`<state>/watch/`, the loop its own under `<state>/run/`.
