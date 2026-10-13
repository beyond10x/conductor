---
format: aep.planning-md/3
id: story:dashboard-overview
kind: story
status: draft
title: The overview shows what needs you, repositories, today and health at a glance
relations:
- decomposes: epic:dashboard-for-engineers
- depends_on: story:dashboard-spa
- depends_on: story:onboarding-run
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/dashboard
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/collect/github.rs
- confidence: cited
  path: crates/conductor/src/collect/gitlab.rs
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/tests/collect_github.rs
- confidence: cited
  path: crates/conductor/tests/collect_gitlab.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/observation.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 6
---
## Why

Part of `epic:dashboard-for-engineers` (the operator, 2026-10-13: "make it actually useful for our
engineers"). The snapshot holds repositories, pull and merge requests, workflow runs and releases,
and `status.rs` already works out each repository's deviations for `STATUS.md`, but the dashboard
serves none of it. Its "last event" shows a dispatch's newest state, days old on a busy session.
Design reference: `docs/design/dashboard-mockup.html`, the overview.

## Acceptance

- Spec first, in `spec/domains/observation.yaml`, validated before any code: open pull and merge
  requests carry their `url` (`PullRequestObservation`, `RecordPullRequest` with `--url`, the
  `PullRequests` view; merged ones already do), recorded by `snapshot.rs` (`record_pull_request`,
  the `pull-requests` view) and filled by `collect/github.rs` and `collect/gitlab.rs`; and a view
  `RepositoryFigures` (per repository that is not archived: its deviations and the five counts the
  board compares) in the `snapshot` group of `spec/components.yaml`.
- The collectors fill `url`: in `crates/conductor/tests/collect_github.rs`, with a fake `gh` on
  `PATH` answering one open and one merged pull request, the snapshot's `pull-requests` and
  `merged-pull-requests` rows each carry the `url` the fake answered; in
  `crates/conductor/tests/collect_gitlab.rs`, the same with a fake `glab` and one open and one
  merged merge request.
- One function computes the figures: `status.rs`'s `figures()` is made shared, and both
  `publish-board` (`STATUS.md`) and `/api/board` call it; the app does not recompute deviations.
  Test: for one fixture snapshot, `/api/board`'s deviations per repository equal those of the
  `STATUS.md` `publish-board` writes.
- Read-only endpoints `/api/board`, `/api/repositories`, `/api/pull-requests` (with `url`),
  `/api/workflow-runs` and `/api/releases` answer `{snapshot: {id, started_at}, rows: [...]}` from
  the newest complete snapshot, the store opened read-only as `status.rs` does. Tests in
  `crates/conductor/tests/dashboard.rs`: each endpoint's rows over a fixture snapshot; with a newer
  snapshot still collecting or failed, the rows of the older complete one; with an empty store,
  `snapshot: null` and no rows; a POST to each is 405.
- "Last event" is the session's newest report, not its dispatch's newest state: a Rust test whose
  dispatch has a state event of days ago and a report of minutes ago reads the report in that
  session's row of `/data.json`.
- `/api/events?since=<rfc3339>` answers, read-only, `{rows: [...]}` with one row per event at or
  after `since`, newest first, each `{at, repository (or null), kind, text, link?}` where `kind` is
  one of:
  - `dispatch`: a line with a `brief` in `dispatches/*.jsonl` under the records root, read through
    `dashboard.rs`'s `jsonl()` as `collect()` reads it; `at` is the line's `at`, `repository` the
    repository whose controller session is its `to`, `text` its goal;
  - `report`: an event line (`event`, `at`) of a dispatch in the same files; `repository` is its
    dispatch's, `text` the event;
  - `decision`: a line in `decisions/*.jsonl` under the records root (the same `jsonl()`), `at` its
    `decided_at` (`raised_at` for an open request), `repository` its `repository` or null, `text`
    its class and choice or question;
  - `release`: a row of the newest complete snapshot's `releases` view, `at` its publication time,
    `link` its `url`;
  - `merge`: a row of that snapshot's `merged-pull-requests` view, `at` its merge time, `link` its
    `url`.
  "Today" is `since` = 00:00 UTC of the current day, pinned to UTC in the app and in the tests, so
  no test depends on the runner's time zone. Rust tests in `crates/conductor/tests/dashboard.rs`
  over fixture records and a fixture snapshot: one per kind, each with one event just after and one
  just before `since`, answering the first and not the second, with its `repository` and `link`.
- The overview in the app, each part with a Vitest test over fixtures whose keys a Rust test
  holds to the served endpoints:
  - summary strip: needs you, red CI on main, controllers busy/idle/silent, open pull and merge
    requests and those with a failing check, shipped today (releases published since 00:00 UTC),
    sessions over the context threshold, free disk, spend today — each figure
    from the fixture;
  - "needs you" cards: one per `awaiting_operator` row and one per repository with a red `main`
    workflow, each with the command to copy;
  - repository table: one row per repository with its state chips from `/api/board`, and filters
    all, mine, red CI. "Mine" is the repositories named by `?mine=a,b`; the list is kept in
    `localStorage` and used when the address has no `?mine=`; no identity is read;
  - timeline of today: the rows of `/api/events?since=<00:00 UTC>`, one line per event with its
    time, repository, kind and text, linked where it has a `link`; the Vitest test renders a
    fixture with one event of each kind and finds each, newest first;
  - disk and spend collapsed.

## Notes

- After `story:dashboard-spa` (the app, `dashboard.rs`, `tests/dashboard.rs`).
- After `story:onboarding-run`: the `url` field and the view regenerate `crates/conductor-model`,
  add a flag in `crates/conductor/src/cli.rs` and change `website/docs/reference/cli.md`, which the
  stories up to `story:onboarding-run` edit in turn; `spec/components.yaml` is edited by
  `story:snapshot-member-failures` and `story:snapshot-duration` before it.
- Takes over the endpoints of the retired `story:dashboard-snapshot-api` that the overview renders.
  The repository table's "waiting on" column and its "waiting" filter are
  `story:dashboard-repo-view`'s, which reads `/api/waiting-on`; that story also filters
  `/api/events` by repository.
- Shares `crates/conductor/tests/collect_github.rs` and `collect_gitlab.rs` with no other story in
  the set; `snapshot.rs` is edited by `story:snapshot-member-failures` and
  `story:snapshot-duration`, both ordered before this story.

## Scope

`spec/domains/observation.yaml`, `spec/components.yaml`, `crates/conductor-model`,
`crates/conductor/src/cli.rs`, `website/docs/reference/cli.md`, `crates/conductor/src/status.rs`,
`crates/conductor/src/snapshot.rs`, `crates/conductor/src/collect/github.rs`,
`crates/conductor/src/collect/gitlab.rs`, `crates/conductor/src/dashboard.rs`,
`crates/conductor/dashboard/`, `crates/conductor/tests/dashboard.rs`,
`crates/conductor/tests/collect_github.rs`, `crates/conductor/tests/collect_gitlab.rs`.
