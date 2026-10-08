---
format: aep.planning-md/3
id: story:instance-session-names
kind: story
status: active
title: Every session an instance starts carries the instance's prefix
relations:
- decomposes: epic:multi-instance-host
- supersedes: story:cross-instance-message-guard
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/collect/sessions.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/controller.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/store/controller.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/dispatch.yaml
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T23:21:55Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "proposed", to: "active", at: "2026-10-08T23:21:55Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"review_outcome":5}}}
---
## Why

Session names are one namespace for the whole machine (`specification:multi-instance-host`,
findings 1, 3): two instances, or an instance and the operator's own sessions, can hold one name.
The rename has to land in one piece: the names sessions start under, the names the profile's
commands use, the guard's recipient rule and the collectors' placement (design critic round 2:
renaming conductor alone cuts controllers off, because the guard allows only `conductor`).

## Acceptance

- Spec first: this story owns `SessionPrefix` and `SessionName` in `spec/domains/config.yaml`
  (`spec/drafts/host.yaml` imports them): `instances[].session_prefix` (optional; letters, digits,
  `-`), `Controller.session_name` typed `SessionName`, set to `<prefix>-<repository>`, or to
  `<repository>` when the instance has no prefix. `config validate` refuses two instances with one
  prefix, and two instances without one (tests).
- An instance without a prefix keeps today's names and today's guard rules, so installing this
  changes nothing for a running instance: the switch is a config change conductor makes at a
  restart of its choosing (a test: a one-instance config without a prefix starts `conductor` and
  allows a controller's SendMessage to `conductor`).
- Sessions start under the prefixed names: `Taskfile.yml` `conductor:start` passes
  `-n <prefix>-conductor`, `dev:start` `-n <prefix>-conductor-dev`; the profile's controller start
  and resume commands `-n <controller session name>`, read from `conductor config show`. Tests read the Taskfile commands and the profile.
- The guard: a controller's SendMessage is allowed to `<its prefix>-conductor` (and its own
  sub-agents), denied to a session of another prefix naming both instances; conductor's only to
  sessions of its own prefix. Tests with two instances in one config (this absorbs
  `cross-instance-message-guard`).
- The sessions collector places a session whose name carries the instance's prefix, wherever its
  directory is, and leaves a session with another prefix redacted; a test with two fixture
  sessions shows one placed and one not.
- The separator is `-`: Claude Code accepts it in a background session name today
  (`claude agents --json` lists the running session `conductor-dev`, started by `task dev:start`).

Lookups by name in the other tasks (attach, restart, sessions) are story `instance-scoped-tasks`.

## Scope

`spec/domains/config.yaml`, `spec/domains/dispatch.yaml`, `crates/conductor/src/config.rs`,
`crates/conductor/src/controller.rs`, `crates/conductor/src/store/controller.rs`,
`Taskfile.yml` (`conductor:start`, `dev:start` names), `.agents/conductor.md`,
`crates/conductor/src/guard/rules.rs`, `crates/conductor/src/collect/sessions.rs`,
`crates/conductor/tests/{config,taskfile,guard_rules,instance_sources}.rs`, `crates/conductor-model/`.
