---
format: aep.planning-md/3
id: upstream-blocker:substrate-long-sessions
kind: upstream-blocker
status: open
title: Substrate cannot host a long-lived Claude Code session
relations:
- blocks: story:substrate-session-provider
revision: 2
---
## What would clear it

Substrate lets one run reach several declared destinations (today one aperture per run), keeps a
session past one hour and across attachment loss, lets a session use AF_UNIX for its harness's own
sockets, and delivers a credentials file the harness reads (today secrets arrive only as a memfd on
an fd). Source: docs/analysis/2026-10-09-substrate-benefits.md § 3; docs/analysis/2026-10-09-substrate-sessions.md. Owner: the substrate repository (conductor dispatches it).

## Status

It blocks only the archived `story:substrate-session-provider` (Claude Code itself inside
substrate), which `story:substrate-tool-server` superseded. No live story waits on it. It stays open
as the record of why that route is parked: an upstream blocker has no retired state, and moving it to
`cleared` would claim substrate changed.
