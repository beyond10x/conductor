---
format: aep.planning-md/3
id: design:architecture-next
kind: design
status: draft
title: 'conductor v1: daemon, APIs, substrate placements, agent kinds, SQLite with sync'
summary: 'docs/design/architecture-next.md: conductord on the control host, instance runtimes in substrate boxes (local or remote), federated /v1 APIs, one dashboard served locally and deployed remotely, agent kinds with claude-code first, SQLite synced to the daemon, secrets through the secrets crates, several operators; modelled in ESS first; built on the integration branch v1'
relations:
- designs: epic:architecture-next
- informed_by: epic:host-bridge
- informed_by: epic:session-confinement
- informed_by: epic:conductor-run
- informed_by: epic:multi-instance-host
- informed_by: epic:process-supervision
revision: 2
---
## Design

The design is `docs/design/architecture-next.md`: proposed 2026-11-07 from the operator's request of that day, revised 2026-11-08 on his answers to its open questions (§13 there). Five concepts: `conductord` on the control host reconciling instances from the config and owning the global API; one instance runtime per instance with the loop and the instance API; a placement, which is a Substrate box on this host or on a remote daemon; an agent kind per role slot (`claude-code` first, today's behaviour kept exactly); UIs over the APIs. The store is one SQLite file per instance through Eventlog's port, synced to the daemon. The whole system is modelled in `spec/` first (`story:spec-v1`).

## Supersedes (moved 2026-11-08)

- `design:conductor-on-agency` and its epics `agency-runner`, `dispatch-runner`, `loom-agent-runner`, `cross-repository-waves`: Agency is dropped by the operator.
- `epic:host-bridge` stories bridge-session, ask-bridge-route, ask-direct, dashboard-ask-input, host-dashboard-home, host-dashboard-instances, host-instances-list, status-all-instances: the federated API and the one dashboard do this; `mailbox-operator-messages` stays.
- `epic:session-confinement` stories substrate-client, substrate-tool-server, substrate-gate-provider, session-writes-bwrap, session-forge-credentials: the substrate placement and the secret slots do this.
- Of this epic: placement-loom, placement-mantle, store-postgres.
- `docs/design/roles-and-surfaces.md` §4 (the bridge session) and §5 (isolation through per-instance config directories).

## Keeps

`epic:conductor-run` (the loop is the runtime's core), `epic:multi-instance-host` for disk and build slots on this host, `epic:process-supervision` C5.

## Open questions

Design §15: the controller's trees, with its default.
