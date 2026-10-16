---
format: aep.planning-md/3
id: story:review-p2-0-14
kind: story
status: draft
title: The eight P2 defects of the 0.14.0 external review are fixed
revision: 1
---
# Story: review-p2-0-14

## Outcome

The eight P2 defects of the operator-requested external review of 0.14.0 (0e7ce756) are fixed. Each has the review's probe as a passing test.

## Context

The review reproduced every defect with a failing probe, against a baseline of 1,051 Rust tests and 152 Vitest tests passing. The three P1s are separate stories: story:reclaim-alias-protected, story:controller-start-atomic and story:build-run-cancel-group.

## Acceptance

1. dispatch.rs:312: `report-done` appends its audit line before it commits Done, or a retry after a failed append completes and does not fail with DispatchStateConflict.
2. goal.rs:138: `mark-goal-met` records the evidence it was given.
3. rollup.rs:280: an OverBudget (partial) snapshot records `prs_closed_unmerged` as unknown, not as a count.
4. collect/blockers.rs:584: the repositories `a/api` and `b/api` keep separate identities (full name, not basename).
5. collect/mod.rs:532: reading a collector's output pipes respects the collector deadline.
6. watch.rs:269: the CI watch routes each GitHub repository to its own owner.
7. trust.rs:291: trust covers nested GitLab checkouts.
8. dashboard/health.rs:188: the dashboard's doctor run passes the explicit `--config`.

Each item has its review probe in the suite, failing before the fix and passing after.

## Out of Scope

Refactors beyond each fix.

## Ambiguities

None.
