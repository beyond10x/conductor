---
format: aep.planning-md/3
id: story:started-report-after-restart
kind: story
status: active
title: A restarted controller's second started report on its dispatch is accepted
scope:
- confidence: inferred
  path: crates/conductor/src/broker.rs
- confidence: cited
  path: crates/conductor/src/run/mailbox.rs
- confidence: inferred
  path: crates/conductor/tests/broker.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: inferred
  path: docs/design/conductor-run.md
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:03Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:04Z", actor: "human:timo", revision: 6}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from conductor 1.0.1. After `stop-controller` and
a restart, the lead reported `started` on its dispatch again, and the mailbox refused it:
`dispatch report-started: DispatchStateConflict: dispatch DSP-… did not move: only a sent
dispatch starts`.

## Acceptance

- A `started` report on a dispatch that is already Started, from the controller restarted for it,
  is accepted without a move and leaves no error in the runtime journal; a test holds it.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor` mailbox report handling in the run loop — cited, the story says "the mailbox refused it" and `crates/conductor/src/run/mailbox.rs:216` is the only mailbox `[REPORT]` handler
- **Files:** `crates/conductor/src/run/mailbox.rs` — cited, the `"started"` arm at line 256 calls `report_started` for every held state but `Blocked` (line 245), and line 88 writes the refusal to the journal
- **Files:** `crates/conductor/tests/run.rs` — cited, the mailbox report tests of `story:report-on-sent-dispatch` are at lines 2103-2260 and the new test belongs beside them
- **Symbols:** `report` and `state_of` in `run/mailbox.rs`, `DispatchState::Started`, `crate::dispatch::report_started`, `Tick::note` — cited
- **Also likely:** `docs/design/conductor-run.md` — inferred, lines 84-86 describe the `started`-on-`Blocked` rule and would gain this one
- **Also likely:** `crates/conductor/src/broker.rs` — inferred, `move_dispatch` (line 247) sends a lead's `started` to the same `report_started` with no state check, so it refuses the same way; the story names only the mailbox
- **Also likely:** `crates/conductor/tests/broker.rs` — inferred, only if the broker path is fixed too
- **Documents:** `docs/design/conductor-run.md` — inferred; the module table in `run/mailbox.rs:13` is in the source file already listed
- **Confidence:** high — the quoted error is produced at `crates/conductor/src/dispatch.rs:283` and reached from `run/mailbox.rs:257`, and the sibling fix for `Blocked` landed in exactly these two files
- **Would collide with:** any unit touching `crates/conductor/src/run/mailbox.rs` (the `report` function or its module table), or adding tests near line 2100-2260 of `crates/conductor/tests/run.rs` — cited
- **Safety fact:** the mailbox reads the held state before it moves anything (`state_of`, `run/mailbox.rs:229`), so a `started` on a `Started` dispatch can return early there without touching `dispatch report-started`, the specified `wrong-state` outcome (`spec/domains/dispatch.yaml:1034`) or the generated `crates/conductor-model` — step 2, unproven — cited
