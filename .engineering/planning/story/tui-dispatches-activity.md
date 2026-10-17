---
format: aep.planning-md/3
id: story:tui-dispatches-activity
kind: story
status: draft
title: The TUI shows the dispatches and the activity feed
relations:
- decomposes: epic:tui-frontend
- depends_on: story:tui-overview
revision: 1
---
# Story: tui-dispatches-activity

## Outcome

Two screens in `conductor tui`: the dispatches (open and recent, with their repository, state and age) and the activity feed (events since a point in time), from the dashboard's read model.

## Context

- Each session row carries its current dispatch, computed from `dispatches/*.jsonl` (crates/conductor/src/dashboard.rs:53-60, crates/conductor/src/dashboard/repo.rs:55-100); the dispatch states are `sent`, `started`, `blocked`, `unblocked`, `done`, `failed`, `cancelled` (docs/design/conductor.md § 10 table of records, :404).
- `/api/events?since=<rfc3339>` answers the events since then, 00:00 UTC of the day without `since` (dashboard.rs:30-35; crates/conductor/src/dashboard/overview.rs:59).
- `/api/waiting-on` and `/api/blockers` answer what a repository waits on (repo.rs:57-62).
- The activity feed is the page's timeline data; the page's lanes and bars (`story:dashboard-timeline`) are out of scope here.

## Acceptance

1. Spec first: no new noun; the `tui` command's model is unchanged (`conductor.observation.TuiRun`, `story:tui-overview`), validated with `ess specify validate --path spec`.
2. The Dispatches screen lists open dispatches first, then the day's closed ones, each with repository, state, the session that holds it and its age; selecting a row shows its history lines (sent, started, blocked with reason, done). A snapshot test over a fixture with one dispatch in each state matches stored frames.
3. The Activity screen lists events newest first from 00:00 UTC; `s` asks for a `since` (an RFC 3339 instant or `1h`, `6h`, `24h`) and the list equals `/api/events?since=` for the same instant over the same fixture.
4. New events arrive on refresh at the top without moving the selected row; the screen shows how many arrived while the operator was scrolled.
5. A repository's blockers and what it waits on show in that repository's dispatch detail, equal to `/api/waiting-on` and `/api/blockers` for it.
6. No TUI code opens `dispatches/` or `decisions/` files (the test of `story:tui-overview`, extended to these modules).

## Out of Scope

Timeline lanes, charts, token series.

## Ambiguities

- Whether the activity feed pages by count or by time window; the page uses the day.
