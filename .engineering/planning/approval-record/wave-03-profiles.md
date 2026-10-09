---
format: aep.planning-md/3
id: approval-record:wave-03-profiles
kind: approval-record
status: draft
title: The operator approved wave 03's profile diff
relations:
- decides: story:instance-session-names
- decides: story:controller-report-sendmessage
revision: 1
---
## The question

Wave 03 changes two profiles, `.agents/conductor.md` and `.agents/repo-controller.md`; a profile
change merges only on the operator's word. Shown to him: the 57-line diff
(`git diff 4a3701257 318a73fd2 -- .agents/`), summarised as: controllers start under
`<prefix>-<repo>` names; controllers send to `conductor.session_name`; every report is a SendMessage
call; without a prefix every name stays as today.

## The decision

The operator, 2026-10-09, to conductor-dev: "ok".

## What follows

`wave/03` merges into `main`; the binary is installed with the config migration
(`session_prefix: work` for the second instance); conductor restarts because its profile changed.
