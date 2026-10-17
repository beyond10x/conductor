---
format: aep.planning-md/3
id: story:decision-relay-recorded-answers
kind: story
status: draft
title: Every recorded answer reaches its controller once, whoever recorded it
relations:
- decomposes: epic:dashboard-decisions
revision: 2
---
# Story: decision-relay-recorded-answers

## Outcome

Every answer to a decision request reaches the controller that asked, once, whoever recorded it. An operator answer recorded outside conductor's chat, for example from a desktop UI through `decision record-operator-decision` and `decision answer-escalated-request`, is relayed by conductor at its next turn as `[DECISION <request-id>] <option> <reason> (<decision-id>)`.

## Context

Requested by the operator through the loom-canvas session (2026-10-17): loom-canvas becomes his front door to conductor. Today recording an answer is not delivering it: conductor sends the `[DECISION]` line only from chat (docs/design/conductor.md § 3, message table), so an answer recorded elsewhere never reaches the controller.

Decided by conductor-dev: the delivery is a record on the request, not a test of `recorded_by`. Every answer, conductor's included, goes through the same path, and "once" is enforced by the record, not by conductor's memory.

## Acceptance

1. An answered request carries `delivered_at` and `delivered_by` (spec `decision.yaml`), unset until delivered.
2. `conductor decision undelivered-answers [--format text|json]` lists the answered requests not yet delivered: request id, repository, option, reason, decision id, `recorded_by`. Test: an answer recorded by `answer-escalated-request` with `--recorded-by loom-canvas` is listed.
3. `conductor decision mark-answer-delivered --request-id <id>` sets the delivery once. A second call refuses `AlreadyDelivered` and exits non-zero. Test.
4. The migration marks every request answered before this release as delivered, so nothing is sent twice. Test on a fixture store.
5. The conductor profile's loop runs `undelivered-answers` at the start of every turn, sends each `[DECISION]` line to its controller, then marks it delivered. If the controller is not running, the line waits for the controller's next start, and the profile says so.
6. The guard lets conductor run `mark-answer-delivered` and denies it to controllers and workers. Test in `guard_rules`.

## Out of Scope

Delivery through the mailbox transport (story:mailbox-operator-messages).

## Ambiguities

None.
