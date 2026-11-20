---
format: aep.planning-md/3
id: review-result:w68-critic-panel
kind: review-result
status: active
title: Critic panel, wave 68 (collector-failure-stays-its-own)
summary: facts PASS-WITH-CHANGES (wrong each cite, understated readers); testability PASS-WITH-CHANGES (fixtures, old tests, risk tests); spec-first FAIL->PASS (collector-run reason field modelled first); risk FAIL->PASS (dispatch review fullness, github reuse base, retention); overall PASS-WITH-CHANGES, all taken in
relations:
- reviews: story:collector-failure-stays-its-own
revision: 1
---
PASS-WITH-CHANGES (all required edits taken into revision 3-4 of the story)

Four lenses over `story:collector-failure-stays-its-own` revision 2, checked against the tree at
the wave branch's base.

## Facts: PASS-WITH-CHANGES

- `collect/repositories.rs:1904-1946` is the forge retry loop of one command, not `each`; `each`
  is `:2000-2046`. Fixed.
- `snapshot.rs:239-290` / `break` at `:288` / `Failed` at `:302-311`: hold (loop ends `:289`).
- "`repository activity` and `metric compute` read only the newest `Complete` snapshot" holds but
  understates it: status, board, dispatch cycle, watch, dashboard, shipped and the github reuse
  base read only `Complete` too. Listed with file:line.
- Missed precedent: `OverBudget` already leaves a snapshot `Complete` with one collector partial
  (`snapshot.rs:262-271`); the rollup already guards on forge runs `Completed` (`rollup.rs:260-270`).
  Added.

## Testability: PASS-WITH-CHANGES

- The two named tests had no fixture; now stated (`records`, `fails`, `records`).
- Three existing tests in `tests/snapshot.rs` (`:250`, `:279`, `:423`) assert the old behaviour and
  would fail; now named as rewritten.
- "Say which collector failed" and the risk outcomes had no test; three tests added (run,
  collect_github, observation_retention).
- The repositories-collector per-repository question was "decided in design", an outcome without a
  test; moved out of scope.

## Scope and spec-first: FAIL on revision 2, PASS after edits

- A per-collector failure reason is not modelled: `CollectorRunObservation`, `RecordCollectorRun`
  and `CollectorRuns` have no reason field, and the spec comment says `Failed` fails the snapshot.
  A `## Spec first` section now states the field, the comment, a scenario, `task regen` and the
  re-pinned binding; no new snapshot state.
- Scope lacked `spec/`, `http.yaml`, `crates/conductor-model` and the readers; added.

## Risk: FAIL on revision 2, PASS after edits

- A `Complete` snapshot with a failed forge collector reads as zero open reviews in the dispatch
  cycle's review fullness: over-dispatch. Outcome now requires the forge-complete base.
- The github collector reuses the previous `Complete` snapshot's workflow runs; a failed github run
  would make an empty base. Outcome and test added.
- Retention counts only `Complete` snapshots; with `retention.snapshots: 1` a partial snapshot would
  drop the last good forge rows. Outcome: retention also keeps the newest forge-complete snapshot.
- Partial snapshots hiding stale rows: other readers show the failed collector's kinds as not read
  rather than empty.
