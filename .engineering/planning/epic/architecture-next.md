---
format: aep.planning-md/3
id: epic:architecture-next
kind: epic
status: active
title: 'conductor v1: a daemon, confined instances, abstract agents, one store'
summary: conductord reconciles instances from the config and federates their APIs; every instance and agent runs in a substrate box, local or remote; any agent kind in any role, claude-code first; SQLite everywhere with sync; one dashboard served locally and deployed remotely; several operators; all of it modelled in ESS first and built on the integration branch v1
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:23Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:23Z", actor: "human:timo", revision: 4}
---
## Outcome

One daemon on the control host reconciles every instance from the config and serves one API; every instance runs in a substrate box at a placement, local or remote, with its own API; one dashboard, a TUI and the CLI read the federated API; every role is filled by an agent kind, `claude-code` first with today's behaviour; the store is one SQLite file per instance synced to the daemon; several operators with grants. The whole system is modelled in `spec/` first. Design: `docs/design/architecture-next.md`.

## Acceptance

1. The domains and components of design §3 are in `spec/`, validated, with one scenario per new command, and `crates/conductor-model` is regenerated from them (story:spec-v1).
2. `conductor daemon serve` starts, stops and restarts instance runtimes so that the running set equals the config; `GET /v1/instances` lists them with placement, state, binary version and last seen; a second reconciliation with nothing changed does nothing (a test counts actions); the registry file no longer exists.
3. Every instance serves `/v1/` on its runtime socket with today's dashboard routes renamed; the daemon proxies `instances/{name}/*`, serves the dashboard app at `/` and on its TLS listener, and the per-instance dashboard server no longer exists.
4. `sessions`, `decisions`, `unanswered`, `kpis`, `events` and `ask` on the global API return the union of every instance's rows, each with `instance`; an unreachable instance is a row with `state: unreachable`, never an error.
5. Every store is one SQLite file through Eventlog's port; the daemon's synced copy of every instance is complete within `cadence.sync` of each event; `conductor instance restore` seeds a new box from the copy; the tree is migrated once.
6. Every role slot is an agent kind at a placement; `claude-code` passes today's session scenarios and tests unchanged; a second, run-to-exit kind fills a worker slot in a test.
7. A placement is a substrate box, local or remote, with the profile of design §6, secrets as sealed slots and a named refusal when the box cannot be given; conductor keeps no confinement provider of its own.
8. The config has `globals`, precedence role over instance over global, operators with grants, `config show --effective`, and `PUT /v1/config` validates, commits, records `ConfigChanged` and reconciles; every write names its operator.
9. The live instance moves to `v1` once, reversibly; `v1` merges into `main` after the cutover has held one day; the release is 1.0.0.

## Out of Scope

A second agent kind beyond the test double (`takt`, `codex`, a Loom agent turn: stories after v1); the tool broker (`epic:agent-tools`); the role renames (`epic:role-names`); Agency, Loom and mantle (dropped, design §12).

## Ambiguities

Design §15: the controller's trees, with its default.
