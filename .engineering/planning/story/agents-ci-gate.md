---
format: aep.planning-md/3
id: story:agents-ci-gate
kind: story
status: implemented
title: AGENTS.md names CI's task check as the merge and release gate
relations:
- decomposes: epic:disk-and-gate
scope:
- confidence: cited
  path: AGENTS.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T19:01:41Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-12T19:01:41Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-12T19:14:37Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`story:ci-full-gate` left a follow-up (its Notes): `AGENTS.md` still sends every merge and release
through a local `task check` on a host short of disk, although CI runs it on every pull request.

## Acceptance

- `AGENTS.md` says a pull request merges when its `task check` job is green and that release step 2
  is that green job on the release commit, a local `task check` being needed only where CI cannot
  run; no other line of it asks for a local gate.

## Scope

`AGENTS.md`.
