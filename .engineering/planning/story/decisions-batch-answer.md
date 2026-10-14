---
format: aep.planning-md/3
id: story:decisions-batch-answer
kind: story
status: draft
title: The operator answers several decisions at once by clicking options
summary: 'Write path: one guarded endpoint records answers as the operator''s'
relations:
- decomposes: epic:dashboard-decisions
revision: 1
---
## Why

The operator wants to "easily make decissions by clicking options and then sending them in batches" (epic:dashboard-decisions).

## Acceptance

- On the Decisions page the operator answers each open request by its kind (Yes or No; one option; or several options), may add his words to each, and sends the batch; one POST records each answer as `record-operator-decision` with the chosen option or options, his words (or "chosen on the Decisions page" when none) and the recording session `dashboard`, and answers the batch row by row: recorded, or the refusal; a request answered meanwhile is refused, not recorded twice.
- The endpoint takes only answers to open escalated requests, and only with the token the page was served with; the guard refuses a tool call of conductor or a controller that names the endpoint; tests hold each refusal.
- Each recorded answer prints one watch line `answered <request-id> <option>` at once, so conductor relays it within one watch interval.
- The website's security pages state the boundary and its residual risk (a local process outside the guard).

## Scope

`spec/domains/decision.yaml`, `crates/conductor/src/dashboard.rs`, `crates/conductor/src/guard.rs`, `crates/conductor/src/watch.rs`, `crates/conductor/dashboard/`, `website/docs/security/`, tests.
