---
format: aep.planning-md/3
id: story:session-envelope-model
kind: story
status: draft
title: A session's envelope is requested, measured and recorded
relations:
- decomposes: epic:session-confinement
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/collect/sessions.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/observation.yaml
revision: 7
---
## Why

architecture-decision-record:external-confinement: conductor confines each session from outside and measures the box at the boundary, as
metaharness does with its `ProcessEnvelope` (request, provider, measurement, strict refusal).

## Acceptance

- Spec first: `conductor.config` gains `confinement` per instance (`mode: strict | report | off`,
  default `report`) and per role the envelope request: `limits` (`memory_max`, `cpu_weight`,
  `tasks_max`), `writable` (paths; the defaults: the session's checkout, its managed trees, the build
  cache, scratch, the harness's own state directory; conductor also its records), `network`
  (`open` only, for now). `conductor.observation` gains the envelope measured for a session:
  provider, cgroup path, the limits read back, the writable paths applied, or "unboxed" with why.
- `conductor doctor` reports which providers this host supports and why not.
- Tests: config parsing and refusal; a measurement recorded and shown by `conductor snapshot sessions`.

## Scope

`spec/domains/config.yaml`, `spec/domains/observation.yaml`, `crates/conductor/src/config.rs`,
`crates/conductor/src/doctor.rs`, `crates/conductor/src/collect/sessions.rs`, `crates/conductor-model/`,
`crates/conductor/tests/`.
