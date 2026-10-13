---
format: aep.planning-md/3
id: story:dashboard-health
kind: story
status: implemented
title: The health view shows doctor, snapshot failures and collector times with their fixes
relations:
- decomposes: epic:dashboard-for-engineers
- depends_on: story:dashboard-repo-view
- depends_on: story:setup-command
- depends_on: story:snapshot-member-failures
- depends_on: story:snapshot-duration
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/dashboard
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
- confidence: cited
  path: spec/domains/observation.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T13:13:57Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "proposed", to: "active", at: "2026-10-13T13:14:09Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "active", to: "implemented", at: "2026-10-13T13:19:39Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":6}}}
---
## Why

Part of `epic:dashboard-for-engineers` (the operator, 2026-10-13). When a snapshot fails or is
slow, an engineer today runs `conductor doctor`, reads the snapshot views and guesses the fix. The
records exist once `story:snapshot-member-failures` and `story:snapshot-duration` are in; nothing
shows them together with what to do. Design reference: `docs/design/dashboard-mockup.html`, the
health view.

## Acceptance

- Spec first, in `spec/domains/observation.yaml`, validated before any code: an enum
  `FailureClass` — `needs login` (a prompt or a refused credential), `unreachable host`,
  `timed out`, `old store` (a first line holding `[unsupported_protocol_version]`, the code `aep`
  gives when it refuses an `aep.project/1` store), `other`.
- `/api/health` answers, read-only:
  - `doctor`: the rows and verdict of `conductor doctor` (computed through a function
    `doctor.rs` shares with the command), from a cache the server fills at start and refreshes
    every 10 minutes off the request path, served with the time it was taken as `taken_at`; the
    interval and the clock are passed in through `Sources`, so a test can shorten them;
  - `snapshot`: the newest snapshot in any state, with its state and `failure_reason`;
  - `collector_runs`: that snapshot's `snapshot collector-runs` rows;
  - `member_failures`: that snapshot's `snapshot member-failures` rows, each with its class and a
    fix text from one table in `dashboard.rs` keyed by `FailureClass`;
  - `disk`: free space, the floors `thresholds.disk_low` and `thresholds.disk_admit` from the
    instance's config, a `state` — `ok` above `disk_low`, `low` at or below it, `admit-only` at or
    below `disk_admit` — and every disk-waste row.
- Rust tests in `crates/conductor/tests/dashboard.rs`:
  - doctor is cached: with a fake `aep` on `PATH` that counts its runs, two requests after start
    answer the same `doctor` and `taken_at` and the count stays at one;
  - doctor is refreshed off the request path: with a fixture clock and a short interval, a request
    after the interval answers at once with the cached `doctor` and its old `taken_at` (the fake
    `aep` sleeps, and the answer does not wait for it), the count then reaches two, and the next
    request answers the new result with a later `taken_at`;
  - the newest snapshot is served when it failed, with its `failure_reason`, and when it is still
    collecting;
  - `collector_runs` and `member_failures` hold the fixture snapshot's rows;
  - one test per class, each a member failure whose first line is of that class
    (`could not read Username` / `Permission denied (publickey)`, `Could not resolve hostname`,
    `gave no answer within 120 s`, a first line holding `[unsupported_protocol_version]` — what
    `aep` prints for an `aep.project/1` store, "error: project document (…) is not valid:
    [unsupported_protocol_version] project.version: this store is aep.project/1 …" — and
    anything else) answering its class and that class's fix text;
  - free disk against the floors: with `thresholds.disk_low` and `thresholds.disk_admit` set in a
    fixture config and a fixture free-space reading above `disk_low`, at `disk_low`, and at
    `disk_admit`, `disk.state` answers `ok`, `low` and `admit-only`;
  - disk waste in full: a fixture of 85 waste rows answers 85 rows.
- The health view in the app renders each part, with a Vitest test per part over fixtures whose
  keys a Rust test holds to `/api/health`.

## Notes

- After `story:dashboard-repo-view` (the app, `dashboard.rs`, `tests/dashboard.rs`).
- Reads what `story:setup-command` (doctor rows), `story:snapshot-member-failures` and
  `story:snapshot-duration` create; it depends on each directly. All three come before
  `story:onboarding-run`, which `story:dashboard-overview` waits for, and that one edits
  `doctor.rs` last before this.
- `story:sandbox-spec` depends on this story.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/doctor.rs`,
`crates/conductor/src/dashboard.rs`, `crates/conductor/dashboard/`,
`crates/conductor/tests/dashboard.rs`.
