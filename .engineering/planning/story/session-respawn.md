---
format: aep.planning-md/3
id: story:session-respawn
kind: story
status: active
title: session respawn hands a session over and restarts it under its own name
summary: seven hand-overs by hand in a day; two left a duplicate session
relations:
- depends_on: story:one-session-per-role
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:53:36Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:53:36Z", actor: "human:timo", revision: 3}
---
## Why

Seven hand-overs were done by hand on 2026-11-07: supervisor 3, maintainer 3, teacher 1. Two of them left a duplicate session: a maintainer named `<name>-2` at 13:22Z, and a second teacher at 14:39Z.

## Acceptance

- `conductor session respawn <name>` asks the live session of that name for its hand-over and waits for it (bounded, named on refusal). It waits until the session has no running sub-agent, stops it, and starts the successor under the same name with the start command of that role's profile.
- It exits 1 if `claude agents --json` then shows two live rows of that name, or a row named `<name>-<digits>`; a test over a fake agent list covers both cases.
- Each step (asked, hand-over written, sub-agents done, stopped, started) is one line on standard output, and the respawn is recorded as a decision.

## Notes

Depends on `story:one-session-per-role`, which makes `session start` refuse a second session.
