---
format: aep.planning-md/3
id: epic:deterministic-steps
kind: epic
status: draft
title: Conductor and controller steps that a program can do run with no model call
revision: 1
---
## Why

The organisation's long-running goal is deterministic agentic engineering: work runs through the loom
SDK and AEP so that delivery, fixes and incident work are deterministic, with the fewest model calls
possible. Conductor and its controllers spend model calls today on steps a program can do: keeping
records, parsing reports, polling CI and status, verifying releases, resuming sessions after a limit,
running the cycle's bookkeeping. The retrospective (`docs/design/self-improvement.md`) measured 758
sleep calls in CI polls, 37 hand-overs and 24 restarts in a week, and conductor appending ~310 records
lines by hand on one day before the commands wrote them.

## Outcome

Every conductor and controller step whose result is fully determined by its inputs runs as a
`conductor` command or the watch, with no model call; sessions call the model only to judge, plan,
write code and talk to a person. Measured by T4 LLM calls per task (`story:kpi-per-task`), per step
kind, with a target that goes down each release.

## Steps that become deterministic

| step | today | becomes | story |
|---|---|---|---|
| records bookkeeping (dispatch, report, decision lines) | hand-written by conductor until v0.9.0 | the commands write them (done) | story:records-from-commands (implemented) |
| report parsing and the dispatch state it implies | conductor reads and records each report | `receive-message` moves the dispatch (started, blocked, done) from a parsed report | story:report-to-state |
| CI waits | sessions poll `gh run` with sleeps | a registered wait the watch fires | story:watch-pr-checks, story:watch-waits |
| status reads by other sessions | full status read every few minutes | `conductor status --follow` feed | story:status-command |
| the cycle's bookkeeping (snapshot, board, reclaim, records check, KPIs) | conductor runs each step by prompt | `conductor cycle`, run by the scheduler; conductor reads its summary | story:cycle-command |
| resume after a usage limit | conductor lists and respawns by prompt | the watch respawns limit-stopped controllers (rules item 64) | story:auto-resume |
| release verification | a model reads the release page | `conductor release verify <repo> <tag>`: archives, sums, notes, tag author | story:release-verify |
| hand-over open work | written by the model from memory | `conductor handover draft` renders it from the store | story:handover-draft |
| restart and ready report | messages composed by models | modelled messages, sent by the task | story:restart-message |
| disk slots | asked and granted by message | `conductor build run` (done in v0.10) | epic:disk-protocol |

## Not covered

Judgement steps (planning, review, code, escalations) keep their model calls; the loom SDK side of
deterministic execution is loom's epic.
