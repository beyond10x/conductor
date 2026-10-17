---
format: aep.planning-md/3
id: story:session-home-spike
kind: story
status: draft
title: One controller starts with its own HOME, CLAUDE_CONFIG_DIR and TMPDIR under the instance, and what breaks is recorded with its fix
relations:
- decomposes: epic:conductor-run
revision: 1
---
## Why

A spawned session gets no `HOME`, `CLAUDE_CONFIG_DIR`, `CARGO_HOME` or `TMPDIR` of its own
(`crates/conductor/src/confine/systemd.rs:381-406`), so its transcripts, trust, plugins and scratch
land in the operator's home. The operator (2026-10-17): nothing of conductor's may leak into the
host. Setting these blindly breaks what the harness needs (plugins, trust, the sessions list the
collector reads, the transcripts the watch reads).

## Outcome

One controller started with `HOME`, `CLAUDE_CONFIG_DIR` and `TMPDIR` under
`<instance>/home/`, and a record of what then fails and the fix for each, so the setting can become
the default in a follow-up story with no guessing.

## Acceptance

1. `conductor spawn --repository <repo> --home-spike` (a hidden flag, removed by the follow-up)
   starts the controller with `HOME`, `CLAUDE_CONFIG_DIR` and `TMPDIR` pointed under the
   instance's state directory, the harness trust file written there, and `PATH` as today.
2. `docs/analysis/session-home-spike.md` records, from one real start on the host: whether the
   session starts; whether `claude agents --json` from the operator's environment lists it;
   whether the b10x plugins (aep, ess, worktree skills) resolve; whether the watch finds its
   transcript; whether `gh`, `git` identity and the bot route work; whether `conductor build run`
   works; each with the command run and its output, and the fix for each failure (a symlink, a
   copied file, a config key, or "not fixable without the fence").
3. The follow-up story `story:session-home` is drafted from that record with the exact changes.

## Scope

`crates/conductor/src/spawn.rs`, `crates/conductor/src/confine/systemd.rs`,
`docs/analysis/session-home-spike.md`, `.engineering/planning/story/session-home.md` (draft).
