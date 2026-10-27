---
format: aep.planning-md/3
id: story:run-daemon
kind: story
status: draft
title: 'conductor run is the instance''s loop: watch handlers, mailbox, cycle, dispatch v1 and the daily metrics, installed as a user unit'
relations:
- decomposes: epic:conductor-run
revision: 1
---
## Why

Every routine step of the loop is a model turn today (`docs/design/conductor-run.md` § 2): the
cycle runs on a `CronCreate` in a session, watch lines are read under `Monitor` and acted on by
prompt, hand-overs and resumes are messages, dispatch is a session writing charters. The watch
already computes every signal (`crates/conductor/src/watch.rs:766-797`) and the build ledger already
queues (`crates/conductor/src/build.rs:912-1116`); only the acting is missing.

## Outcome

`conductor run` is the instance's loop: a process that runs the watch pass and the handlers of
design § 3 on `cadence.watch`, the cycle on `cadence.cycle`, `metric compute` and the brief at
`cadence.daily`, reads the mailbox, moves dispatches from reports, dispatches controllers from
charters, and is installed as the user unit `conductor_<instance>-run.service`. Adopts
`story:cycle-command`, `story:report-to-state` and `story:auto-resume`.

## Acceptance

1. Spec first: `spec/domains/session.yaml` declares the daemon's tick record (`RunTick`: kind
   watch|cycle|daily, started, lines handled, actions taken) and the actions it records reuse the
   existing dispatch events (`handover-sent`, `handover`, `controller-restarted`, `resumed`,
   `relayed`, `red-ci`, `stalled`, `cleanup`, `decision-sent`); `ess specify validate` exits 0.
2. `conductor run --once` runs one watch pass, the handlers and the mailbox, then one cycle, and
   exits 0; `conductor run` loops on the cadences; `conductor run install` writes and starts
   `conductor_<instance>-run.service` (systemd provider; `none` provider: prints the command to run)
   and `conductor run uninstall` stops it. `conductor doctor` reports the unit's state.
3. Handlers, each with a test over a fixture store, fixture watch lines and a fake `tmux`,
   `systemd-run`, `systemctl` and `claude` on `PATH` (`crates/conductor/tests/run.rs`):
   - `context:` over the threshold → one `[HANDOVER <repo>] context <n>k` delivered per session;
     over 1.25× the threshold, or when `<checkout>/.git/handoff/` holds a file newer than the
     delivery → `spawn::end`, the hand-over path appended to the charter as the resume point,
     `spawn` again; records `handover-sent`, `handover`, `controller-restarted`.
   - `usage limit:` → sessions marked; a later tick with headroom (an `ais status --json` reader
     behind a trait, faked in tests) → `spawn::resume` for each marked session, or respawn when
     over the threshold; records `usage-limit`, `resumed`.
   - `PR checks done`, `CI red on main`, `CI green again` → delivered to the live controller as
     `[REPORT <repo> -] watch: <line>`; records `relayed`; red main without a controller and with a
     charter → spawn; records `red-ci`.
   - `session stalled`/`session silent` → first a delivered check-in, second a respawn; records.
   - `session gone`/`exited (blocked)` with an open dispatch → respawn.
   - `disk low` → `gc`; records `cleanup`.
   - `answered <id> <choice>` → `[DECISION <id>] <choice> <reason> (<decision id>)` delivered to the
     controller that raised it; records `decision-sent`.
4. Mailbox: each tick reads new files under `<state>/mailbox/conductor/`; `[REPORT …]
   started|blocked|unblocked|done|failed` moves the dispatch (the `report-to-state` acceptance);
   `[NEED]` is recorded and delivered to the other repository's live controller, else kept for its
   next start; `[DECISION-REQUEST]` is recorded with `raise-decision-request` and handed to
   `decide::handle_request` (`story:decide-model-adapter`; until it lands, escalated as class O with
   the options parsed from the message); `[RESOURCE]` answered "use conductor build run";
   `[HANDOVER]`/`blocked handover <path>` recorded as a report. One test per form.
5. Cycle: `snapshot start-snapshot`, `publish-board`, `records check`, then dispatch v1: for each
   Active repository with `charters/<repo>.md` and no Running controller, `spawn`, until
   `controllers.max_working`; a `merge: human` repository at `controllers.max_open_reviews` open
   pull requests is skipped; one `cycle` line. Daily: `metric compute` for the previous UTC day and
   `REPORT.md` with deviations only. Tests over a fixture instance.
6. No profile shipped in `.agents/` names `CronCreate`, `Monitor` or `watch run --follow` after this
   story; `crates/conductor/tests/profiles.rs` holds it. `session.rs` `CONDUCTOR_PROMPT` and
   `task conductor:start|restart` are removed; the Taskfile test is updated.

## Scope

`spec/domains/session.yaml`, `spec/domains/dispatch.yaml` (if a new event is needed),
`crates/conductor-model` (regenerated), new `crates/conductor/src/run.rs` (+ `run/handlers.rs`,
`run/mailbox.rs`, `run/dispatch.rs`), `crates/conductor/src/watch.rs` (expose `Watch::pass` lines
as values), `crates/conductor/src/spawn.rs` (`end`, `resume`, `spawn` callable with a resume
point), `crates/conductor/src/metric.rs` (callable), `crates/conductor/src/session.rs`,
`crates/conductor/src/cli.rs`/`cli.yaml`, `Taskfile.yml`, `.agents/conductor.md` (retired),
`.agents/conductor-dev.md`, `crates/conductor/tests/run.rs`, `crates/conductor/tests/profiles.rs`,
`crates/conductor/tests/taskfile.rs`, `website/docs/reference/`.
