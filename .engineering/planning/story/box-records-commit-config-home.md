---
format: aep.planning-md/3
id: story:box-records-commit-config-home
kind: story
status: draft
title: A box's records commit reads the bot route's config from the box
summary: The daily tick's records commit in a box ran b10x-gates against the daemon's ~/.config, which the box does not bind, and was refused
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#242
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/run/records.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: crates/conductor/tests/placement.rs
- confidence: cited
  path: crates/conductor/tests/run_records.rs
- confidence: cited
  path: docs/design/architecture-next.md
- confidence: cited
  path: website/docs/guides/run-isolated.md
revision: 3
---
## Why

Issue #242. In a box (`placement: systemd`) the daily tick's records commit is refused at
`commit`: `b10x-gates: protected file ~/.config/b10x/b10x-bot.json unavailable: not found`. The
box's sessions commit through the bot route because their harness settings set `XDG_CONFIG_HOME`
(and `B10X_GATES_POLICY`) to a directory under the instance's cache root, by hand; the runtime,
which runs the daily tick, never sees those, so `b10x-gates` falls back to `$HOME/.config`, the
daemon's home, which the box does not bind.

On main (release 1.19.2 plus waves 65-66):

- The box's start environment is the service's, `HOME` the daemon's
  (`crates/conductor/src/daemon/reconcile.rs:1605-1607`), plus `box.env`, the runtime's log,
  `CLAUDE_CONFIG_DIR` and `XDG_STATE_HOME` (`crates/conductor/src/placement.rs:1826-1848`,
  `request_on`). Nothing sets `XDG_CONFIG_HOME`; conductor names no box config directory at all.
- The records commit runs `b10x-gates bot -- commit` with the runtime's environment as it is and no
  policy (`crates/conductor/src/run/records.rs:418-428`, `bot`; `Route` carries none,
  `run/records.rs:59-73`), built by the daily tick at `crates/conductor/src/run/dispatch.rs:795-802`
  and by `--commit-records` at `crates/conductor/src/run.rs:575`.
- `box.env` may name `XDG_CONFIG_HOME` (`crates/conductor/src/config.rs:508`, `BOX_ENV_OWN`), but
  nothing tells an operator that the bot route of a box needs it, and `conductor doctor` checks no
  bot key for a box.

## Outcome

- A box's start sets `XDG_CONFIG_HOME` to `<cache>/xdg-config` (the instance's cache directory, a
  root the box writes) unless `box.env` names it, as `XDG_STATE_HOME` is set to the state's
  `xdg-state`; the runtime and every session it starts read their tools' config, the bot route's
  included, from one place.
- The records commit's bot route runs with `B10X_GATES_POLICY` set to the instance's `gates_policy`
  when it names one, read as the box reads it.
- `conductor doctor`, for a boxed instance on `forge.route: gates`, names a row
  `box <name> bot route:` when `b10x/b10x-bot.json` is not under the box's `XDG_CONFIG_HOME`, with
  the path to put it at (reported, not a failure).
- Design: `docs/design/architecture-next.md` names the box's config directory beside its state
  directory; `website/docs/guides/run-isolated.md` says where the bot key goes.

Decision (class C): conductor sets the default and `box.env` still overrides it, so an instance that
already names `XDG_CONFIG_HOME` keeps its value; undone by dropping the default.

## Acceptance (tests that fail on the current code)

- `tests/placement.rs` `a_box_start_sets_xdg_config_home_under_the_cache_directory`.
- `tests/placement.rs` `box_env_naming_xdg_config_home_overrides_the_default`.
- `tests/run_records.rs` `the_records_commit_runs_the_bot_route_with_the_instance_gates_policy`.
- `tests/run_records.rs` `the_records_commit_in_a_box_reads_the_bot_key_under_the_box_config_dir`.
- `tests/doctor.rs` `doctor_names_a_boxed_gates_instance_without_a_bot_key_in_its_config_dir`.

## Boundaries

- Conductor does not copy or write the bot key: the operator puts it in the box's config directory.
- Pods (`story:pod-gates-policy`) and `placement: host` are unchanged.
- Hand-written harness settings that set `XDG_CONFIG_HOME` keep working; removing them is the
  operator's step.
