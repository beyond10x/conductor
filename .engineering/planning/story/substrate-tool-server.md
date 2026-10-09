---
format: aep.planning-md/3
id: story:substrate-tool-server
kind: story
status: draft
title: A session's commands run inside substrate through conductor's tool server
relations:
- decomposes: epic:session-confinement
- depends_on: story:substrate-tool-spike
- depends_on: story:substrate-client
- depends_on: story:session-envelope-seam
- depends_on: story:guard-all-tools
- supersedes: story:substrate-session-provider
- depends_on: story:session-writes-bwrap
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/src/substrate.rs
- confidence: cited
  path: crates/conductor/src/tools_serve.rs
- confidence: cited
  path: crates/conductor/tests/guard_hook.rs
- confidence: cited
  path: crates/conductor/tests/tools_serve.rs
- confidence: cited
  path: docs/analysis/substrate-tool-server-run.md
- confidence: cited
  path: spec/domains/config.yaml
revision: 5
---
## Why

The substrate route docs/analysis/2026-10-09-substrate-benefits.md § 5 recommends: Claude Code stays outside (login, model API, SendMessage,
`claude agents` keep working) and the commands it runs execute inside substrate. metaharness's
`mcp-serve` is unconfined by design, so conductor serves its own. Supersedes
`story:substrate-session-provider` (Claude Code itself inside substrate), which docs/analysis/2026-10-09-substrate-benefits.md § 3 found does
not fit.

## Acceptance

- The harness keeps its `systemd` provider (and bwrap layer); the tool server is a second layer
  per role. Spec first, in `spec/domains/config.yaml`: per role, `tools: builtin | substrate`, and
  for `substrate` the programs, toolchains and network aperture. Write subtrees are the role's
  `writable` paths that lie inside the session's tree; no second list.
- The envelope measurement gains the tool layer: `substrate` with `run` published, or withheld and
  why. Under `strict`, a withheld `run` is a shortfall.
- `conductor tools-serve` serves `run`, `file_write` and `file_edit` over stdio MCP from the harness
  catalogue, executing in substrate through `story:substrate-client`, with calls served
  concurrently. Tests (`crates/conductor/tests/tools_serve.rs`): a fake substrate; two calls in
  flight at once; an undeclared program refused; a host without delegation publishes no `run` and
  the measurement says why.
- A test reads the start command for a `tools: substrate` controller: `--tools` keeps SendMessage
  and Agent and leaves out Bash, Write and Edit; `--strict-mcp-config` and `--mcp-config` name the
  server.
- A guard test: a hook payload for `mcp__conductor__run` reaches the guard's rules and is decided
  (`crates/conductor/tests/guard_hook.rs`, after `story:guard-all-tools` routes every tool).
- A recorded run (`docs/analysis/substrate-tool-server-run.md`), one controller:
  - its init event's tool list shows SendMessage and the `mcp__conductor__` verbs, and no Bash;
  - `run cargo test --offline -p conductor-cli` executes in substrate with usage in the tool result;
  - a write outside the write subtrees is refused;
  - SendMessage is delivered, and the session appears in `claude agents --json`.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model/`, `crates/conductor/cli.yaml`,
`crates/conductor/src/tools_serve.rs` (new), `crates/conductor/src/substrate.rs`,
`crates/conductor/src/spawn.rs`, `crates/conductor/src/confine/mod.rs`,
`crates/conductor/tests/tools_serve.rs` (new), `crates/conductor/tests/guard_hook.rs`,
`docs/analysis/substrate-tool-server-run.md` (new). After `substrate-tool-spike` (go),
`substrate-client`, `session-envelope-seam`, `session-writes-bwrap` (`confine/mod.rs`) and
`guard-all-tools`.
