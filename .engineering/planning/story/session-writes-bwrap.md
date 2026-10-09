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
  path: crates/conductor/src/confine/bwrap.rs
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/confine/systemd.rs
- confidence: cited
  path: crates/conductor/tests/confine_bwrap.rs
- confidence: cited
  path: docs/analysis/session-writes-run.md
- confidence: cited
  path: docs/design/conductor.md
revision: 9
---
## Why

The guard does not check where a Bash command writes (design § 7); architecture-decision-record:external-confinement puts that boundary
outside the harness. bubblewrap around the session enforces it in the kernel and keeps the
checkout, network and credentials where they are (docs/analysis/2026-10-09-substrate-benefits.md § 2). Supersedes
`story:guard-bash-write-targets`, which extends the Bash heuristics the ADR retires; its other
promise, conductor's file-tool writes to `<records>/docs/analysis`, moves to `story:guard-all-tools`.

## Acceptance

- bwrap is a layer of the `systemd` provider, applied when the role's request has `writable` paths:
  inside its unit, the session runs under `bwrap` with the root read-only and those paths bound
  writable. The measurement adds the writable paths applied. Under `strict`, `bwrap` missing or a
  path not bound is a shortfall and the start refuses.
- Its doctor probe: `bwrap` is installed and unprivileged user namespaces are allowed.
- Tests (`crates/conductor/tests/confine_bwrap.rs`): the generated `bwrap` argument list for a
  controller and for conductor; the measurement with the paths; a `strict` refusal with `bwrap`
  missing (fake).
- One recorded live run (`docs/analysis/session-writes-run.md`), a controller under the box:
  - a Bash write outside the writable paths fails with a filesystem error; one inside succeeds;
  - network: the session completes a model turn, and `gh api rate_limit` exits 0;
  - the harness's own state: the session appears in `claude agents --json`, receives a SendMessage,
    and resumes with `claude --resume`.
- The README Security model and design § 7 say what the box enforces and what the guard still does.

## Scope

`crates/conductor/src/confine/bwrap.rs` (new), `crates/conductor/src/confine/mod.rs`,
`crates/conductor/src/confine/systemd.rs`, `README.md`, `docs/design/conductor.md`,
`crates/conductor/tests/confine_bwrap.rs` (new), `docs/analysis/session-writes-run.md` (new). After
`session-limits-systemd` (it wraps inside the unit).

## Learned from the spike (docs/analysis/session-box-spike.md)

- Writable, besides the request's `writable`: the session's `TMPDIR` (its settings' `env`; the
  Bash tool fails with `EROFS` without it) and `/run/user/<uid>/cc-socks` (SendMessage cannot reach
  the session without it).
- `~/.claude.json` is not persisted through a bind of the single file: Claude Code writes it through
  `~/.claude.json.lock` and `~/.claude.json.tmp.<n>` in `~`. The live run records which bind makes
  it persist (for example a writable `~` overlay limited to those names), or the gap stays open in
  the README Security model.
- Not observed in the spike: an OAuth token refresh under `bwrap`. The live run notes whether one
  happened.
