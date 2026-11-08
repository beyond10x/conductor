---
format: aep.planning-md/3
id: epic:architecture-next
kind: epic
status: draft
title: 'The next architecture: a daemon, federated APIs, confined placements and an abstracted store'
summary: conductord reconciles instances from the config and federates their APIs; instances run confined, local or remote; one dashboard and a TUI; the store is a trait with a mirror
revision: 1
---
## Outcome

One daemon on the control host reconciles every instance from the config and serves one API; every instance runs confined at a placement, local or remote, with its own API; one dashboard, a TUI and the CLI read the federated API; the store is a trait with file, sqlite and database backends and a mirror at the daemon. Design: `docs/design/architecture-next.md`.

## Acceptance

1. `conductor daemon serve` starts, stops and restarts instance runtimes so that the running set equals the config; `GET /v1/instances` lists them with placement, state, binary version and last seen; a second reconciliation with nothing changed does nothing (a test counts actions).
2. Every instance serves `/v1/` on its runtime socket with today's dashboard routes renamed; the daemon proxies `instances/{name}/*`, serves the dashboard app once, and `conductor_<instance>-dashboard.service` no longer exists.
3. `sessions`, `decisions`, `unanswered`, `kpis`, `events` and `ask` on the global API return the union of every instance's rows, each with `instance`; an unreachable instance is a row with `state: unreachable`, never an error.
4. `store:` selects `tree://`, `sqlite://` or `postgres://`; the daemon's mirror of a `sqlite://` instance is complete within `cadence.mirror` of each event; `conductor instance restore` seeds a new box from the mirror.
5. `placement: substrate` runs the runtime and its sessions in a substrate workspace with the profile of design §5, secrets as sealed slots, and a named refusal when the host cannot provide the box; `placement: host` behaves as today.
6. The config has `globals`, precedence role over instance over global, `config show --effective`, and `PUT /v1/config` validates, commits, records `config-changed` and reconciles.
7. Every migration step of design §10 is reversible by the means named there, and the host keeps working through steps 1 to 5 with sessions where they are today.

## Out of Scope

Agency and dispatch runners (`design:conductor-on-agency`); the tool broker (`epic:agent-tools`); the role renames (`epic:role-names`).

## Ambiguities

Section 11 of the design lists the operator's open questions with defaults.
