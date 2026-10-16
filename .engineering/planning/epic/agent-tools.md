---
format: aep.planning-md/3
id: epic:agent-tools
kind: epic
status: draft
title: Sessions call short conductor tools instead of long shell commands
revision: 2
---
# Epic: agent-tools

## Outcome

Sessions act only through a confined tool surface, never through arbitrary Bash:
- `tool` is a separate binary with no admin command, a thin client of the instance's broker over a socket mounted into the session's workspace.
- The broker holds the forge credentials, the bot route and the store writes. It checks each call against the caller's grant and records it.
- On Claude Code the surface is an MCP server, and Bash is denied to every worker.
- Leads compile each worker's grant with `agent run`; a child's grant never exceeds its parent's.
- `tool define` makes aliases of verbs the caller holds, checked per step at call time.

Design: docs/design/roles-and-surfaces.md § 6, approved by the operator on 2026-10-16. This replaces the earlier ranking of `conductor` verbs, which the operator redirected.

## Why

The operator, 2026-10-16:
- "I really do not want at all that my workers and agents … are able to call arbitrary bash."
- "if we give them the conductor tool, they are leaving the sandbox."

The 48 h analysis: of 24,647 calls, 96 % of Bash calls were compound, there were 212 guard refusals, and tool-call arguments were 78 % of output tokens.

## Acceptance

1. The broker and `tool` ship with read (`show`, `search`, `diff`), build (`test`, `check`, `fmt`), `commit`, `send` and `report`. Implementers move to them with Bash denied.
2. Before and after, from transcripts by the same analysis: context per turn, round trips, refusals.
3. Then the rest of § 6.3 (forge, plan, team, memory with TTL, wait, `agent run`), with grants compiled by the lead.
4. Then `tool define` aliases with session, team and instance scope and expiry. Use is recorded, and an alias used often becomes a draft story.
5. The `conductor` binary loses its session commands once no session calls them.

## Out of Scope

Code payloads in Write and Edit.

## Ambiguities

None. The operator chose `tool`, MCP and Bash denied.
