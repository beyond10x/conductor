---
format: aep.planning-md/3
id: story:claims-cite-source
kind: story
status: implemented
title: A news or status claim cites the source it was checked against
relations:
- decomposes: epic:conductor-autonomy
- depends_on: story:work-beside-long-gate
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T09:16:57Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-13T09:16:57Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-13T09:16:58Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Why

A news item conductor sent named the wrong issue as fixed and had to be corrected; conductor's own
fault log names reading a suite's completion from its steps file without the runner's exit line
(2026-10-12). `story:watch-unsent-report` is adjacent but does not cover it.

## Acceptance

- `.agents/conductor.md` requires every news or status claim about a release, an issue or a gate,
  in a report or the daily brief, to cite the source it was read from (the release page, the issue,
  the runner's exit line) and to be checked against that source before it is sent; a test in
  `crates/conductor/tests/profiles.rs` holds the rule.

## Notes

- Conductor has no separate news prompt: its reports and the daily brief come from the profile's
  cycle and daily-cycle steps, so the rule lives there.
- Shares `.agents/conductor.md` and `crates/conductor/tests/profiles.rs` with the other profile
  stories of `epic:conductor-autonomy`; it depends on `story:work-beside-long-gate` so it lands
  after it on those files.

## Scope

`.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
