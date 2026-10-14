---
format: aep.planning-md/3
id: story:decision-reads-charter
kind: story
status: implemented
title: A decision about a repository is checked against its charter's do-not-touch lines
relations:
- decomposes: epic:loop-without-handwork
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T04:34:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T04:34:27Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T04:34:27Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

On 2026-10-14 conductor ordered a manual run of a repository's long-running test that the
repository's charter says not to touch. The order rested on journal evidence alone; it was reversed
twice and cost a restart of the run and a day's delay on its read.

## Acceptance

- `conductor decision record-conductor-decision --repository <R>` prints the "Do not touch" and
  standing-rule lines of R's charter on standard error before it records; a CLI test with a fixture
  charter holds it.
- `.agents/conductor.md` says, under deciding, that a decision about a repository is checked
  against its charter's "Do not touch" lines first; `crates/conductor/tests/profiles.rs` holds the
  sentence.

## Scope

`crates/conductor/src/decision.rs`, `crates/conductor/tests/`, `.agents/conductor.md`,
`crates/conductor/tests/profiles.rs`.
