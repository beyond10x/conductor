---
format: aep.planning-md/3
id: story:install-prerequisites
kind: story
status: implemented
title: The README installs every prerequisite with a command
tags:
- first-user-review
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:03:12Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-08T20:03:12Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:45Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

A first user on macOS: README Install lists `aep` and `worktree` without install commands; the
first snapshot failed "start `aep`: No such file or directory". Conductor's collectors and gate
start `aep`, `worktree`, `ess`, `gh`, `git`, `claude`, `jq` and `task`, and nothing says which
versions it was tested with or checks that they are there. The ecosystem already has the
installer for its own tools: `b10x` (agentplugins), with `b10x init <products>` (plugins and CLIs
from checksummed release archives), `b10x pin <cli> <version>` (`b10x.toml`) and `b10x check`
(a session-start drift check that prints only problems).

## Acceptance

- `b10x.toml` at the repository root pins the ecosystem CLIs conductor is tested with
  (`aep`, `ess`, `worktree`), written with `b10x pin`; `task check` uses the pinned `ess`.
- README Install: (1) the `b10x` plugin (`/plugin marketplace add beyond10x/agentplugins`,
  `/plugin install b10x@b10x`), (2) `b10x init aep,ess,worktree` and its apply step, (3) the
  system tools (`gh`, `git`, `jq`, `task`, `claude`) with one install line each, every command
  checked to work.
- Spec first: `conductor doctor` reports each required program as present with its version, or
  missing; an ecosystem CLI older than its `b10x.toml` pin is reported. `conductor snapshot`
  refuses at start, naming the missing program, instead of failing inside a collector.
- The settings files an instance writes for its roles may carry a SessionStart hook `b10x check`;
  `docs/config.md` § Sessions without the user's config shows it.

## Scope

`b10x.toml` (new), `README.md`, `Taskfile.yml` `check`, `spec/`, `crates/conductor/cli.yaml`,
`crates/conductor/src/` (doctor), `docs/config.md`, `crates/conductor/tests/`.
