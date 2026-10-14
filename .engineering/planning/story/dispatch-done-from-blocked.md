---
format: aep.planning-md/3
id: story:dispatch-done-from-blocked
kind: story
status: draft
title: A done report closes a sent or blocked dispatch
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

`dispatch report-done` refuses a dispatch that is not `Started` ("only a started dispatch is
done"). On 2026-10-14 conductor had to record an unblock by hand before a done that followed a
blocked report, and in an outside user's trial a controller that never sent `started` left
conductor recording it by hand before its `done` was accepted.

## Acceptance

- Spec first: `ReportDone` moves a dispatch to `Done` from `Sent`, `Started` or `Blocked`
  (`spec/domains/dispatch.yaml`); `ess specify validate --path spec` exits 0 and the conformance
  scenarios for the new transitions pass.
- `conductor dispatch report-done` accepts a sent and a blocked dispatch and records the done line
  in the records' `dispatches/` log; tests in `crates/conductor/tests/` hold both; a cancelled or
  done dispatch is still refused.

## Scope

`spec/domains/dispatch.yaml`, `crates/conductor-model`, `crates/conductor/src/dispatch.rs`,
`crates/conductor/tests/dispatch.rs`.
