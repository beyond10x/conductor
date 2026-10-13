---
format: aep.planning-md/3
id: story:watch-disk-floors
kind: story
status: implemented
title: 'The watch reports each disk floor crossed: low, admit and gate stop'
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: docs/design/conductor.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T13:13:53Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T13:14:06Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T13:19:37Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

On 2026-10-13 free space on `/` fell to 14G, below `thresholds.disk_admit` (20G) and
`thresholds.gate_stop` (15G), a release gate's watchdog stopped its suite, and `conductor watch run
--follow` printed nothing: the watch reports only `disk_low` (100G), once per fall, and the host had
been under 100G for days (`crates/conductor/src/watch.rs:1236`, one marker). Conductor learnt of the
stop from the operator.

## Acceptance

- The watch prints `disk low: <n>G free on /` when free space falls under `disk_low`, `disk admit:
  <n>G free on / (builds wait)` under `disk_admit`, and `disk gate stop: <n>G free on / (gates
  stop)` under `gate_stop`, each once per fall: a floor's next report waits until free space has
  risen above that floor again (above `disk_clear` for `disk_low`, `gate_resume` for `gate_stop`,
  `disk_admit` itself for `disk_admit`). A test in `crates/conductor/tests/watch.rs` with a fixture
  free-space sequence 120G, 90G, 18G, 14G, 14G, 16G, 18G, 14G reads exactly: low at 90G, admit at
  18G, gate stop at 14G, nothing at the repeated 14G and at 16G, gate stop again at 14G after 18G.
- `.agents/conductor.md` names the three lines and what conductor does on each (pause new builds on
  `disk admit`, run `conductor gc` and tell the controllers on `disk gate stop`); a test in
  `crates/conductor/tests/profiles.rs` holds the three line names.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`, `.agents/conductor.md`,
`crates/conductor/tests/profiles.rs`, `docs/design/conductor.md`.
