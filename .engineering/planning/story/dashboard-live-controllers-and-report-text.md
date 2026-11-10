---
format: aep.planning-md/3
id: story:dashboard-live-controllers-and-report-text
kind: story
status: implemented
title: The dashboard counts live controllers and its events carry the report text
scope:
- confidence: inferred
  path: crates/conductor/dashboard/src/views/__tests__/__snapshots__/pages.spec.ts.snap
- confidence: inferred
  path: crates/conductor/dashboard/src/views/__tests__/fixtures/events.json
- confidence: inferred
  path: crates/conductor/dashboard/src/views/__tests__/fixtures/overview-events.json
- confidence: inferred
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/src/dashboard/overview.rs
- confidence: inferred
  path: crates/conductor/src/dashboard/repo.rs
- confidence: cited
  path: crates/conductor/src/dashboard/sessions.rs
- confidence: inferred
  path: crates/conductor/tests/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/dashboard_overview.rs
- confidence: cited
  path: crates/conductor/tests/dashboard_sessions.rs
revision: 22
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:05Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:05Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-10T07:51:28Z", actor: "human:timo", revision: 22, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from conductor 1.0.1, a trial instance. The
dashboard showed 0 controllers while three spawned leads worked: after SessionNotReachable the
loop records the controller Stopped (run/handlers.rs:872-890, see
story:deliver-reaches-bg-session), and dashboard/sessions.rs:64-70 lists only Running and Paused;
`/v1/instances/example-org/data.json` returned `"controllers":0`. The report rows in `/events` carry only
the verb ("started", "message-received"), not the progress text. Detail: the reporter's defect list item 10.

## Acceptance

- A report row in `/events` carries the report's text as well as its verb; a test holds it.
- The dashboard counts a controller whose session `claude agents` shows live, whatever the store
  recorded for it; a test with a fake `claude` holds it.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor` dashboard read side (`src/dashboard/`) — cited
- **Files:** `crates/conductor/src/dashboard/sessions.rs` (`roster()` 65-72 keeps only `Running | Paused`; `shown()` 188-194 counts live rows), `crates/conductor/src/dashboard/overview.rs` (`events()` 398-411 pushes the verb as a report row's `text`; module table 18-28 documents it), `crates/conductor/tests/dashboard_sessions.rs` (controller count 384), `crates/conductor/tests/dashboard_overview.rs` (796-803 pins `"text": "started"`) — cited
- **Symbols:** `sessions::roster`, `sessions::shown`, `Shown::controllers`, `overview::events`, `ControllerState::Stopped`, `DispatchLogLine::{first_line, reason, evidence}` — cited
- **Also likely:** `crates/conductor/tests/dashboard.rs` (asserts `data["controllers"]` at 352, 766, 814, 1012), `crates/conductor/src/dashboard/repo.rs` (298-302 keeps `Running` controllers only, the same blind spot on the repository page), `crates/conductor/src/dashboard.rs` (`collect()` 1565-1588 doc and `"controllers"` key), `crates/conductor/dashboard/src/views/__tests__/fixtures/overview-events.json`, `crates/conductor/dashboard/src/views/__tests__/fixtures/events.json`, `crates/conductor/dashboard/src/views/__tests__/__snapshots__/pages.spec.ts.snap` — inferred, touched only if the count's doc, the repository page or the app fixtures follow the change
- **Documents:** none found; `website/docs/guides/dashboard.md` mentions `/v1/events` but no report text — inferred
- **Confidence:** high — the story cites `dashboard/sessions.rs:64-70` and the tree shows the one place report rows get their text
- **Would collide with:** any unit touching `crates/conductor/src/dashboard/sessions.rs` or `overview.rs`, or the `dashboard_sessions.rs` / `dashboard_overview.rs` tests; no overlap with `story:deliver-reaches-bg-session`'s `run/handlers.rs`, `session.rs` or `spawn.rs` — inferred
- **Safety fact:** `shown()` counts a controller only when its session-list row is live (`sessions.rs:192`), so widening `roster()` beyond `Running | Paused` cannot raise the count for a dead session; it does add a `not running` row for every `Stopped` record whose session is gone unless those are dropped (`sessions.rs:188-203`, `session()` 217-223) — step 2, unproven — inferred
