---
format: aep.planning-md/3
id: story:request-amend-escalated
kind: story
status: draft
title: An escalated decision request can be completed in place
revision: 3
---
# Story: request-amend-escalated

## Outcome

An escalated decision request can be completed in place: its kind, lettered options, recommended and default letters, deadline and dry run are set without withdrawing and raising it again, so a re-sent operator item carries what the profile asks for.

## Context

A conductor NEED (2026-10-16): O-20261015-substrate-adopt and H-20261015-slack-adapter-removal were escalated with no default or deadline. `decision escalate-request` refuses a request that is already escalated (`RequestStateConflict`, "only an open request is escalated", crates/conductor/src/decide.rs:1052). Conductor fell back to re-sending them by text, so the Decisions page shows them without a default or deadline.

## Acceptance

1. Spec first: `AmendEscalatedRequest` (request id; optional kind, lettered options, recommended letters, default letters, deadline, dry run, reason) is declared in `spec/domains/decision.yaml` for a request in state Escalated. It is refused for any other state with `RequestStateConflict`, and a scenario reads the amended fields in the `DecisionRequests` view.
2. `conductor decision amend-escalated-request --request-id <id> --reason <r> [--kind] [--lettered-options <json>] [--recommended-letters] [--default-letters] [--deadline <rfc3339>] [--dry-run <text>]` (the flag names of `escalate-request`; the CLI tree guard, crates/conductor/tests/cli_tree.rs, requires flags equal to the command's input fields) writes the change and an `amended` history entry. `decision requests` and the dashboard's Decisions page show the new fields. A default letter that names no option is refused.
3. Tests in `crates/conductor/tests/` hold an amend of an escalated request, the refusal for an open or answered one, and the refusal of an unknown default letter.
4. `.agents/conductor.md` says to complete an escalated item with this command rather than by text.

## Out of Scope

Amending a recorded decision (story:decision-amend).

## Ambiguities

None.
