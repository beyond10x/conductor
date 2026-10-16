---
format: aep.planning-md/3
id: story:implementers-on-tool
kind: story
status: draft
title: Implementers work through tool with Bash denied, measured before and after on context per turn, round trips and refusals
relations:
- decomposes: epic:agent-tools
- depends_on: story:tool-build-commit-report
- depends_on: story:worker-agents
scope:
- confidence: inferred
  path: .agents
- confidence: inferred
  path: .claude/agents
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/collect/usage.rs
- confidence: cited
  path: crates/conductor/src/guard.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/guard/shell.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/src/usage.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: inferred
  path: crates/conductor/tests/tool_calls.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/observation.yaml
revision: 5
---
# Story: implementers-on-tool

## Outcome

The implementer worker has the harness's file tools and the `tool` MCP server and no Bash; on a harness without MCP, Bash passes only a plain `tool <verb> …` argv. The change is measured from transcripts before and after, by one command: context per turn, round trips and refusals.

## Context

docs/design/roles-and-surfaces.md § 6.2 ("No Bash") and § 7 step 3: "Implementers move to it first, with Bash denied. Measured before and after." Epic acceptance 2 asks for the figures from transcripts by the same analysis.

Baselines on record:
- The 48 h analysis (docs/design/agent-tools.md § 1): 302 transcripts, 24,647 calls, 96 % of Bash calls compound, 212 guard refusals (78 retried in the same form), 78 % of output tokens in arguments, ~153k context re-read per turn, 26 % of reads re-reads.
- One director session, 2026-10-16: 200k context in about 35 minutes over 120 Bash calls. Its largest tool results were `conductor` views (50 KB over 15 calls), a loop over the records (34 KB over 40 calls) and `sed` reads of the records (34 KB over 8 calls). This is the director, not an implementer. It is kept here as the per-session reading the command must reproduce from that session's transcript.

The implementer role and its agent definition come from epic:role-names (§ 3: implementer tools read, search, edit, test, check, commit, note, ask). This story narrows that agent's tools and does not create it. Until story:agent-run-grants, an implementer runs as a sub-agent inside its lead's session, so its `tool` calls run on the lead's grant: the harness shares the parent's MCP servers with a sub-agent.

## Domain relations

- Sub-agent → Session, many-to-one: a sub-agent's calls carry its parent session's identity, and the guard tells a sub-agent by the shape of its id. Inferable, inferred from `docs/design/conductor.md` § 7 (guard row for SendMessage) and `crates/conductor/src/guard.rs`; no `ess/1` document declares it.
- Session → Grant, one-to-one; see story:tool-broker-read.

## Acceptance

After the switch, implementer transcripts hold no Bash call, and `conductor resource tool-calls` shows the before and after figures side by side for the same window length.

1. **Spec first.** `spec/domains/observation.yaml` declares the `ToolCallReading` view input (window, role) and its fields: calls, Bash calls, the compound share, mean context per turn, round trips per merged pull request, guard refusals and refusals retried in the same form, and the largest tool results by source. The role config in `spec/domains/config.yaml` gains `surface: mcp | cli`. Validate and regenerate.
2. **Measure.** `conductor resource tool-calls --since <t> [--until <t>] [--role <r>] [--session <id>]` reads transcripts as `usage.rs` does: counts and sizes only, never a line's text. Test `crates/conductor/tests/tool_calls.rs` on fixture transcripts reproduces each figure exactly. Run over the director session above, it gives the 200k-in-35-minutes reading and the three largest result sources. The before reading is recorded as evidence on this story before the switch, with `aep plan artifact evidence`.
3. **Agent.** The implementer agent definition lists Read, Edit, Write, Grep, Glob and the `mcp__tool__*` verbs of its grant, and not Bash. `conductor spawn` passes the lead's start command the `tool mcp` server through `--mcp-config`. Test `crates/conductor/tests/profiles.rs` reads both.
4. **Guard.** A Bash call from an implementer sub-agent is denied and recorded, and the denial names the `tool` verb that replaces the command when one exists. For a role whose `surface` is `cli`, Bash passes only a single `tool <verb> <args>` argv with no `;`, `&&`, pipe, redirection or substitution. Tests in `crates/conductor/tests/guard_rules.rs`; `tests/guard_latency.rs` shows the allow path does not grow.
5. **After.** At least 24 h after the switch, the same command over a window of the same length is recorded as evidence beside the before reading, for context per turn, round trips and refusals. A figure that got worse is reported to the maintainer as a finding. It is not hidden.

## Scope

- `spec/domains/observation.yaml`, `spec/domains/config.yaml`, `crates/conductor-model` (regenerated)
- `crates/conductor/src/usage.rs`, `crates/conductor/src/collect/usage.rs`, `crates/conductor/cli.yaml`
- `crates/conductor/src/guard.rs`, `crates/conductor/src/guard/rules.rs`, `crates/conductor/src/guard/shell.rs`, `crates/conductor/src/spawn.rs`
- the implementer agent adapter and profile under `.claude/agents/` and `.agents/` (created by epic:role-names)
- `crates/conductor/tests/tool_calls.rs` (new), `crates/conductor/tests/guard_rules.rs`, `crates/conductor/tests/profiles.rs`, `docs/config.md`

## Out of Scope

- Denying Bash to the lead, reviewer, planner or releaser (story:agent-run-grants moves the lead; the other workers follow with their verbs).
- Payloads of Write and Edit (epic Out of Scope).
- An operating-system sandbox for Bash (story `bash-sandbox`).

## Ambiguities

- None. The implementer agent comes from story:worker-agents (epic:role-names).
