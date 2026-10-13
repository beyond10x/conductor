---
format: aep.planning-md/3
id: story:dashboard-repo-view
kind: story
status: draft
title: One repository's page shows what it waits on, its MRs, plan and today
relations:
- decomposes: epic:dashboard-for-engineers
- depends_on: story:dashboard-overview
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/dashboard
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/src/usage.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/decision.rs
- confidence: cited
  path: spec/domains/decision.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 6
---
## Why

Part of `epic:dashboard-for-engineers` (the operator, 2026-10-13). An engineer looks after one
repository and wants to see, on one page, what its controller works on and waits on, its merge
requests, its plan and what happened to it today. A `Decision` names no repository
(`spec/domains/decision.yaml`; a `DecisionRequest` does), so decisions cannot be shown per
repository. Design reference: `docs/design/dashboard-mockup.html`, the repository page.

## Acceptance

- Spec first, validated before any code: `Decision` gains an optional `repository`
  (`spec/domains/decision.yaml`), set by `--repository` on `record-conductor-decision` and
  `record-operator-decision` (`crates/conductor/src/cli.rs`, `crates/conductor/src/decide.rs`,
  regenerated `website/docs/reference/cli.md`); escalations already carry it through their
  `DecisionRequest`. A test records a decision with `--repository` and reads it back in the
  decisions view.
- Read-only endpoints `/api/blockers` and `/api/specifications` answer, like the overview's,
  `{snapshot, rows}` from the newest complete snapshot; tests in
  `crates/conductor/tests/dashboard.rs` read their rows over a fixture snapshot.
- `/api/waiting-on` answers, per repository, the causes that apply, each by a rule over data the
  server holds; a repository's session is the session whose working directory is its checkout. One
  Rust test per cause, with a fixture that meets the rule and one that just misses it:
  - **operator**: an open decision request for the repository of class O or H;
  - **CI**: an open pull or merge request whose newest workflow run is pending or failing, and no
    session of the repository is working;
  - **build slot**: a resource request of the repository's controller in state `Requested`;
  - **another repository**: a `[NEED]` from the repository's controller routed to a dispatch of
    another repository and not closed, naming that repository;
  - **usage limit**: the watch's usage-limit line names the repository's session;
  - **silent**: the watch's `session silent:` line names the repository's session.
- `/api/sessions?repository=<name>` answers, read-only, `{rows: [...]}`, one row per session whose
  working directory is the repository's checkout or one of its managed trees: `{name, repository,
  state, started_at, dispatch: {id, brief} or null, transcript, spend_today}`. `state` and
  `started_at` are those `/data.json` gives the session; `dispatch` is its newest dispatch
  (`dashboard.rs`'s `dispatch()`); `transcript` is the path of its transcript in tilde form,
  `~/.claude/projects/<dir>/<session>.jsonl` (`<dir>` the working directory encoded as
  `usage.rs` decodes it); `spend_today` is the repository's token spend since 00:00 UTC from
  `usage.rs`. Rust tests in `crates/conductor/tests/dashboard.rs`: over a fixture session list,
  records and transcripts, the repository's two sessions answer with these fields and another
  repository's session is absent; the transcript path starts with `~/`; spend counts a transcript
  line of today and not one of yesterday (UTC).
- Today's events for a repository are `/api/events?since=<00:00 UTC>&repository=<name>`, the
  overview's endpoint (`story:dashboard-overview`) with a `repository` filter added here. Rust test:
  over the overview's fixture records and snapshot, the filter answers only that repository's
  dispatch, report, decision (by `Decision.repository`), release and merge rows.
- The overview's repository table gains the filter "waiting" (beside all, mine, red CI): the
  repositories `/api/waiting-on` answers at least one cause for. Vitest test: over a fixture where
  one of three repositories has a cause, "waiting" shows that one only.
- The repository page at `#/repo/<name>`, each part with a Vitest test over fixtures whose keys a
  Rust test holds to the served endpoints:
  - waiting on, from `/api/waiting-on` (also the overview table's "waiting on" column);
  - working on: the newest dispatch to the repository's controller (goal, brief) and its newest
    report;
  - main CI (the newest `main` workflow run) and release (the newest release and the commits since
    it);
  - merge and pull requests, each with its pipeline status and a link to its `url`;
  - the plan: open blockers by kind from `/api/blockers`, and the specification's presence and
    validation from `/api/specifications`;
  - sessions, from `/api/sessions?repository=`: name, state, start, dispatch, today's spend and
    the transcript's path to copy;
  - today's events, from `/api/events?since=<00:00 UTC>&repository=`: its dispatches, reports,
    decisions, releases and merges, one per line with its time and kind.

## Notes

- After `story:dashboard-overview` (the app, `dashboard.rs`, `tests/dashboard.rs`, the endpoint
  shape and its `url` field; the shared `cli.rs` and `reference/cli.md` are ordered through it
  after `story:onboarding-run`).
- A session's context use and its box are not shown: no source serves them per session until the
  sandbox stories (the box) and a recorded context reading exist; they are left to those stories.
- Takes over `/api/blockers` and `/api/specifications` from the retired
  `story:dashboard-snapshot-api`.

## Scope

`spec/domains/decision.yaml`, `crates/conductor-model`, `crates/conductor/src/cli.rs`,
`crates/conductor/src/decide.rs`, `website/docs/reference/cli.md`,
`crates/conductor/src/dashboard.rs`, `crates/conductor/src/usage.rs`, `crates/conductor/dashboard/`,
`crates/conductor/tests/dashboard.rs`, `crates/conductor/tests/decision.rs`.
