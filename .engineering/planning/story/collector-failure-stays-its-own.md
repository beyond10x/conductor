---
format: aep.planning-md/3
id: story:collector-failure-stays-its-own
kind: story
status: implemented
title: A collector that still fails fails only its own rows
summary: One failing collector fails the whole snapshot today; the snapshot should keep every other collector rows
refs:
- provider: github
  reference: beyond10x/conductor#241
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/http.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/collect/github.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/dashboard/health.rs
- confidence: cited
  path: crates/conductor/src/repository.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/src/store/observations.rs
- confidence: cited
  path: crates/conductor/src/store/snapshot.rs
- confidence: cited
  path: crates/conductor/tests/adv_dispatch_unknown_repository_w49.rs
- confidence: cited
  path: crates/conductor/tests/adv_snapshot.rs
- confidence: cited
  path: crates/conductor/tests/collect_github.rs
- confidence: cited
  path: crates/conductor/tests/collect_sessions.rs
- confidence: cited
  path: crates/conductor/tests/collect_stores.rs
- confidence: cited
  path: crates/conductor/tests/collector_bounds.rs
- confidence: cited
  path: crates/conductor/tests/dashboard.rs
- confidence: cited
  path: crates/conductor/tests/dashboard_overview.rs
- confidence: cited
  path: crates/conductor/tests/dashboard_repo.rs
- confidence: cited
  path: crates/conductor/tests/failed
- confidence: cited
  path: crates/conductor/tests/followup_w06.rs
- confidence: cited
  path: crates/conductor/tests/instance_sources.rs
- confidence: cited
  path: crates/conductor/tests/observation_retention.rs
- confidence: cited
  path: crates/conductor/tests/repository_marks.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: crates/conductor/tests/shipped.rs
- confidence: cited
  path: crates/conductor/tests/snapshot.rs
- confidence: cited
  path: crates/conductor/tests/status.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: spec/domains/observation.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/scenarios
- confidence: cited
  path: website/docs/guides/run-isolated.md
- confidence: cited
  path: website/docs/reference
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T12:16:16Z", actor: "human:unknown", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-20T12:16:16Z", actor: "human:unknown", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-20T14:31:47Z", actor: "human:unknown", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #241 asked that a collector that still fails after its retries fail only its own rows.
`story:forge-rate-limit-retried`, which supersedes `story:collector-retries-rate-limit`, retries
rate-limited forge reads but keeps a failing collector failing the whole snapshot; this story takes
the requirement it left out.

On main:

- The snapshot driver runs the collectors in order and, at the first that fails, records its reason
  and stops: no later collector runs (`crates/conductor/src/snapshot.rs:239-289`, the `break` at
  `:288`). The snapshot then ends `Failed` (`crates/conductor/src/snapshot.rs:302-311`), and only a
  `Complete` one is rolled up and starts retention (`:317-320`).
- A collector past its budget already does not fail the snapshot: its run is recorded `OverBudget`
  and the driver goes on with the next (`crates/conductor/src/snapshot.rs:262-271`, the spec at
  `spec/domains/observation.yaml:63-69`). A `Complete` snapshot can hold one collector's rows
  partly; this story makes `Failed` the same for the snapshot's end.
- The run of a collector keeps no reason: `CollectorRunObservation`, `RecordCollectorRun` and the
  `collector-runs` view have no such field (`spec/domains/observation.yaml:814-835`, `:1822-1845`,
  `:2688-2712`); the reason is only the snapshot's `failure_reason` (`FailSnapshot`, `:2141-2163`).
  The spec says `Failed` "fails the snapshot" (`:64-65`).
- Every reader of the observations reads only the newest `Complete` snapshot: `repository activity`
  and `metric compute` (`crates/conductor/src/repository.rs:163-171`), `status` and the board
  (`crates/conductor/src/status.rs:811-818`), the dispatch cycle (`crates/conductor/src/run/dispatch.rs:405-413`,
  review fullness from `waiting_in`), `named_repositories` (`crates/conductor/src/dispatch.rs:1275`),
  the watch (`crates/conductor/src/watch.rs:2451`, `:2492`, `:2523`), the dashboard
  (`crates/conductor/src/dashboard/repo.rs:103-107`, `dashboard/overview.rs:94`,
  `dashboard/sessions.rs:89`), `shipped` (`crates/conductor/src/shipped.rs:92`), and the github
  collector's reuse of the previous snapshot's workflow runs (`crates/conductor/src/collect/github.rs:888-917`).
  The rollup already reads a PR closure only where every forge collector ended `Completed`
  (`crates/conductor/src/rollup.rs:260-270`).
- Retention keeps the observations of the newest `instance.retention.snapshots` `Complete`
  snapshots, 1 or more (`crates/conductor/src/store/observations.rs:149-175`).
- Within the repositories collector, one repository's failed read is the collector's error and no
  further repository is started (`crates/conductor/src/collect/repositories.rs:2000-2046`, `each`).

## Outcome

- A collector that still fails fails only its own rows: the driver runs every other collector, the
  snapshot ends `Complete`, and the failing collector's run is recorded `Failed` with its reason.
- `snapshot collector-runs` shows that reason on the `Failed` run; `snapshot snapshots` and
  `status` say which collectors of the newest snapshot failed.
- A reader that would take a failed collector's missing rows for an empty answer does not: the
  dispatch cycle's review fullness and the github collector's reuse base read the newest snapshot
  whose forge collectors ended `Completed`, as the rollup's closure check already does; the other
  readers read the snapshot's other rows and show the failed collector's kinds as not read.
- Retention keeps, besides the newest `instance.retention.snapshots` `Complete` snapshots, the
  newest one whose forge collectors (`github`, `gitlab`) all ended `Completed`, so the readers
  above always have that base while it is the newest such snapshot.

## Spec first

Modelled in `spec/` before any handler: `reason: Optional<String>` on `RecordCollectorRun`,
`CollectorRunObservation` and the `CollectorRuns` view (absent on a run that did not fail and on
runs recorded before); the `CollectorRunOutcome` comment says `Failed` no longer fails the snapshot;
a scenario records a `Failed` run with its reason. Then
`ess specify validate --path spec`, `task regen` (the binding's `source_digest` re-pinned), and the
handlers. No new snapshot state: a snapshot is `Complete` or `Failed` as today, `Failed` kept for a
driver that cannot keep what a collector recorded (`snapshot.rs:252-258`) and for a gone process.

## Acceptance (tests that fail on the current code)

- `tests/snapshot.rs` `a_failing_collector_leaves_the_later_collectors_rows_recorded`: collectors
  `records("alpha")`, `fails()`, `records("beta")`; the snapshot ends `Complete`, `repositories`
  lists alpha and beta, and `collector-runs` holds `fails` `Failed` with reason
  `the source did not answer`.
- `tests/snapshot.rs` `the_views_read_the_other_collectors_rows_of_a_snapshot_one_collector_failed`:
  `repository activity` reads that snapshot, and `status` names the failed collector.
- `tests/run.rs` `a_failed_forge_collector_is_not_read_as_no_open_reviews`: the newest snapshot's
  github run `Failed`, the one before holding open reviews at `controllers.max_open_reviews`; the
  dispatch cycle still counts the reviews full.
- `tests/collect_github.rs` `the_reuse_base_skips_a_snapshot_whose_github_run_failed`.
- `tests/observation_retention.rs` `a_failed_collector_run_does_not_drop_its_last_good_rows`, with
  `retention.snapshots: 1`.
- Rewritten, as they assert the old behaviour: `tests/snapshot.rs`
  `a_source_that_does_not_answer_fails_the_snapshot_and_keeps_what_was_recorded` (`:250`),
  `a_failed_snapshot_takes_no_more_observations` (`:279`, which needs a failed snapshot made
  another way), `a_refused_observation_fails_the_snapshot_with_the_refusal` (`:423`).

## Boundaries

- The retry of a rate-limited forge read is `story:forge-rate-limit-retried`.
- One repository's failed read within a collector keeping the other repositories' rows
  (`collect/repositories.rs` `each`, the member-failure record as its pattern) is not this story; a
  follow-up story if wanted.
- No snapshot state is added, and no reader beyond those named above changes its choice of snapshot.
- `task docs-generate` rewrites `website/docs/status.md` and `website/data/status.json`; those are
  regenerated after merge, not merged by hand.
