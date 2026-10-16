---
format: aep.planning-md/3
id: decision-blocker:director-tool-verbs
kind: decision-blocker
status: cleared
title: Nobody has decided which tool verbs the director gets, so its conductor commands have no replacement
relations:
- blocks: epic:agent-tools
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-16T19:26:55Z", actor: "human:timo", revision: 4}
---
# Decision blocker: director-tool-verbs

## Question

Which `tool` verbs does the director get for its records, decisions, dispatch, metrics and messages, and does the director move to `tool` with Bash denied like the workers?

## Why it is open

docs/design/roles-and-surfaces.md § 3 names the director's tools (records, decisions, dispatch, metrics, messages), but § 6.3 lists no verb for any of them, and the epic denies Bash to "every worker", which the director is not. Today the director calls `conductor decision`, `conductor dispatch`, `conductor message` and `conductor goal`.

## What it stops

Removing the director's commands from the `conductor` binary. story:conductor-drops-session-commands removes only commands whose callers have moved to `tool`, and leaves the director's commands in place until this is decided.

## Decision

Decided by conductor-dev on 2026-10-16 under the operator's approval of docs/design/roles-and-surfaces.md. It can be undone, so it was not escalated.

- The director gets its own `tool` verb group: its records, decisions (raise, answer within its grant, escalate, amend), dispatch, metric reads and messages. Design § 3 already lists these as the director's tools.
- They are drafted as one story after story:agent-run-grants. story:conductor-drops-session-commands then depends on it, so that in the end no session holds the admin binary (§ 6.1).
