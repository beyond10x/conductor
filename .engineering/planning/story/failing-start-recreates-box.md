---
format: aep.planning-md/3
id: story:failing-start-recreates-box
kind: story
status: implemented
title: A box substrate answers as gone is forgotten and a new one opened; a failed recreation alerts
summary: No silent retry of a stored box substrate no longer holds
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
- depends_on: story:loop-alerts-when-it-cannot-act
scope:
- confidence: cited
  path: crates/conductor/src/alert.rs
- confidence: cited
  path: crates/conductor/src/daemon/reconcile.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: cited
  path: crates/conductor/src/records.rs
- confidence: cited
  path: crates/conductor/tests/placement.rs
- confidence: cited
  path: spec/domains/dispatch.yaml
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T19:24:59Z", actor: "human:unknown", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-19T19:24:59Z", actor: "human:unknown", revision: 9}
- {from: "active", to: "implemented", at: "2026-11-20T02:20:14Z", actor: "human:unknown", revision: 12, decided_on: {"recorded":{"test_result":1}}}
---
## Why

An instance whose stored box names a workspace that no longer exists failed its start with
`resource.not-found` every minute for 19 hours with no alert (#207). A boxed start first closes the
box the daemon holds for the instance: `start_runtime` -> `close_strays`
(`crates/conductor/src/daemon/reconcile.rs:1460-1468`) -> `stop_runtime` -> `close_box`
(`:1617-1640`). When substrate answers that the workspace does not exist, the close fails, the
entry stays in `boxes.json`, nothing is opened, and every pass fails the same way. The run-bound
restart (`keep_boxes` -> `restart_in_box`, `:723-835`) reuses the same stored workspace and keeps
its old `started_at` on a refusal, so it is asked again every tick.

## Outcome

A stored box substrate answers as gone is forgotten and a new box is opened in the same pass;
should the new box's start fail as well, the operator gets one alert instead of a silent retry.

## Acceptance

- Substrate's not-found answer is one named constant in `crates/conductor/src/placement.rs`
  (`resource.not-found`). A close of the stored box, or a run-bound restart in it, that substrate
  answers with it removes the box from `boxes.json`, frees its slot, and the pass (for a restart,
  the next pass) opens a new
  box through `start_runtime`; the daemon log has one line naming the forgotten workspace, and the
  pass records one `Start` (`Succeeded` when the new box opens).
- A new box whose start then fails follows the existing row of failed starts (`failed_start`,
  `retry_after`; the instance reads `Unreachable` on `/v1/instances` with the refusal as detail),
  and the first failure of a recreated box raises one alert through `crate::alert`, condition
  `box-not-recreated`, subject the instance; a second failure of the same row raises none.
- Tests in `crates/conductor/tests/placement.rs` with its fake substrate (`Fake`, `:136`; its
  `close` at `:192`, `keep_boxes` driven at `:2225`), none binding a socket, each failing on main:
  a close answered not-found leads to a new box in the same pass and no `Refused` record; a
  restart answered not-found leads to a new box at the next pass; a recreated box whose open is refused raises one alert
  over three passes.

## Boundaries

The daemon never removes a workspace's data and closes no box substrate still holds; it only
forgets its reference to one substrate answered as gone. Any other close or restart failure keeps
today's behaviour.

## Scope

`crates/conductor/src/daemon/reconcile.rs`, `crates/conductor/src/placement.rs`,
`crates/conductor/tests/placement.rs`; and, found while implementing: `crates/conductor/src/alert.rs`
(the condition's constant), `crates/conductor/src/records.rs` (a dispatch-log append that names
its records, for the daemon of several instances), the alert comments of `spec/domains/dispatch.yaml`
and the alerts paragraph of `docs/design/conductor-run.md`.

## Notes

Depends on `story:loop-alerts-when-it-cannot-act` for `crate::alert`. Shares no file with
`story:unit-merge-policy` or `story:need-starts-unit-of-work`.
