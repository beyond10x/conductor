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
revision: 1
---
## Why

The substrate route docs/analysis/2026-10-09-substrate-benefits.md § 5 recommends: Claude Code stays outside (login, model API, SendMessage,
`claude agents` keep working) and the commands it runs execute inside substrate. metaharness's
`mcp-serve` is unconfined by design, so conductor serves its own. Supersedes
`story:substrate-session-provider` (Claude Code itself inside substrate), which docs/analysis/2026-10-09-substrate-benefits.md § 3 found does
not fit.

## Acceptance

- Spec first, in `spec/domains/config.yaml`: per role, the tool surface: programs, write subtrees,
  toolchains, network aperture.
- `conductor tools-serve` serves `run`, `file_write` and `file_edit` over stdio MCP from the harness
  catalogue, executing in substrate, with calls served concurrently. Tests: a fake substrate; two
  calls in flight at once; an undeclared program refused; a host without delegation publishes no
  `run` and records why.
- A controller started with this surface keeps SendMessage and Agent among its built-ins and loses
  Bash, Write and Edit (`--tools`, `--strict-mcp-config`, `--mcp-config`). A recorded run in
  `docs/analysis/`: `run cargo test --offline -p conductor` executes in substrate with usage in the
  tool result; a write outside the write subtrees is refused; SendMessage is delivered; the session
  appears in `claude agents --json`.
- The guard sees the `mcp__conductor__*` calls (needs `story:guard-all-tools`).

## Scope

`spec/domains/config.yaml`, `crates/conductor-model/`, `crates/conductor/cli.yaml`,
`crates/conductor/src/tools_serve.rs` (new), `crates/conductor/src/confine/substrate.rs`,
`crates/conductor/src/spawn.rs`, `crates/conductor/tests/`, `docs/analysis/`. After
`substrate-tool-spike` (go), `substrate-client`, `session-envelope-seam` and `guard-all-tools`.
