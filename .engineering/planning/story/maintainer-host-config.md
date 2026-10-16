---
format: aep.planning-md/3
id: story:maintainer-host-config
kind: story
status: draft
title: The maintainer is configured once per host in host.yaml, not in any instance's roles
relations:
- decomposes: epic:role-names
- depends_on: story:role-name-aliases
- depends_on: story:instance-names-unique
- informed_by: decision-blocker:maintainer-host-config
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: conductor.example.yaml
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/host.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: crates/conductor/tests/session.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/host.yaml
- confidence: cited
  path: website/docs/reference
revision: 3
---
# Story: maintainer-host-config

## Outcome

The maintainer is configured once per host, in `host.yaml` in the host directory, and no longer in
any instance's `roles`; `task maintainer:start` starts it from there.

## Context

Design: `docs/design/roles-and-surfaces.md` § 2 (the maintainer is one per host, outside every
instance) and § 5b (per host, shared by design: the bridge and the maintainer); `docs/design/conductor.md`
§ 9 (config and instances) and § 12. Decided in `decision-blocker:maintainer-host-config`, option
(B): the host has one file of its own, `host.yaml` in the host directory (`~/.b10x/conductor/host/`),
read by every config file's commands; it holds the per-host roles (maintainer and bridge); the host
registry `instances.json` (story `instance-names-unique`) sits beside it. Today the maintainer is
the role `conductor-dev` in `instances[].roles` (`spec/domains/config.yaml`, `conductor.config.Instance.roles`).

## Domain relations

- Host → host roles (maintainer, bridge), one-to-one each, the host file owns them, they outlive
  every instance — inferable from `decision-blocker:maintainer-host-config` (Decision, option B)
  and `docs/design/roles-and-surfaces.md` § 2; this story declares it first in an `ess/1` document.
- Host → Instance, one-to-many through the registry beside `host.yaml` — from story
  `instance-names-unique` (draft), on which this story depends.

## Acceptance

1. Spec first: `host.yaml` (`conductor.host-config/1`) is declared in `spec/domains/host.yaml`
   (or `spec/domains/config.yaml`): a `roles` list of `conductor.config.Role` holding `maintainer`
   and, where present, `bridge`, any other role refused by its YAML path. `ess specify validate
   --path spec` and `task regen` pass.
2. `conductor config validate` reads `host.yaml` beside the registry: a malformed file is a problem
   named by its path in that file; a `maintainer` or `conductor-dev` role still under
   `instances[].roles` is a note naming its YAML path and saying it moves to `host.yaml` (read from
   there when `host.yaml` names none, for one release). Test: `crates/conductor/tests/config.rs`.
3. `conductor config show --format json` prints the host's roles under `host.roles`, with the
   maintainer's `session_name` as `maintainer` (no instance prefix). Test:
   `crates/conductor/tests/config.rs`.
4. `conductor session start --role maintainer` starts from the `host.yaml` role;
   `task maintainer:start` and `task maintainer:attach` read `host.roles` from
   `conductor config show`. Tests: `crates/conductor/tests/session.rs`,
   `crates/conductor/tests/taskfile.rs`.
5. `docs/config.md` documents `host.yaml` field by field; `conductor.example.yaml` holds no
   maintainer role.

## Scope

`spec/domains/host.yaml`, `spec/domains/config.yaml`, `crates/conductor-model` (generated),
`crates/conductor/src/config.rs`, `crates/conductor/src/host.rs`, `crates/conductor/src/session.rs`,
`Taskfile.yml`, `docs/config.md`, `conductor.example.yaml`, `crates/conductor/tests/config.rs`,
`crates/conductor/tests/session.rs`, `crates/conductor/tests/taskfile.rs`,
`website/docs/reference/**` (generated).

## Out of Scope

- The bridge session itself (epic `host-bridge`, story `bridge-session`), which reads its role from
  `host.yaml`.
- The host registry (story `instance-names-unique`).
- What the maintainer reads (story `maintainer-reads-instances`).
