---
format: aep.planning-md/3
id: story:profiles-from-review
kind: story
status: implemented
title: The profiles carry what the session review found missing
tags:
- session-review
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:27:23Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T20:27:23Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:46Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Found by a review of an instance's sessions after its records moved out of the product checkout (two independent reviewers). The instance's `rules.md` now carries the fixes, but three belong in the generic profiles:
conductor relays every `rules.md` commit to running controllers; below `thresholds.disk_low`
cleanup is class C and dispatched to the owners; a controller reads both its section and the
standing rules, and reports to conductor at PR open, CI red and merge.

## Acceptance

- `.agents/conductor.md` and `.agents/repo-controller.md` state these, citing the config keys.
- A test in `crates/conductor/tests/` reads both profiles for the sentences.

## Scope

`.agents/conductor.md`, `.agents/repo-controller.md`, `crates/conductor/tests/`.
