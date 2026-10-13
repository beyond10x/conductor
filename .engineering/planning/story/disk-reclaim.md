---
format: aep.planning-md/3
id: story:disk-reclaim
kind: story
status: implemented
title: Under a disk floor, conductor frees disk itself instead of waiting
relations:
- decomposes: epic:disk-protocol
- depends_on: story:disk-protocol-spec
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T17:12:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T17:12:18Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T18:12:43Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

On 2026-10-13 work across the instance stopped twice because free disk on `/` fell under the build
floor: builds and gates waited for space while nothing freed it. The watch reports the floors
(`disk low`, `disk admit`, `disk gate stop`), and `conductor gc` can measure and free build output,
but nothing connects the two, so a floor turns into a wait.

## What

`conductor gc reclaim` (declared first in the spec), run by the watch when free disk falls under
`disk_admit` and by conductor's cycle: it applies, in order, until free disk is back over
`disk_admit` plus a margin, the reclaimable rows of `conductor gc` that need no decision — build
output of idle managed trees (no live session, no running gate), build output stripped from
worktree archives, caches past their age — and prints each step with the bytes it freed. It never
touches a tree a live session works in or a running gate's target. When it cannot reach the floor
it prints what is left by path and size, and the watch reports `disk floor not reached: <n>G
short` so conductor raises it to the operator.

## Acceptance

- Spec first: `gc reclaim` and its outcomes (reclaimed, floor not reached) in the spec; `ess
  specify validate --path spec` exits 0.
- A test over a fixture of two idle trees with build output, one tree with a live session and an
  archive with build output, and a fake free-disk reading: `gc reclaim` frees the idle trees and
  the archive, leaves the live one, prints each step with its bytes, and stops once the reading is
  over the floor.
- A test where the fixture cannot reach the floor: the output lists what is left and exits 1; the
  watch prints `disk floor not reached`.
- The watch runs `gc reclaim` once per fall under `disk_admit`, not on every pass.
- `.agents/conductor.md`: under a floor conductor runs `gc reclaim` in the same cycle and reports
  to the operator only if it falls short.

## Scope

`spec/domains/config.yaml` or the gc spec, `crates/conductor/src/gc.rs`, `crates/conductor/src/gc/`,
`crates/conductor/src/watch.rs`, `crates/conductor/tests/gc.rs`, `crates/conductor/tests/watch.rs`,
`.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
