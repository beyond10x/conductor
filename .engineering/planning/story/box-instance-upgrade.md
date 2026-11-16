---
format: aep.planning-md/3
id: story:box-instance-upgrade
kind: story
status: draft
title: conductor instance upgrade upgrades a runtime that runs in a substrate box
summary: The upgrade's running check asks substrate for a boxed runtime, as the reconciler does, instead of the service manager
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:instance-upgrade
scope:
- confidence: cited
  path: crates/conductor/src/daemon/upgrade.rs
- confidence: cited
  path: crates/conductor/src/daemon/reconcile.rs
- confidence: inferred
  path: crates/conductor/tests/placement.rs
revision: 1
---
## Why

Agentbox friction log F7 (2026-11-15): `conductor instance upgrade` to v1.10.0 answered
`InstanceStateConflict` while `GET /v1/instances` said the instance was running; the operator
moved the pin in `daemon/binaries.json` by hand for every release since (1.10.0 to 1.13.0).

Read at v1.13.0: the upgrade decides "running" with `self.units.active(&held.unit)`
(`crates/conductor/src/daemon/upgrade.rs:255-261`, and again at `:433` and `:514`). For a boxed
runtime the unit is `substrate:<session>` (`boxed_unit`, `crates/conductor/src/daemon/reconcile.rs:283-285`),
which no service manager knows, so the check is always false and the refusal at `:260` follows.
The reconciler asks substrate for exactly this case (`Daemon::look`, `reconcile.rs:1312-1328`).

## Outcome

The upgrade's three running checks use the reconciler's answer for a boxed runtime (substrate's
`running` for the box's session), so fetch, install, doctor and restart run for a box as they do
for a user unit, and the restart closes the old box before opening the new one.

## Acceptance

- Against a fake substrate whose box for the instance runs, `POST /v1/instances/{name}/upgrade`
  takes a boxed instance to the new binary and ends `Succeeded` once the new runtime answers;
  with the box not running it is still `InstanceStateConflict` (a daemon upgrade test).
