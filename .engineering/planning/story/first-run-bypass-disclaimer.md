---
format: aep.planning-md/3
id: story:first-run-bypass-disclaimer
kind: story
status: implemented
title: A first start explains the bypass-permissions disclaimer instead of exiting
tags:
- first-user-review
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:03:12Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T20:03:12Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:46Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Reported by a first user on macOS (Darwin, BSD userland) against `21734cd`, one local repository, observe-only first cycle. `task conductor:start` exited: "requires accepting the disclaimer first. Run `claude
--dangerously-skip-permissions` once interactively."

## Acceptance

- The README's Run section says the one-time interactive acceptance, and that a role settings file
  with `skipDangerousModePermissionPrompt: true` avoids it.
- `conductor:start` refuses with that instruction when the acceptance is missing and the role's
  settings do not skip the prompt (detect from `~/.claude.json` or the settings file; say which).

## Scope

`README.md`, `Taskfile.yml` `conductor:start`, `crates/conductor/tests/taskfile.rs`.
