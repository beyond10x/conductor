---
format: aep.planning-md/3
id: story:sessions-without-global-config
kind: story
status: implemented
title: Sessions start without the user's global Claude configuration
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:02:58Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T20:02:58Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:45Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

The operator: "make sure we do not rely on global claude config at all". Every session (conductor,
conductor-dev, each controller) loaded the user's `~/.claude/settings.json` (env, plugins,
attribution, hooks) and `~/.claude/CLAUDE.md`, and found its adapter in `~/.claude/agents/`.
Probed: `claude -p --setting-sources project,local --settings <file>` takes env and
`enabledPlugins` from `<file>` alone, and `claudeMdExcludes` in that file keeps the global
`CLAUDE.md` out of the context.

## Acceptance

- A role may name a `profile`; its session starts with `--append-system-prompt-file <profile>`
  and no `--agent`.
- Every session start the product writes passes `--setting-sources project,local` and the role's
  `--settings`.
- `docs/config.md` says what an instance's settings file must carry when user settings are off.

## Scope

`spec/domains/config.yaml`, `crates/conductor/src/config.rs`, `Taskfile.yml`, `.agents/conductor.md`,
`docs/config.md`, `crates/conductor/tests/{taskfile,config}.rs`, `crates/conductor-docs/src/config.rs`.
