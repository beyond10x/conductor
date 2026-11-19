---
format: aep.planning-md/3
id: review-result:routine-duties-parallel-r2
kind: review-result
status: active
title: Parallel-safety critic, round 2, epic routine-duties-in-loop
summary: approve
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
revision: 1
---
approve

Every story records its scope (`aep plan artifact scope`), cited where the file was read and
inferred for the generated and new files. Within the set, `aep plan artifact waves --kind story
--status draft` reports collisions only between `story:loop-alerts-when-it-cannot-act` and
`story:unit-merge-policy` (`crates/conductor-model`, `crates/conductor/http.yaml`,
`website/docs/reference`), and the `depends_on` edge orders them. The waves put the alert story
first and `story:failing-start-recreates-box` and `story:need-starts-unit-of-work` in one later
wave.

**Grouping:**
- Wave A: `story:loop-alerts-when-it-cannot-act` alone (spec `dispatch.yaml`, regenerated crate,
  `crate::alert`, `run/handlers.rs`, `run/dispatch.rs`, `tests/run.rs`).
- Wave B, at once: `story:unit-merge-policy` (spec `config.yaml`, regenerated crate, `config.rs`,
  `spawn.rs`, `tests/work_start.rs`), `story:failing-start-recreates-box`
  (`daemon/reconcile.rs`, `placement.rs`), `story:need-starts-unit-of-work` (`run/mailbox.rs`,
  `tests/mailbox.rs`). No file is shared among the three.

**Outside the set, not a finding:** the wave-59 units `story:idle-controller-yields-place` and
`story:dispatch-v1-once-per-goal` touch `run/dispatch.rs` and `tests/run.rs`, as the alert story
does: start the alert story after they merge, or in its own tree and merge after them. Other
draft and active stories touching `daemon/reconcile.rs`, `placement.rs`, `spawn.rs` and
`spec/domains/config.yaml` are not in a wave now.

```findings
[]
```
