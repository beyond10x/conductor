---
format: aep.planning-md/3
id: story:dashboard-ask-input
kind: story
status: draft
title: The host dashboard's one input reaches the bridge or one director
relations:
- decomposes: epic:host-bridge
- depends_on: story:ask-bridge-route
- depends_on: story:host-dashboard-home
revision: 2
---
# Story: dashboard-ask-input

## Outcome

The host dashboard's one input sends a request to the bridge, or to one director with `@<instance>`, and shows each answer labelled with its instance.

## Context

docs/design/roles-and-surfaces.md § 4 ("a channel the operator types into, from the terminal (`conductor ask "<text>"`) or from one input on the dashboard"). The input is the same channel as `conductor ask` (story:ask-direct, story:ask-bridge-route), on the host page (story:host-dashboard-home). Like the Decisions page's answer endpoint (epic:dashboard-decisions, Boundary), it is a write path: it carries the page's per-load token, records the sender as `dashboard`, and the guard refuses a session's tool call that reaches it.

## Domain relations

- Ask → Instance and Ask → Route as story:ask-direct and story:ask-bridge-route record them, with their citations; this story adds none.

## Acceptance

1. `POST /api/ask` with `@alpha <text>` delivers to `alpha`'s director inbox only, and without `@` to the bridge's inbox only; `crates/conductor/tests/dashboard_host.rs` holds both.
2. A post without the page's token is refused and delivers nothing; the same test holds it.
3. The page shows each answer as it arrives, the bridge's route line first and every answer labelled `[alpha]` or `[beta]`; the same test reads the page's answer feed.
4. The guard refuses a session's tool call that reaches `/api/ask`; `crates/conductor/tests/guard_rules.rs` holds it.

## Scope

`crates/conductor/src/dashboard.rs`, `crates/conductor/src/dashboard/overview.rs`, `crates/conductor/src/ask.rs`, `crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/dashboard_host.rs`, `crates/conductor/tests/guard_rules.rs`, `website/docs` (the security page: the new write path and its residual risk).

## Out of Scope

Streaming every instance's events to the page (story:dashboard-stream). A host-side history of asks.

## Ambiguities

None.
