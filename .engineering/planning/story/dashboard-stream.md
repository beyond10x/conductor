---
format: aep.planning-md/3
id: story:dashboard-stream
kind: story
status: draft
title: The dashboard streams changes and counts tokens up live
relations:
- decomposes: epic:dashboard-charts
revision: 1
---
## Why

The operator, 2026-10-14: "we should do some realtime SSE stuff, so we have the actual feel of a
realtime application ... see total tokens counting up with a nice animated counter". The app polls
every 20 s; nothing streams.

## Acceptance

- `GET /api/stream` is a `text/event-stream`: an event per store write (snapshot, dispatch, report,
  decision, release, merge) naming what changed, and a `tokens` event every few seconds with the
  instance's running input and output token totals for the day (from session transcripts, counts
  only); a test reads the events over a socket.
- The app subscribes with `EventSource`, refetches only the changed panel, stops the 20 s polls
  while connected (falls back when not), and shows the day's total tokens as an animated counter
  that counts up between events; Vitest holds the subscription, fallback and counter.

## Scope

`crates/conductor/src/dashboard.rs`, `crates/conductor/src/dashboard/stream.rs` (new),
`crates/conductor/dashboard/src/`, `crates/conductor/tests/dashboard_stream.rs` (new).
