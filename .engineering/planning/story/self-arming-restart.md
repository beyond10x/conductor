---
format: aep.planning-md/3
id: story:self-arming-restart
kind: story
status: draft
title: A restarted conductor arms its cycles and watch from the config, not the hand-over
relations:
- informed_by: story:restart-message
- decomposes: epic:conductor-autonomy
- depends_on: story:gc-before-pause
- depends_on: story:status-board
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .claude/conductor-settings.json
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
revision: 4
---
## Why

Conductor wrote 33 hand-overs between 2026-10-08 and 2026-10-13 (11, 5, 3, 1, 8, 5 per day), each a
restart at its context threshold; every new session re-arms its cron jobs, its lease and its watch by
hand from the hand-over's first section. `story:controller-start-reading` and
`story:restart-message` cover parts of this.

## Acceptance

- `task conductor:restart` starts the new session with a prompt that names the two recurring jobs
  (at `cadence.cycle` and `cadence.daily` of `conductor config show`) and the watch command
  `conductor watch run --follow`; a test in `crates/conductor/tests/taskfile.rs` reads the task's
  prompt and finds the cadence keys and the watch command in it.
- `.agents/conductor.md`'s hand-over rule says a hand-over keeps only open work and does not list the
  jobs or the watch; a test in `crates/conductor/tests/profiles.rs` holds that sentence.

## Notes

- Shares `.agents/conductor.md` and `crates/conductor/tests/profiles.rs` with the other profile
  stories of `epic:conductor-autonomy`, and `Taskfile.yml` and `crates/conductor/tests/taskfile.rs`
  with `story:status-board`: it depends on `story:gc-before-pause` and `story:status-board` so it
  lands last on those files.

## Scope

`Taskfile.yml`, `.claude/conductor-settings.json`, `.agents/conductor.md`,
`crates/conductor/tests/taskfile.rs`, `crates/conductor/tests/profiles.rs`.
