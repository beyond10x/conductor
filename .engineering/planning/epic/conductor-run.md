---
format: aep.planning-md/3
id: epic:conductor-run
kind: epic
status: draft
title: 'The loop is a program: conductor run senses, relays, hands over, dispatches and records with no model call; a model answers only a class-C decision request'
relations:
- decomposes: initiative:self-improvement
revision: 1
---
## Why

Measured on 2026-10-17 (`docs/analysis/takeover-baseline-2026-10-17.md`): the conductor session
sent 268 messages in ten hours, 236 of them relays; it ran at a median context of 432k against a
200k threshold and never restarted; the maintainer session spent 29 % of the instance's tokens on 15
releases of conductor itself; the rules were 98 KB of prose; the self-improvement loop produced
zero readings. The organization's thesis says what is required and whether completion was earned
are decided deterministically, and a model is asked only among named actions
(`docs/design/conductor-run.md` § 1).

## Outcome

The loop is a program, `conductor run`: it senses, watches, relays, hands over, resumes, dispatches
and records with no model call, calls a model once per class-C decision request through a bounded
adapter, and sends class O and H to the Decisions page. No session of the instance arms a timer,
watches anything or relays a line. Acceptance is `docs/design/conductor-run.md` § 5.

## Stories

`story:mailbox-send-and-deliver`, `story:run-daemon` (adopting `story:cycle-command`,
`story:report-to-state`, `story:auto-resume`), `story:decide-model-adapter`,
`story:session-home-spike`, `story:controller-profile-for-run`, `story:maintainer-guard`,
`story:charter-generated`.

## Not covered

The bwrap write fence (`epic:session-confinement`), the TUI (`epic:dashboard-for-engineers`), rule
text of any kind.
