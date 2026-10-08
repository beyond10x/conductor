---
format: aep.planning-md/3
id: story:instance-session-names
kind: story
status: draft
title: Every session an instance starts carries the instance's prefix
relations:
- decomposes: epic:multi-instance-host
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/src/collect/sessions.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/controller.rs
- confidence: cited
  path: crates/conductor/src/store/controller.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/dispatch.yaml
revision: 9
---
## Why

Session names are one namespace for the whole machine (`specification:multi-instance-host`,
findings 1, 3): two instances, or an instance and the operator's own sessions, can hold one name.

## Acceptance

- Spec first: this story owns the types `SessionPrefix` and `SessionName` in
  `spec/domains/config.yaml` (`spec/drafts/host.yaml` imports them from there):
  `instances[].session_prefix` (default the instance name; letters, digits, `-`), and
  `Controller.session_name` typed `SessionName`, set to `<prefix>-<repository>`. `config validate`
  refuses two instances with one prefix (test).
- The profile's start and resume commands name conductor `<prefix>-conductor`, conductor-dev
  `<prefix>-conductor-dev` and a controller `<prefix>-<repository>`; a test reads them.
- The sessions collector places a session whose name carries the instance's prefix, wherever its
  directory is, and leaves a session with another prefix redacted; a test with two fixture
  sessions shows one placed and one not.
- The separator is `-`. Before anything else, a probe starts `claude --bg -n <prefix>-probe` once;
  if Claude Code refuses the name, the story stops and reports, and the separator is re-decided.

Taskfile lookups are story `instance-scoped-tasks`.

## Scope

`spec/domains/config.yaml`, `spec/domains/dispatch.yaml`, `crates/conductor/src/config.rs`,
`crates/conductor/src/controller.rs`, `crates/conductor/src/store/controller.rs`,
`.agents/conductor.md`, `crates/conductor/src/collect/sessions.rs`,
`crates/conductor/tests/config.rs`, `crates/conductor/tests/instance_sources.rs`.
