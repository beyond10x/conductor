---
format: aep.planning-md/3
id: story:host-role-session-unprefixed
kind: story
status: implemented
title: The host's maintainer session carries no instance prefix
summary: host.roles and the instance's maintainer role are named <role>; maintainer:start uses it
refs:
- provider: github
  reference: beyond10x/conductor#214
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/concepts/config.md
- confidence: cited
  path: website/docs/concepts/roles.md
- confidence: cited
  path: website/docs/develop/how-we-work.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:26:22Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-19T18:26:23Z", actor: "human:unknown", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-21T01:32:35Z", actor: "human:unknown", revision: 6, decided_on: {"recorded":{"test_result":2}}}
---
## Why

Issue #214. `conductor config show` printed the host's maintainer under `host.roles` as
`role: conductor-dev, session_name: <first instance's prefix>-conductor-dev`: `host.roles` is the
instance's maintainer role (`crates/conductor/src/config.rs:1879`, `host_maintainer`, shown at
`:7480`), whose `session_name` is derived as every instance role's, `<session_prefix>-<role>`
(`role_session_name`, `:2146`). The maintainer develops conductor for the host, not for one
instance, so the name carried an unrelated instance's prefix and `task maintainer:start`
(`Taskfile.yml:56`, which reads `host.roles[].session_name`) started it under that name. A role of
`globals.roles` (supervisor, teacher) is already named after the role with no prefix (`:3833`).

## Outcome

- The maintainer role (`maintainer`, or its old name `conductor-dev`) derives its session name as
  `<role>`, with no instance prefix, under the instance's roles and under `host.roles`; so
  `session start --role maintainer`, the attach tasks, the dashboard and the loop's restart of a
  gone maintainer (`run/handlers.rs:1009`) all use it.
- A file that writes the name of the release before, `<session_prefix>-<role>`, still validates
  and reads as `<role>`; any other written name is refused by its path, as before.
- Spec: `conductor.config.Role` in `spec/domains/config.yaml` says so.

## Acceptance (fail on main)

- `crates/conductor/tests/config.rs` `the_host_maintainer_s_session_carries_no_instance_prefix`.
- `crates/conductor/tests/taskfile.rs` `the_start_tasks_name_the_session_from_config_show`:
  `dev:start` passes `-n conductor-dev` with a prefix (was `-n a-conductor-dev`).

## Boundaries

The director's and the leads' names keep the prefix. No new validation of a prefix that a bare
maintainer name would carry (a prefix `conductor` beside `conductor-dev`): the host roles of
`globals.roles` are bare names without such a check too.
