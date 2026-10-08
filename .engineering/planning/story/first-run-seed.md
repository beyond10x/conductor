---
format: aep.planning-md/3
id: story:first-run-seed
kind: story
status: implemented
title: A fresh instance has a first hand-over and a north star
tags:
- first-user-review
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:03:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T20:03:11Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:45Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Reported by a first user on macOS (Darwin, BSD userland) against `21734cd`, one local repository, observe-only first cycle. The start prompt reads "the hand-over committed last" in `docs/handoff/` and `NORTHSTAR.md`;
on a fresh instance neither exists and the README does not say to create them.

## Acceptance

- Spec first: `conductor init` writes a records directory skeleton for the active instance (a git
  repository with `NORTHSTAR.md`, `docs/handoff/<date>.md` with a first task, `rules.md` with the
  three role sections, `decisions/`, `dispatches/`, `charters/`), refusing to overwrite.
- README Configure points at it.

## Scope

`spec/`, `crates/conductor/cli.yaml`, `crates/conductor/src/`, `README.md`.
