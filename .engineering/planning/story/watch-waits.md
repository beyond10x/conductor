---
format: aep.planning-md/3
id: story:watch-waits
kind: story
status: draft
title: Waits are registered and the watch resumes from a cursor, surviving restarts
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/dispatch.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: spec/scenarios/register-wait.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

On 2026-10-13 conductor re-armed one long wait (a repository's conformance suite: its process
group, its steps file, its `EXIT=` line) ten times, once per session, and each of eight hand-overs
re-derived it. The watch meanwhile reported that repository's controller silent three times while
it waited on that same suite after reporting `blocked`, and a hand-over told the next session to
ignore those lines. The watch itself runs under the harness's `Monitor`, which expires after
30 minutes: 26 Monitor calls and 10 turns that only re-armed it.

## Acceptance

- Spec first: `RegisterWait` (dispatch id, pid or process group, file, pattern), `EndWait` and a
  view `Waits` are declared in `spec/domains/dispatch.yaml`; a scenario
  `spec/scenarios/register-wait.yaml` registers and ends one; `ess verify conform run --target
  interpreted` passes it.
- `conductor dispatch register-wait --dispatch <id> --pid <n> --file <path> --until '<pattern>'`
  records the wait; `conductor watch run` prints `wait ended: <dispatch> <pattern matched|process
  gone>` once; a test in `crates/conductor/tests/watch.rs` ends a fake process and reads the line.
- A controller whose dispatch is `blocked` on a live registered wait is not reported
  `session silent`; the same test file holds it.
- `conductor watch run --since-cursor` prints the lines produced since the cursor kept under
  `<state>/watch/`, moves the cursor and exits, so a cron at `cadence.watch` replaces the expiring
  Monitor; a test in `crates/conductor/tests/watch.rs` reads each line exactly once over two
  calls. `task conductor:restart`'s prompt arms that cron; `crates/conductor/tests/taskfile.rs`
  holds it.
- Design § 8 names both; `.agents/conductor.md` says a wait is registered, never carried in a
  hand-over; `crates/conductor/tests/profiles.rs` holds the sentence.

## Notes

- After `story:watch-goal-health` on `crates/conductor/src/watch.rs`.
- `story:handover-draft` renders the `Waits` view; it comes after this story.
- `story:gate-run` registers its own wait when it starts; it comes after this story.

## Scope

`spec/domains/dispatch.yaml`, `spec/scenarios/register-wait.yaml` (new), `crates/conductor-model`,
`crates/conductor/src/dispatch.rs`, `crates/conductor/src/watch.rs`, `crates/conductor/src/cli.rs`,
`crates/conductor/cli.yaml`, `crates/conductor/tests/watch.rs`, `Taskfile.yml`,
`crates/conductor/tests/taskfile.rs`, `.agents/conductor.md`, `crates/conductor/tests/profiles.rs`,
`docs/design/conductor.md`, `website/docs/reference/cli.md`.
