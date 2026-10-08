---
format: aep.planning-md/3
id: story:session-envelope-model
kind: story
status: draft
title: A session's envelope is requested, measured and recorded
relations:
- decomposes: epic:session-confinement
- depends_on: story:host-grants
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
revision: 8
---
## Why

architecture-decision-record:external-confinement: conductor confines each session from outside and measures the box at the boundary, as
metaharness does with its `ProcessEnvelope` (request, provider, measurement). This story is the
data: what is requested and what was measured. The seam that acts on it is
`story:session-envelope-seam`.

## Acceptance

- Spec first, in `spec/domains/config.yaml`:
  - per instance, `confinement`: `mode: strict | report | off` (default `report`), `provider:
    systemd | none` (default `systemd`; `substrate` is added by `story:substrate-client`), and
    `limits` for the instance as a whole (`memory_max`, `cpu_weight`, `tasks_max`), which takes over
    `instances[].limits` from the archived `instance-cgroup-limits`;
  - per role, the request: `limits` (same three keys), `writable` (paths; defaults: the session's
    checkout, its managed trees, the build cache, scratch, the harness's own state directory, and
    for conductor its records), `network` (`open` only).
- Not modelled, with the reason written in the spec's description: readable paths (bwrap binds the
  root read-only; nothing narrower is planned), programs (the shell stays open; a program list
  belongs to `story:substrate-tool-server`), lifetime (sessions are long-lived and ended by
  conductor).
- Spec first, in `spec/domains/observation.yaml`: the envelope measured for a session: provider,
  cgroup path, the limits read back, the writable paths applied, or `unboxed` with the reason.
- Tests: config parsing; `config validate` refuses an unknown mode or provider and a role limit
  above its instance's; `conductor snapshot sessions` shows a fixture measurement. Recording a real
  one is the providers' work.
- `docs/config.md` documents the new keys.

## Scope

`spec/domains/config.yaml`, `spec/domains/observation.yaml`, `crates/conductor/src/config.rs`,
`crates/conductor/src/collect/sessions.rs`, `crates/conductor-model/`, `crates/conductor/tests/`,
`docs/config.md`. After `host-grants`, which comes after `instance-scoped-tasks` and
`instance-session-names`: all four change `config.yaml`, `config.rs` and the generated crate, and
`instance-session-names` also changes `collect/sessions.rs`.
