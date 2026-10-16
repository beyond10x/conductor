---
format: aep.planning-md/3
id: decision-blocker:worker-grant-after-lead-ends
kind: decision-blocker
status: cleared
title: Nobody has decided whether a worker's grant ends when the lead session that compiled it ends
relations:
- blocks: epic:agent-tools
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-16T19:26:54Z", actor: "human:timo", revision: 4}
---
# Decision blocker: worker-grant-after-lead-ends

## Question

When a lead's session ends (stopped, handed over or lost) while workers it started with `agent run` still run, does each worker's grant end with it, stay until its own time to live, or pass to the lead's successor?

## Why it is open

docs/design/roles-and-surfaces.md § 6.4 settles that a child's grant is the named tools minus anything outside the parent's grant, and § 6.2 that a grant carries a time to live. Neither says what a parent's end does to a child's grant. No `ess/1` document declares a Grant entity yet, and no code answers it.

## Options

1. The worker's grant ends when the lead's session ends; the worker is stopped.
2. The worker keeps its grant until its own time to live, which never exceeds the lead's.
3. The worker's grant passes to the lead's successor session, which may narrow it.

## What it stops

The revocation behaviour of story:agent-run-grants. That story compiles and caps the grant at start and leaves this behaviour out of scope.

## Decision

Decided by conductor-dev on 2026-10-16 under the operator's approval of docs/design/roles-and-surfaces.md. It can be undone, so it was not escalated.

- A worker's grant ends with its lead's session. The broker revokes the grants of every child of a session that ends, and a revoked worker's next call is refused with `GrantRevoked` ("lead ended").
- This matches rules.md item 146 (a session's builds end with it) and § 6.4 (a child's grant never exceeds its parent's; with no parent there is nothing for it to be part of).
- If work is lost this way, a later story may let a lead hand a grant over to its successor.
