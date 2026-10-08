---
format: aep.planning-md/3
id: story:portable-trust
kind: story
status: implemented
title: Workspace trust is written by the conductor CLI, not GNU shell
tags:
- first-user-review
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:03:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T20:03:11Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:45Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Reported by a first user on macOS (Darwin, BSD userland) against `21734cd`, one local repository, observe-only first cycle. `task trust` uses `stat -c %Y` and `chmod --reference`, which BSD userland lacks.

## Acceptance

- Spec first: a `conductor trust` command (leaf in `crates/conductor/cli.yaml` / the spec) that
  marks every checkout under `checkouts.root` and the instance's records directory as trusted in
  `~/.claude.json`, with the same refusals `task trust` has today (records path with a line break,
  not a directory) and an atomic write that keeps the file's mode and refuses when the file
  changed during the run (a content hash, not a one-second mtime).
- `task trust` calls it.
- Tests run it against a scratch HOME.

## Scope

`spec/`, `crates/conductor/cli.yaml`, `crates/conductor/src/` (new module), `Taskfile.yml`, `crates/conductor/tests/`.
