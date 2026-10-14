---
format: aep.planning-md/3
id: story:decisions-notify
kind: story
status: draft
title: An escalation reaches the operator at once and again at 30 minutes
summary: Watch lines on escalation and when unanswered
relations:
- decomposes: epic:dashboard-decisions
revision: 1
---
## Why

"its not acceoptable to have important blocking decissions sitting unnoticed for 2 hours" (the operator, 2026-10-14). Rules item 79 now asks conductor to notify at once and again at 30 minutes by hand (epic:dashboard-decisions).

## Acceptance

- `decision escalate-request` makes the watch print `escalated <O|H> <request-id> <question>` at once, and the watch prints `unanswered <O|H> <request-id> <minutes>` once when an escalated request passes 30 minutes unanswered and again at its deadline; the profile tells conductor to send a PushNotification on each; tests drive both lines with a fixed clock.
- `conductor status` and the dashboard overview show the count of unanswered O/H items and the oldest one's age.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/src/status.rs`, `.agents/conductor.md`, tests.
