---
format: aep.planning-md/3
id: story:handover-draft
kind: story
status: draft
title: conductor handover draft renders a hand-over's open work from the store
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/handover.rs
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/tests/handover.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: spec/domains/session.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

Conductor wrote nine hand-overs on 2026-10-13, 31 KB in all. About three quarters could come from a
command: open-dispatch tables (36%), a "first task" section re-arming jobs (26%, still present after
`story:self-arming-restart` shipped), and the same long wait re-derived eight times. Headers pointed
at up to three earlier hand-overs, and one operator word was pointed at six times though rules.md
already held it. The list of operator items lived only in hand-overs while `decision hands-todo` was
empty, and three disk items needing the operator's word vanished from every later hand-over without
a decision.

## Acceptance

- Spec first: a `HandoverDraft` view (open dispatches, pending requests, hands-todo, registered
  waits, decisions since the lease was taken, free disk) is declared in
  `spec/domains/session.yaml`; `ess specify validate --path spec` exits 0.
- `conductor handover draft` prints a markdown hand-over with those six sections from the store and
  a final `## Notes` section left empty for the session; over a fixture store with one row of each
  a test in `crates/conductor/tests/handover.rs` reads each row in its section and no section
  naming cron jobs or the watch.
- `.agents/conductor.md` moves "a hand-over keeps only open work: it does not list the jobs or the
  watch" from the restart bullet to the general hand-over rule, and adds: a hand-over starts from
  `conductor handover draft`, never points at an earlier hand-over, and an operator item is
  recorded with `decision escalate-request` when first written. A test in
  `crates/conductor/tests/profiles.rs` holds the three sentences.

## Notes

- After `story:records-from-commands` (dispatches, requests and decisions reach the store) and
  `story:watch-waits` (the `Waits` view).
- Shares `.agents/conductor.md` and `crates/conductor/tests/profiles.rs` with the other profile
  stories of this set.

## Scope

`spec/domains/session.yaml`, `crates/conductor-model`, `crates/conductor/src/handover.rs` (new),
`crates/conductor/src/lib.rs`, `crates/conductor/src/cli.rs`, `crates/conductor/cli.yaml`,
`crates/conductor/tests/handover.rs` (new), `.agents/conductor.md`,
`crates/conductor/tests/profiles.rs`, `website/docs/reference/cli.md`.
