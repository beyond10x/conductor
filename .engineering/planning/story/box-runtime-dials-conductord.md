---
format: aep.planning-md/3
id: story:box-runtime-dials-conductord
kind: story
status: implemented
title: A runtime in a local box dials its instance API out to conductord
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/conductor/Cargo.toml
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/daemon/api.rs
- confidence: cited
  path: crates/conductor/src/daemon/dial.rs
- confidence: cited
  path: crates/conductor/src/daemon/mod.rs
- confidence: cited
  path: crates/conductor/src/daemon/reconcile.rs
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/src/instance.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: cited
  path: crates/conductor/tests/instance_api.rs
- confidence: cited
  path: crates/conductor/tests/placement.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/concepts/the-host.md
revision: 19
transitions:
- {from: "draft", to: "proposed", at: "2026-11-14T20:52:23Z", actor: "human:agent", revision: 3, executor: "human:timo"}
- {from: "proposed", to: "active", at: "2026-11-14T20:52:23Z", actor: "human:agent", revision: 4, executor: "human:timo"}
- {from: "active", to: "implemented", at: "2026-11-21T01:32:38Z", actor: "human:unknown", revision: 19, decided_on: {"recorded":{"test_result":2}}}
---
## Why

Substrate refuses `socket(AF_UNIX)` inside a box (its ADR 0039), and a local box serves no
ingress. A runtime at `placement: systemd` (or `linux`) on the local substrate daemon therefore
cannot bind `api.sock`; observed on a host running conductor 1.8.0 against substrate-daemon
0.20.0: `bind <state>/api.sock: Permission denied`, so conductord never reaches the instance API
and the runtime restarts in a loop.

## Outcome

With the new key `daemon.dial_port` set, conductord listens on `127.0.0.1:<dial_port>`. A runtime
it starts in a box on the local substrate daemon gets `CONDUCTOR_API_DIAL` instead of a socket,
binds nothing, keeps a few connections dialed out to conductord through the egress aperture
`conductord`, and answers one instance API exchange on each. conductord routes the instance's
proxied and federated reads over those connections. The box still reaches the model host through
the aperture `conductor`: the session names both apertures, `conductord` first so it listens on
`127.0.0.1` in the box (substrate design 26 § 2, the SDK's `apertures` builder).

## Acceptance

- `daemon.dial_port` is a config key in `spec/domains/config.yaml`, absent 0 (off); the
  generated model and the config reference carry it.
- Each dialed connection opens with `CONDUCTOR-DIAL/1 <instance> <token>`; a connection whose
  token is not the instance's is closed unanswered, so the runtime needs the daemon's token slot
  and refuses to start without it, naming the slot.
- With the key set, a local box's start names the apertures `conductord` and `conductor`, in
  that order, and carries `CONDUCTOR_API_DIAL=127.0.0.1:<dial_port>`; without it, only
  `conductor` and no dial variable (`tests/placement.rs`).
- The real binary serves the instance API over a dialed connection, and refuses a wrong token and
  no token (`tests/instance_api.rs`, `daemon::dial` unit tests).
- `placement: none` keeps its socket, a pod its TCP listener, a box on a remote daemon its relay.

## Boundaries

- The operator declares the aperture on the substrate daemon
  (`--egress-aperture conductord=127.0.0.1:<dial_port>/tcp`); conductor does not write that unit.
- `conductor instance relay` inside a remote box is not changed.
