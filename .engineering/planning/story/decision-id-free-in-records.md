---
format: aep.planning-md/3
id: story:decision-id-free-in-records
kind: story
status: implemented
title: A new decision id is free in the store and in the hand-written decision files
scope:
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/tests/decision.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T02:22:17Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T02:22:17Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T02:34:02Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

On 2026-10-12 `conductor decision record-operator-decision` without `--decision-id` allocated
`DEC-<date>-<nn>`, an id the instance's hand-written `decisions/2026-10.jsonl` already held, because
the store allocates the next id from its own records only and the hand-written lines were never
imported (`story:log-import` has not run). Conductor recorded the decision again by hand under the
next free id (reported by conductor, 2026-10-13). The conductor profile warns about this but the
command does not prevent it.

## Acceptance

- Given no `--decision-id`, the decision record commands allocate an id that is free both in the
  store and in the instance's `decisions/*.jsonl` (read from the records directory the config
  names); a test with a records fixture holding `DEC-<date>-01` and `-02` allocates `-03`.

## Scope

`crates/conductor/src/decide.rs`, `crates/conductor/tests/decision.rs`.
