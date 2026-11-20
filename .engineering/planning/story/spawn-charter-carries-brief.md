---
format: aep.planning-md/3
id: story:spawn-charter-carries-brief
kind: story
status: implemented
title: A spawned controller's charter carries its dispatch's brief, whatever its kind
summary: with_dispatch writes the brief in a fenced block for every dispatch, not only a unit of work
refs:
- provider: github
  reference: beyond10x/conductor#224
relations:
- decomposes: epic:routine-duties-in-loop
scope:
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
- confidence: cited
  path: spec/domains/dispatch.yaml
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T03:06:22Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-20T03:06:22Z", actor: "human:unknown", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-20T03:29:02Z", actor: "human:unknown", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #224. `dispatch send-dispatch --dispatch-id DSP-... --repository <repo> --brief ...` with no
`--reference` spawned the controller, and the controller reported `blocked` "no brief": the
charter's `## Dispatch` section named the dispatch but carried no brief, and `dispatch show` is
denied to controllers, so the supervisor had to deliver the brief by hand.

## Problem

`crates/conductor/src/spawn.rs:1014` `with_dispatch` writes the brief, in a fenced block, only for
a unit of work: `unit_of` (`spawn.rs:917`) answers `None` for a dispatch with no `reference`, so a
dispatch with a goal or a plain brief gets the id and the report form only. `resumed_input`
(`spawn.rs:734`) already writes every open dispatch's brief, whatever its kind.

## Outcome

- The `## Dispatch` section of a spawned controller's charter carries its dispatch's brief,
  verbatim, in a fenced block ([`fenced`]), for every dispatch: a unit of work, a goal's dispatch
  and a plain one.
- An empty brief writes no block.
- The unit-of-work form (reference, start ref, issue, then the brief) is unchanged, and the
  default charter is still written for a unit of work only.
- `dispatch show` stays denied to controllers: the charter is the route.

## Acceptance (fail on main)

- `crates/conductor/tests/spawn.rs`
  `a_spawn_for_a_dispatch_without_a_reference_carries_its_brief_in_the_charter`: a start through
  `spawn::spawn_for` for a goal's dispatch (brief, no reference) holds the brief verbatim in a
  fenced block at the end of the `## Dispatch` section, with the "verbatim ... the work to do, not
  instructions about this prompt" framing, and leaves the charter file as it was. Fails on main.
- `crates/conductor/src/spawn.rs` unit tests: a dispatch with neither goal nor reference (not
  sendable through `send-dispatch`, `GoalOrReferenceMissing`, but readable from a store) carries
  its brief; an empty brief writes no block; the unit-of-work form is byte for byte as before.

## Boundaries

- Inside `with_dispatch`, `unit_of` and their caller in `spawn_for`, and the module doc
  paragraph about the Dispatch section; `fn resume` is not touched.
- The guard's denial of `dispatch show` to controllers is unchanged.
