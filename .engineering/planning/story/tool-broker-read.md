---
format: aep.planning-md/3
id: story:tool-broker-read
kind: story
status: draft
title: A session reads files through the instance's broker with tool show, search and diff, over a socket and an MCP server, within its grant
relations:
- decomposes: epic:agent-tools
- informed_by: story:views-open-as-reader
- supersedes: story:substrate-tool-server
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: inferred
  path: crates/conductor-broker
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/confine/places.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/no_organization.rs
- confidence: inferred
  path: crates/tool
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/config.yaml
- confidence: inferred
  path: spec/domains/tool.yaml
revision: 5
---
# Story: tool-broker-read

## Outcome

A session of an instance calls `tool show`, `tool search` and `tool diff`, from the CLI or from the `tool` MCP server, and each call goes over the session's own socket to the instance's broker, which checks it against the session's grant, runs it, records it, and answers in one line, or refuses it naming the grant rule it broke.

## Context

docs/design/roles-and-surfaces.md § 6.1, § 6.2 and § 6.3 (approved by the operator, 2026-10-16), § 7 step 3. The 48 h analysis (docs/design/agent-tools.md § 1: 24,647 calls, 78 % of output tokens in arguments, 26 % of file reads re-reading a file already read) puts bounded reads first. This story is the tracer bullet through every layer: spec, the two new crates, the socket, the grant check, the call record and the MCP surface, with the three read verbs. Later stories add verbs to the same broker.

## Domain relations

No `ess/1` document declares a broker, a grant or a tool call yet; acceptance 1 declares them. Until then each relation below rests on the approved design, not on a typed model:
- Instance → Broker, one-to-one; the instance owns its broker, which does not outlive it. Decided in docs/design/roles-and-surfaces.md § 6.1 ("the instance's broker").
- Session → Grant, one-to-one, every session has exactly one; the session owns it, and the grant ends at its time to live or with the session. Decided in § 6.2 ("every session starts with one"; "time to live").
- Session → Socket, one-to-one; the caller's identity is the socket, never a token. Decided in § 6.2 ("the socket is the session's").
- Broker → ToolCall record, one-to-many; recorded in the instance's store, outliving the session. Decided in § 6.2 ("checked against the grant, run by the broker, and recorded").
- Session → Instance, many-to-one; the broker refuses a call whose session belongs to another instance. Decided in § 5b.

## Acceptance

A session with a grant naming `show`, `search` and `diff` reads a file of its checkout through `tool show` from the MCP server and from the CLI, and a call outside its grant is refused with the grant rule named and recorded.

1. **Spec first.** A new domain `spec/domains/tool.yaml` declares `Grant` (tools with limits: repositories, paths, max output bytes, time to live), `ToolCall` (session, verb, outcome, duration, output bytes; no argument text beyond paths) and the errors `NotGranted { rule }`, `GrantExpired`, `ForeignInstance` and `UnknownSession`, with typed `relations:` for every relation above. `spec/components.yaml` gains the components `conductor-broker` and `tool`. A role's `grant` is a config field in `spec/domains/config.yaml`, defaulting to the role's default tools of design § 3. `ess specify validate --path spec` passes and `task regen` is clean.
2. **Two crates.** `crates/conductor-broker` (library: socket server, grant check, call record, verbs) and `crates/tool` (the `tool` binary: a thin client with no store access and no admin command). `crates/conductor` gains `conductor broker serve`, which runs the instance's broker; the instance's services start it the way they start the dashboard.
3. **Socket per session.** When `conductor spawn` or `conductor session start` starts a session, the broker creates that session's socket under the instance's state directory and the session's environment names it (`TOOL_SOCKET`); a boxed session sees it at a fixed path in its workspace. Test `crates/conductor-broker/tests/identity.rs`: a call on session A's socket acts as A whatever it claims; a socket of a session that stopped is refused `UnknownSession`; a session of another instance is refused `ForeignInstance`.
4. **Verbs.** `tool show <path> [--lines a:b | --fn <symbol>] [--max n]` is bounded by `--max` and the grant's output limit and prints `unchanged since <time>` for a re-read of an unchanged range in the same session; `tool search <pattern> [--path p]`; `tool diff [--stat]`, `--stat` by default. Paths outside the grant's paths are refused `NotGranted`. Each verb prints one line on success, and on failure at most 40 lines and a log reference. Test `crates/conductor-broker/tests/read_verbs.rs`.
5. **Grant check and record.** Every call is recorded as a `ToolCall`; a refusal exits 1 with one line naming the grant rule (`NotGranted: <rule>`). Test `crates/conductor-broker/tests/grant.rs`: a verb absent from the grant, a path outside it and an expired grant are each refused, recorded, and named.
6. **MCP.** `tool mcp` serves the granted verbs over stdio MCP, one MCP tool per verb, backed by the same socket. Test `crates/tool/tests/mcp.rs`: `tools/list` returns exactly the granted verbs; `tools/call show` returns what `tool show` prints.
7. **No admin command.** Test `crates/tool/tests/cli_tree.rs`: the `tool` command tree holds none of `conductor`'s operator commands (`init`, `setup`, `config`, `trust`, `upgrade`, `doctor`, `dashboard`, `gc`). `crates/conductor/tests/no_organization.rs` scans both new crates; `crates/conductor/tests/isolated_home.rs` rules hold for their tests.
8. The guard's allow path does not grow: `tests/guard_latency.rs` reads as before.

## Scope

- `spec/domains/tool.yaml` (new), `spec/domains/config.yaml`, `spec/components.yaml`, `crates/conductor-model` (regenerated)
- `crates/conductor-broker/` (new), `crates/tool/` (new), `Cargo.toml` (workspace members)
- `crates/conductor/cli.yaml`, `crates/conductor/src/spawn.rs`, `crates/conductor/src/session.rs`, `crates/conductor/src/confine/places.rs`
- `crates/conductor/tests/no_organization.rs`, `Taskfile.yml` (the broker service), `docs/config.md`

## Out of Scope

- Build, commit, send and report (story:tool-build-commit-report); every other verb of § 6.3.
- Denying Bash to anybody (story:implementers-on-tool).
- Grants compiled by a lead (story:agent-run-grants).

## Ambiguities

None.

## Supersedes

story:substrate-tool-server (epic:session-confinement). Both offer a session an MCP server, keyed by a per-role config value, that replaces Bash in its start command. That story serves a general `run` verb (any declared program, run inside substrate), which the operator ruled out on 2026-10-16 ("no arbitrary bash"); the broker serves only named verbs, each checked against a grant. What survives from it is substrate as a possible execution back end for the broker's build verbs, which a later story may take up. Its `tools:` role key is not created; this story's `grant` key replaces it. That story's own body was not changed here: it belongs to another decomposition.
