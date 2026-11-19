---
format: aep.planning-md/3
id: story:doctor-usage-probe-notifier
kind: story
status: active
title: Doctor names a missing usage probe and a missing notifier
summary: Two doctor rows, neither failing
refs:
- provider: github
  reference: beyond10x/conductor#205
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T08:24:07Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-18T08:24:07Z", actor: "human:unknown", revision: 3}
---
## Why

Issue #205: the headroom probe (`ais`) and the notifier (`notify-send`) were missing on the host; each
tick erred into `runtime.log` only, and `conductor doctor` named neither.

## Outcome

- `conductor doctor` shows one row for the usage probe (the `usage.probe` command or `ais`): present,
  or missing and optional ("limits resume at their reset time"), and one row for the operator
  notifier (`notify-send`): present, or missing with what is lost. Neither row fails doctor.

## Acceptance

- `tests/doctor.rs` `doctor_names_a_missing_notifier_and_an_unrunnable_probe` with a PATH lacking both. Fails on main.
