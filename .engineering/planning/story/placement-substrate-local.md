---
format: aep.planning-md/3
id: story:placement-substrate-local
kind: story
status: draft
title: 'placement: substrate runs an instance in a local substrate workspace with sealed secrets'
summary: 'the substrate provider: profile from design §5, bubblewrap plus cgroup through substrate-daemon on the control host, named refusals'
relations:
- decomposes: epic:architecture-next
- depends_on: story:daemon-reconcile
- depends_on: story:secrets-source
revision: 1
---
## Why

Operator, 2026-11-07: "the actual instance should run in a confined way ... look at substrate from B10X ... Docker, or Firecracker ... Substrate should support all of that." Design §5; ADR external-confinement.

## Acceptance

- A `substrate` provider behind `confine::Provider` admits a `ConfinementProfile` through `substrate_host::confinement::open` (or the SDK against `substrate-daemon`'s socket) with: argv and cwd of `conductor instance serve`, the read-only roots, one workspace holding today's six writable places, cpu/memory/pids/storage limits, default-deny egress with the forge and model hosts, and every credential as a sealed memfd slot; nothing in argv or env (a test reads the exec request).
- Sessions inside the box are substrate PTY sessions running tmux as today; `conductor session attach|deliver|list` keep working unchanged against a local substrate placement.
- When substrate refuses (`exec.sandbox-unavailable` or a box short of the request), `strict` does not start the instance and records the named reason; `report` falls back to `placement: host` and records that.
- The daemon keeps `substrate-daemon.service` running on the control host (`epic:process-supervision` C5) and pins the substrate crates by git revision as harness does.
