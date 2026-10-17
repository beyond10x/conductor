---
format: aep.planning-md/3
id: story:controller-profile-for-run
kind: story
status: draft
title: 'Profiles fit the daemon: the controller reports with conductor message send, the conductor profile is retired, the maintainer profile is at most 60 lines'
relations:
- decomposes: epic:conductor-run
revision: 1
---
## Why

The controller profile (`.agents/repo-controller.md`) tells a session to report with
`SendMessage` to the conductor session, to watch nothing but also to handle disk words, usage and
hand-over messages by prompt. With `conductor run` the daemon does those, and a report is a command.
The operator (2026-10-17): a worker "would just receive the story and start working".

## Outcome

A controller profile that fits on two screens: read the charter and `AGENTS.md`, plan in the
store, deliver in waves in managed trees, build with `conductor build run`, report with
`conductor message send`, write the hand-over file on `[HANDOVER]` and stop. Nothing about timers,
disk words, usage, relays or the conductor session.

## Acceptance

1. `.agents/repo-controller.md` is at most 120 lines; every `SendMessage` mention is replaced by
   `conductor message send --to conductor --from <session> --text -`; the "Usage limits", disk-word
   and `[RESOURCE] build-slot` paragraphs are gone; `[HANDOVER <repo>]` keeps: finish the step,
   write `<checkout>/.git/handoff/<UTC stamp>.md`, report `blocked handover <path>`, stop.
2. `.agents/conductor.md` is removed; `.claude/agents/conductor.md` is removed;
   `conductor config validate` accepts an instance with no `conductor` role and `session start
   --role conductor` is refused naming `conductor run`.
3. `.agents/conductor-dev.md` is at most 60 lines: observe through `conductor status`, the
   dispatch log and `metric kpi-readings`; plan spec first; deliver in managed trees with
   `conductor build run`; hand over at 150k context to `<state>/conductor-dev/handoff/`; every
   wave names the metric it moves and records before and after.
4. `crates/conductor/tests/profiles.rs` holds the line caps, the absence of `CronCreate`,
   `Monitor`, `SendMessage`, `watch run --follow` and `[RESTART conductor]` in every shipped
   profile, and the presence of `conductor message send` in the controller profile.
5. `README.md`, `AGENTS.md` and `website/docs/` name `conductor run install` where they named
   `task conductor:start`; `task docs-check` exits 0.

## Scope

`.agents/repo-controller.md`, `.agents/conductor.md` (removed), `.agents/conductor-dev.md`,
`.claude/agents/`, `crates/conductor/src/session.rs`, `crates/conductor/src/config.rs`,
`crates/conductor/tests/profiles.rs`, `README.md`, `AGENTS.md`, `website/docs/`.
