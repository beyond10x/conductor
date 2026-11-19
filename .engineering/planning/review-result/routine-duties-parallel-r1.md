---
format: aep.planning-md/3
id: review-result:routine-duties-parallel-r1
kind: review-result
status: active
title: Parallel-safety critic, round 1, epic routine-duties-in-loop
summary: 'changes: no scopes recorded, shared generated crate and alert module without edges'
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
revision: 1
---
changes

No story records a scope (`aep plan artifact scope`), so `aep plan artifact waves` reads all four
as unassessed, which looks the same as safe. From the bodies and the code read by the design
critic, the surfaces are:

- `story:unit-merge-policy`: `spec/domains/config.yaml`, `crates/conductor-model` (generated),
  `crates/conductor/http.yaml` (re-pinned `source_digest`), `crates/conductor/src/config.rs`,
  `crates/conductor/src/spawn.rs`, `crates/conductor/src/spawn/`, `crates/conductor/tests/work_start.rs`,
  `docs/config.md`, `website/docs/concepts/the-loop.md`, `website/docs/reference/config.md` (generated).
- `story:failing-start-recreates-box`: `crates/conductor/src/daemon/reconcile.rs`,
  `crates/conductor/src/placement.rs`.
- `story:need-starts-unit-of-work`: `crates/conductor/src/run/mailbox.rs`,
  `crates/conductor/tests/mailbox.rs`.
- `story:loop-alerts-when-it-cannot-act`: `spec/domains/dispatch.yaml`, `crates/conductor-model`,
  `crates/conductor/http.yaml`, `crates/conductor/src/alert.rs` (new), `crates/conductor/src/lib.rs`,
  `crates/conductor/src/run/handlers.rs`, `crates/conductor/src/run/dispatch.rs`,
  `crates/conductor/tests/run.rs`, `website/docs/reference/` (generated).

Collisions: `unit-merge-policy` and `loop-alerts-when-it-cannot-act` both regenerate
`crates/conductor-model` and re-pin `crates/conductor/http.yaml`. Once the design findings move the
alert into a shared `crate::alert`, `failing-start-recreates-box` and `need-starts-unit-of-work`
read what `loop-alerts-when-it-cannot-act` produces and need a `depends_on` edge to it. No edge is
recorded.

```findings
[
{"file":".engineering/planning/story/unit-merge-policy.md","line":1,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"no recorded scope on any of the four stories"},
{"file":"crates/conductor/http.yaml","line":1,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"unit-merge-policy and loop-alerts-when-it-cannot-act both regenerate crates/conductor-model and re-pin http.yaml; no ordering edge"},
{"file":".engineering/planning/story/failing-start-recreates-box.md","line":1,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"failing-start-recreates-box and need-starts-unit-of-work need a depends_on edge to the story that adds crate::alert"}
]
```
