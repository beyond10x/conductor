---
format: aep.planning-md/3
id: story:substrate-tool-spike
kind: story
status: draft
title: 'Spike: a session''s commands run inside substrate through an MCP server'
relations:
- decomposes: epic:session-confinement
revision: 1
---
## Why

docs/analysis/2026-10-09-substrate-benefits.md § 5 finds the tools-in-substrate route (the harness unconfined, its commands run inside
substrate through an MCP server) the better substrate route, and lists what nobody has run: whether
`run` is published for an adopted tree, git in a linked tree, offline cargo, the one-core clamp,
SendMessage and sub-agents beside an MCP server.

## Acceptance

A page in `docs/analysis/` with the commands and their output for the three steps of docs/analysis/2026-10-09-substrate-benefits.md § 5.4,
and a go/no-go for: the tool route, and gates run through the in-process host on an adopted tree.
Each step records:
- `b10x-harness tools … --substrate-embedded` under a delegated scope: `run` published or withheld;
- a scratch binary on `b10x-harness-substrate`: `git status`, `cargo test --offline -p conductor`
  (wall time, CPU, memory peak), a write outside `target` refused or not;
- a throwaway `claude --bg --tools "SendMessage,Agent,Read,Grep,Glob" --strict-mcp-config
  --mcp-config <file>`: the verbs seen, SendMessage delivered, listed in `claude agents --json`,
  a long `run` blocking a sub-agent or not, the MCP client's timeout on a 15-minute call.

## Scope

`docs/analysis/` (new page); scratch code outside the repository, not committed.
