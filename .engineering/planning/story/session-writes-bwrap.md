---
format: aep.planning-md/3
id: story:session-writes-bwrap
kind: story
status: draft
title: A session can write only its own places
relations:
- decomposes: epic:session-confinement
- depends_on: story:session-limits-systemd
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/conductor/src/confine.rs
- confidence: cited
  path: docs/design/conductor.md
revision: 4
---
## Why

The guard does not check where a Bash command writes (design § 7); architecture-decision-record:external-confinement puts that boundary
outside the harness. bubblewrap around the session enforces it in the kernel and keeps the
checkout, network and credentials where they are (docs/analysis/2026-10-09-substrate-benefits.md § 2).

## Acceptance

- With the provider on, the session runs under `bwrap` with the root read-only and the envelope's
  `writable` paths bound writable; the network and the harness's own state stay usable.
- Tests: the generated `bwrap` argument list for a controller and for conductor; a recorded live run
  where a Bash write outside the writable paths fails and one inside succeeds.
- The README Security model and design § 7 say what the box enforces and what the guard still does.

## Scope

`crates/conductor/src/` (the `confine` module), `README.md`, `docs/design/conductor.md`,
`crates/conductor/tests/`. After `session-limits-systemd` (same module).
