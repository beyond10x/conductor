---
format: aep.planning-md/3
id: design:architecture-next
kind: design
status: draft
title: 'The next architecture: daemon, APIs, placements, store'
summary: 'docs/design/architecture-next.md: conductord on the control host, instance runtimes at placements (host, substrate, mantle), federated /v1 APIs, UIs over them, Store trait with tree, sqlite and postgres and a mirror'
relations:
- designs: epic:architecture-next
- informed_by: epic:host-bridge
- informed_by: epic:session-confinement
- informed_by: epic:conductor-run
- informed_by: epic:multi-instance-host
- informed_by: epic:process-supervision
revision: 1
---
## Design

The design is `docs/design/architecture-next.md` in the repository (proposed 2026-11-07 from the operator's request of the same day). Four components: `conductord` on the control host reconciling instances from the config and owning the global API; one instance runtime per instance with the loop and the instance API, confined at a placement (`host`, `substrate` local or remote, `mantle`); UIs (one dashboard, the TUI, the CLI) over the federated API; and a `Store` trait with `tree`, `sqlite` and `postgres` backends, mirrored at the daemon for remote instances.

## Supersedes (the operator moves them)

- `docs/design/roles-and-surfaces.md` §4 (the bridge session) and §5 (isolation through per-instance config directories).
- `epic:host-bridge` stories bridge-session, ask-bridge-route, ask-direct, dashboard-ask-input, host-dashboard-home, host-dashboard-instances, host-instances-list, status-all-instances (the federated API and the one dashboard do this; `mailbox-operator-messages` stays).
- `epic:session-confinement` stories substrate-client, substrate-tool-server, substrate-gate-provider, session-writes-bwrap (the `substrate` placement does this; session-forge-credentials is answered by secret slots).

## Keeps

`epic:conductor-run` (the loop is the runtime's core), `epic:multi-instance-host` for the `host` placement, `epic:process-supervision` C5, `design:conductor-on-agency` and the runner epics untouched.

## Open questions

Section 11 of the design: the authoritative store of a remote instance, where the dashboard is served, the secrets source (class H), the controller's trees, the first tuning grant. Each carries its default.
