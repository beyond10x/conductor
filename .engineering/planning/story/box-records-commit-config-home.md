---
format: aep.planning-md/3
id: story:box-records-commit-config-home
kind: story
status: implemented
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
  path: crates/conductor/tests/run_records.rs
- confidence: cited
  path: docs/design/architecture-next.md
- confidence: cited
  path: website/docs/guides/run-isolated.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T08:50:44Z", actor: "human:unknown", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-20T08:50:45Z", actor: "human:unknown", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-20T12:39:53Z", actor: "human:unknown", revision: 9, decided_on: {"recorded":{"test_result":1}}}
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

Why the default is scoped to the bot route and not set box-wide: the box binds the host's forge
logins and tool config under `HOME` at placement (`crates/conductor/src/placement.rs:1302-1314`,
`LOGINS` and `HOME_FILES`), which gh, glab, git and worktree would stop reading under a box-wide
`XDG_CONFIG_HOME`.

## Outcome

- The records commit's `b10x-gates` runs with `XDG_CONFIG_HOME` set to `box.env`'s value when it
  names one, else `<cache>/xdg-config` when the instance is boxed; the box's start environment is
  unchanged. `placement: host` runs `b10x-gates` with the environment unchanged.
- `Route` carries `policy: Option<PathBuf>` from `instances[].gates_policy`; when set, the bot
  command runs with `B10X_GATES_POLICY=<it>`; when unset, the inherited variable is left as is.
- `conductor doctor`, for a boxed instance on `forge.route: gates`, names a row
  `box <name> bot route:` when `b10x/b10x-bot.json` under that `XDG_CONFIG_HOME` (`box.env`'s, else
  `<cache>/xdg-config`), or the key file it names, is missing, with the path to put it at (reported,
  not a failure). Doctor names the path only, never the contents; conductor neither copies nor reads
  the key. With no gates policy the policy setting is skipped and so is that part of the row.
- Doctor's existing `gates_policy` readability check also covers a boxed instance's policy path
  lying under a root the box binds.
- Design: `docs/design/architecture-next.md` names the bot route's config directory of a box beside
  its state directory; `website/docs/guides/run-isolated.md` says where the bot key goes.

Decision (class C): conductor sets the default for the bot route only and `box.env` still overrides
it, so an instance that already names `XDG_CONFIG_HOME` keeps its value; undone by dropping the
default.

## Acceptance (tests that fail on the current code)

- `tests/run_records.rs` `the_records_commit_runs_the_bot_route_with_the_instance_gates_policy`.
- `tests/run_records.rs` `the_records_commit_passes_the_box_config_dir_to_b10x_gates_when_the_runtime_has_none`.
- `tests/run_records.rs` `an_unboxed_records_commit_sets_no_xdg_config_home`.
- `tests/doctor.rs` `doctor_names_a_boxed_gates_instance_without_a_bot_key_in_its_config_dir`
  (names a row when `b10x/b10x-bot.json` or the key file it names is missing).
- `tests/doctor.rs` `doctor_reads_the_bot_key_under_box_env_xdg_config_home`.
- `tests/doctor.rs` `doctor_names_no_bot_route_row_for_an_unboxed_instance`.

## Boundaries

- Conductor does not copy, read or write the bot key: the operator puts it in the box's config
  directory.
- The box's start environment and `BOX_ENV_OWN` (`config.rs`) are unchanged; `box.env` overrides.
- Pods (`story:pod-gates-policy`) and `placement: host` are unchanged.
- Hand-written harness settings that set `XDG_CONFIG_HOME` keep working; removing them is the
  operator's step.
- `task docs-generate` rewrites `website/docs/status.md` and `website/data/status.json`; those are
  regenerated after merge, not merged by hand.
