---
format: aep.planning-md/3
id: story:substrate-session-provider
kind: story
status: draft
title: Controller sessions run under substrate
relations:
- decomposes: epic:session-confinement
scope:
- confidence: cited
  path: crates/conductor/src/substrate.rs
- confidence: cited
  path: docs/config.md
revision: 3
---
## Why

The later provider for sessions (architecture-decision-record:external-confinement). A foreground `claude -p` runs under substrate with
enforced limits (docs/analysis/2026-10-09-substrate-sessions.md); a controller session does not fit yet (docs/analysis/2026-10-09-substrate-benefits.md § 3).

## Acceptance

- Once the upstream blocker is cleared: a controller session started through the substrate provider
  meets the epic's acceptance (limits, writes, usage, SendMessage, `claude agents`), and the provider
  is selectable per instance.

## Scope

`crates/conductor/src/` (the substrate provider), `docs/config.md`.
