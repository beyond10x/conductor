---
format: aep.planning-md/3
id: story:session-writes-bwrap
kind: story
status: draft
title: A session can write only its own places
relations:
- decomposes: epic:session-confinement
- depends_on: story:session-limits-systemd
- supersedes: story:guard-bash-write-targets
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/conductor/src/confine.rs
- confidence: cited
  path: docs/design/conductor.md
revision: 5
---
## Why

The guard does not check where a Bash command writes (design § 7); architecture-decision-record:external-confinement puts that boundary
outside the harness. bubblewrap around the session enforces it in the kernel and keeps the
checkout, network and credentials where they are (docs/analysis/2026-10-09-substrate-benefits.md § 2). Supersedes
`story:guard-bash-write-targets`, which extends the Bash heuristics the ADR retires.

## Acceptance

- With the provider on, the session runs under `bwrap` inside its systemd unit: the root read-only,
  the request's `writable` paths bound writable.
- Its doctor probe: `bwrap` is installed and unprivileged user namespaces are allowed.
- Tests: the generated `bwrap` argument list for a controller and for conductor.
- One recorded live run in `docs/analysis/`, a controller under the box:
  - a Bash write outside the writable paths fails with a filesystem error; one inside succeeds;
  - network: the session completes a model turn, and `gh api rate_limit` exits 0;
  - the harness's own state: the session appears in `claude agents --json`, receives a SendMessage,
    and resumes with `claude --resume`.
- The README Security model and design § 7 say what the box enforces and what the guard still does.

## Scope

`crates/conductor/src/confine/bwrap.rs` (new), `README.md`, `docs/design/conductor.md`,
`crates/conductor/tests/`, `docs/analysis/`. After `session-limits-systemd` (it wraps inside the
unit).
