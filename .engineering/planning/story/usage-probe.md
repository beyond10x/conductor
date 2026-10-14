---
format: aep.planning-md/3
id: story:usage-probe
kind: story
status: draft
title: Conductor sees each account's usage headroom and forecasts when the pool runs out
relations:
- decomposes: epic:usage-continuity
revision: 1
---
## Why

Conductor cannot see usage: nothing it reads says how much of each account's windows is left or
when they reset, so a stop is only seen after it happens (epic:usage-continuity).

## Acceptance

- Spec first: config key `usage.probe` names a command that prints the accounts as JSON (account
  id, active, health, and per window `used_percent` and `resets_at`); the snapshot records each
  reading with its time; a test with a fake probe holds the record. No email or account name is
  stored: an account is its id.
- The watch computes the pool headroom per window (remaining percent summed over usable accounts)
  and the burn rate from the last readings, and prints `usage: pool runs out ~<time>, next reset
  <time>` once per change of the forecast hour; `conductor status` shows the same line; tests hold
  both.
- `conductor doctor` reports the configured rotation watcher (`usage.watcher`, a systemd user unit)
  as running or not.

## Scope

`spec/domains/config.yaml`, `spec/domains/observation.yaml`, `crates/conductor-model`,
`crates/conductor/src/config.rs`, `crates/conductor/src/collect/`, `crates/conductor/src/watch.rs`,
`crates/conductor/src/status.rs`, `crates/conductor/src/doctor.rs`, `crates/conductor/tests/`.
