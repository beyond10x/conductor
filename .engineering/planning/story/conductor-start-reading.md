---
format: aep.planning-md/3
id: story:conductor-start-reading
kind: story
status: draft
title: Conductor restarts less often and starts on less reading
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/conductor-dev.md
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: docs/analysis/conductor-start-reading.md
- confidence: cited
  path: docs/design/conductor.md
revision: 2
---
## Why

Conductor restarted eight times in eleven hours on 2026-10-13: three at its context threshold, five
asked for by conductor-dev in two bursts (12-20 minutes apart), two of them only for a rules.md
item, while another rules.md change reached the controllers as a relayed diff with no restart. Each
start read about 1,500 lines (91k-113k tokens of context) before arming anything: the whole
rules.md, design §§ 2-12, the north star and status, and up to three earlier hand-overs; about
650k tokens went to orientation. One restart killed a running snapshot, another left a cycle
unfinished. While conductor-dev was unreachable, conductor could not be restarted and ran 1 h 40 min
past its threshold.

## Acceptance

- Design § 12 says a rules.md change reaches a running conductor as a relayed diff, like the
  controllers', and a restart is only for the profile, the adapter, the controller profile or the
  design; `.agents/conductor-dev.md` says restarts are batched with the next wave, at most one an
  hour unless one corrects a live fault. A test in `crates/conductor/tests/profiles.rs` holds both
  sentences.
- `.agents/conductor.md`'s session-start list reads design §§ 4-8 only, and the rules for its role
  through `conductor rules show --section conductor`, not the whole file; the same test holds it.
- `.agents/conductor.md` says: when `[RESTART conductor]` cannot be answered because conductor-dev
  is unreachable, conductor writes its hand-over and runs `task conductor:restart` itself; the same
  test holds it, and `crates/conductor/tests/taskfile.rs` shows the task allowed from conductor's
  records directory.
- A recorded restart on the instance reads under 40k tokens of context before the first job is
  armed, measured as `docs/analysis/conductor-start-reading.md` records it.

## Notes

- After `story:rules-show` (the command this story's profile names).
- Shares `.agents/conductor.md` and `crates/conductor/tests/profiles.rs` with
  `story:records-from-commands`, `story:watch-pr-checks`, `story:watch-waits`,
  `story:handover-draft` and `story:news-pending`; one at a time.
- The conductor-side twin of `story:controller-start-reading`.

## Scope

`.agents/conductor.md`, `.agents/conductor-dev.md`, `docs/design/conductor.md`, `Taskfile.yml`,
`crates/conductor/tests/profiles.rs`, `crates/conductor/tests/taskfile.rs`,
`docs/analysis/conductor-start-reading.md` (new).
