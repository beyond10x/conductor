---
format: aep.planning-md/3
id: story:resumable-gates
kind: story
status: implemented
title: A long gate reruns only what failed
relations:
- decomposes: epic:conductor-autonomy
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T09:16:56Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-13T09:16:56Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-13T09:16:56Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Why

A repository's pre-release gate ran 8–18 hours per attempt and failed four times on test-harness
defects, never on product code; each fix restarted the whole suite from its first step, so a release
prepared on 2026-10-11 was still in its gate on 2026-10-13 (conductor's reports, 2026-10-13).

## Acceptance

- `.agents/repo-controller.md` requires, for any gate over an hour, that after a failure the
  controller reruns only the failed steps and the steps that depend on them, and keeps one full run
  for the release commit; a test in `crates/conductor/tests/profiles.rs` holds that wording.
- `.agents/conductor.md`'s dispatch rules state the same rule for the charters conductor writes; the
  same test file holds that wording.

## Notes

- Shares `.agents/conductor.md`, `.agents/repo-controller.md` and
  `crates/conductor/tests/profiles.rs` with the other profile stories of `epic:conductor-autonomy`.
  It is first in their order: resumable-gates, work-beside-long-gate, claims-cite-source,
  operator-items-age, gc-before-pause, self-arming-restart.

## Scope

`.agents/repo-controller.md`, `.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
