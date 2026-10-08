---
format: aep.planning-md/3
id: story:substrate-session-spike
kind: story
status: active
title: 'Spike: run a controller session inside substrate'
relations:
- decomposes: epic:multi-instance-host
scope:
- confidence: cited
  path: docs/analysis
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T22:02:56Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-08T22:02:56Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
---
## Why

Substrate confines a process in a cgroup with cpu, memory and pids limits and reports exact usage
(`spec:multi-instance-host`, findings 7, 8). Whether a `claude` background session, with its OAuth
credentials, network, PTY and child processes, runs inside a substrate session is not known.

## Acceptance

- A written result in `docs/analysis/`: a scratch `substrate-daemon` with a delegated cgroup root
  under the user service, a `claude -p` and a `claude --bg` session started through it in a scratch
  checkout, what worked and what was refused, with the commands and outputs.
- The limits and metrics the run observed (`GET /v1/metrics`) for one session.
- A go/no-go line for running controllers under substrate, and the stories it would take.

## Scope

`docs/analysis/` (new page); no product code.
