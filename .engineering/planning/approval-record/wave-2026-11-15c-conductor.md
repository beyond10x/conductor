---
format: aep.planning-md/3
id: approval-record:wave-2026-11-15c-conductor
kind: approval-record
status: approved
title: 'Conductor wave 2026-11-15c: a boxed runtime''s sessions run without claude --bg'
relations:
- decides: story:box-session-without-bg
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-15T12:13:21Z", actor: "human:agent", revision: 2, executor: "human:timo"}
---
# Conductor wave 2026-11-15c: a boxed runtime's sessions run without `claude --bg`

## Approval

The operator approved this wave on 2026-11-15 and asked to get it running without being asked
per step: make the controllers and the maintainer a runtime starts inside a substrate box run,
without changing substrate's sandbox rules, and deliver it on an integration branch with a
planning item for every change, without a release.

## Units

| unit | artifact | note |
|---|---|---|
| U1 | `story:box-session-without-bg` | a `child` provider: `claude -p` with stream-json input as a child process, its messages through a FIFO, listed beside `claude agents --json`, stopped by a signal, resumed with `claude -p --resume`; chosen where a Unix socket is refused |

## Terms

- **Gate:** the pull request's checks; nothing merges red and no check is overridden.
- **Release:** none in this wave; the release decision is separate.
- **Evidence:** `tests/box_session.rs`; the local run recorded on the story.
