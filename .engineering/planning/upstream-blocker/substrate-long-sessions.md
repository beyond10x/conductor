---
format: aep.planning-md/3
id: upstream-blocker:substrate-long-sessions
kind: upstream-blocker
status: open
title: Substrate cannot host a long-lived Claude Code session
relations:
- blocks: story:substrate-session-provider
revision: 1
---
## What would clear it

Substrate lets one run reach several declared destinations (today one aperture per run), keeps a
session past one hour and across attachment loss, lets a session use AF_UNIX for its harness's own
sockets, and delivers a credentials file the harness reads (today secrets arrive only as a memfd on
an fd). Source: docs/analysis/2026-10-09-substrate-benefits.md § 3; docs/analysis/2026-10-09-substrate-sessions.md. Owner: the substrate repository (conductor dispatches it).
