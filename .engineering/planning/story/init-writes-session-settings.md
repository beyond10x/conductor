---
format: aep.planning-md/3
id: story:init-writes-session-settings
kind: story
status: draft
title: conductor init writes the session roles' settings files
summary: init leaves no settings file wiring the guard; they were written by hand
revision: 2
---
## Why

Reported 2026-11-07 from a running instance: after `conductor init`, the settings files the
director and controller roles start with had to be written by hand. Verified in the code:
`crates/conductor/src/init.rs` writes the records skeleton (`NORTHSTAR.md`, the hand-over,
`rules.md`, `README.md`, `.engineering/workspace.yaml`, `decisions/`, `dispatches/`,
`charters/`) and no settings file; `roles[].settings` defaults to none (`docs/config.md`, the
`roles` row), and the files that wire the guard as a PreToolUse hook exist only in the product
checkout (`.claude/controller-settings.json`, `.claude/conductor-settings.json`).

## Acceptance

- On an instance whose roles name no settings file, `conductor init` writes a settings file for
  each session role that wires the guard as a PreToolUse hook, pinned to the instance, and prints
  each path it wrote; or it names, on one line per role, the settings file to write and the
  command that writes it.
- A settings file that exists is left as it is, and `conductor init` says so.
- After `conductor init` on a fresh instance, `conductor spawn` refuses no repository for a
  missing settings file (`NoProfile`).
