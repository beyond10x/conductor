---
format: aep.planning-md/3
id: story:plan-hygiene
kind: story
status: draft
title: The snapshot flags stale blockers, closable epics and released plans
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/collect/blockers.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/tests/collect_stores.rs
- confidence: cited
  path: crates/conductor/tests/fixtures
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/status.rs
- confidence: cited
  path: spec/domains/observation.yaml
revision: 2
---
## Why

On 2026-10-13 a survey the operator asked for, run by four sub-agents, found planning records that
held or misreported work in five or more repositories: blockers citing upstream issues closed eight
days earlier, blockers and `blocked_by` edges pointing at implemented stories, epics whose children
were all implemented, and release plans still in draft after their release. The cycle's only plan
probe lists open decision blockers and sees none of these. Conductor then spent an hour building
the records' status file by hand from four surveys, after `story:status-board` had shipped,
because the board has no section for them.

## Acceptance

- Spec first: a plan finding (repository, artifact id, kind `stale-blocker|closable-epic|
  released-plan|edge-to-implemented`, cited source) is declared in `spec/domains/observation.yaml`
  with `RecordPlanFinding` and a view `PlanFindings`; `ess specify validate --path spec` exits 0.
- The blockers collector records a finding for a blocker whose cited forge issue is closed or whose
  `blocked_by` target is implemented, an epic or initiative all of whose children are implemented,
  a release plan whose release is published, and a `blocked_by` edge to an implemented artifact;
  a test in `crates/conductor/tests/collect_stores.rs` over a fixture workspace with one of each
  reads four findings and none for a healthy store.
- `snapshot publish-board` lists the findings in a "Planning records" section by repository; a test
  in `crates/conductor/tests/status.rs` holds the heading and a row.
- `.agents/conductor.md` says a cycle sends each repository its own findings in its next dispatch;
  `crates/conductor/tests/profiles.rs` holds the sentence.

## Notes

- Shares `crates/conductor/src/status.rs` and `crates/conductor/tests/status.rs` with
  `story:watch-goal-health` and `story:status-command`; one at a time.
- Pin drift between repositories is `story:dependency-map`, not this story.
- `story:snapshot-duration` measures each collector; this adds work to the blockers collector,
  so it lands after that story.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/collect/blockers.rs`,
`crates/conductor/src/snapshot.rs`, `crates/conductor/src/status.rs`,
`crates/conductor/tests/collect_stores.rs`, `crates/conductor/tests/fixtures`,
`crates/conductor/tests/status.rs`, `.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
