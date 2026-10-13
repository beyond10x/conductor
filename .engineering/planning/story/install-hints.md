---
format: aep.planning-md/3
id: story:install-hints
kind: story
status: active
title: Doctor and setup print an install command for each missing program
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T15:22:36Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T15:22:37Z", actor: "human:timo", revision: 3}
---
## Why

The team's doctor runs on 2026-10-13 showed `task`, `claude`, `jq`, `worktree` and `gh` missing on
Linux and macOS, and nobody in the check thread said where to get them (brain's case list, cases 1-5
and 7). `conductor setup` prints `to install: <program>` and no command.

## What

For each missing or too-old program, doctor and setup print one install line for this machine's
system, read from `uname` (and the package manager found on `PATH`): for example `brew install
go-task jq gh` on macOS, the distribution's package or the upstream release URL on Linux, the
Claude Code install page for `claude`, and the Beyond10x setup
(`https://github.com/beyond10x/agentplugins/releases/latest/download/SETUP.md`) for `aep`, `ess`
and `worktree` at the pins doctor names.

## Acceptance

- With `task` missing on a fake macOS (`uname -s` prints `Darwin`, `brew` on `PATH`), doctor prints
  `task: missing` and, under it, `  install: brew install go-task`.
- With `jq` missing on a fake Linux with `apt-get` on `PATH`: `  install: sudo apt-get install jq`;
  with no package manager known: the upstream release URL.
- `aep`, `ess` and `worktree`, missing or older than their pin, print the Beyond10x setup line with
  the pin.
- `conductor setup --apply` prints the same lines as `to install:` commands.
- SETUP.md step 4 and the getting-started page show one such line.

## Scope

`crates/conductor/src/doctor.rs` (the printed lines), `crates/conductor/src/setup.rs`,
`crates/conductor/tests/doctor.rs`, `crates/conductor/tests/setup.rs`, `SETUP.md`,
`website/docs/getting-started.md`.
