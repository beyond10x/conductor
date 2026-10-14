---
format: aep.planning-md/3
id: story:dashboard-timeline
kind: story
status: draft
title: The event timeline shows lanes, dispatch bars and kinds at a glance
relations:
- decomposes: epic:dashboard-charts
revision: 1
---
## Why

The operator, 2026-10-14: the dashboard's event timeline is "just some green dots on a string ...
we have very different types of events ... visualize this in a more meaningful, more visually
appealing way". That day `/api/events` held 468 rows from 15 repositories: 257 were
`message-received`, decisions did not appear at all, and each row's text was one verb (`sent`,
`blocked`, `done`) without the dispatch, pull request or goal it belongs to.

## Acceptance

- `/api/events` carries, per row, the dispatch id, goal, pull request or release it belongs to, the
  decision class for decisions (which it now includes), and a severity (`info`, `progress`,
  `attention`, `done`); `message-received` rows are folded into a per-repository count per 10
  minutes instead of one row each. Tests in `crates/conductor/tests/dashboard_*.rs` hold the
  fields, the decisions and the fold.
- The overview's timeline is a horizontal time axis (today, zoomable to the last hour) with one
  lane per repository plus one for conductor: a dispatch is drawn as a bar from `sent` to `done`
  (striped while `blocked`), merges and releases as distinct marks, decisions as diamonds coloured
  by class (C, O, H), message traffic as a faint density band, and a "now" line; hovering a mark
  shows its details and link, a kind filter hides kinds, and colours and marks are readable in light
  and dark themes and without colour (shape differs per kind). Vitest and a page snapshot hold the
  lanes, bars, marks and filter; the 390 px width falls back to the current list.
- With `story:dashboard-stream`, new events slide in without a reload.

## Scope

`crates/conductor/src/dashboard/overview.rs`, `crates/conductor/tests/dashboard_overview.rs`,
`crates/conductor/dashboard/src/views/overview/TodayTimeline.vue`,
`crates/conductor/dashboard/src/charts/`, `crates/conductor/dashboard/src/api/overview.ts`,
`crates/conductor/dashboard/dist/`.
