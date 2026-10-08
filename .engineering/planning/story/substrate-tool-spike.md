---
format: aep.planning-md/3
id: story:substrate-tool-spike
kind: story
status: active
title: 'Spike: a session''s commands run inside substrate through an MCP server'
relations:
- decomposes: epic:session-confinement
scope:
- confidence: cited
  path: docs/analysis/substrate-tool-spike.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T23:21:55Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-08T23:21:55Z", actor: "human:timo", revision: 7}
---
## Why

docs/analysis/2026-10-09-substrate-benefits.md § 5 finds the tools-in-substrate route (the harness unconfined, its commands run inside
substrate through an MCP server) the better substrate route, and lists what nobody has run: whether
`run` is published for an adopted tree, git in a linked tree, offline cargo, the one-core clamp,
SendMessage and sub-agents beside an MCP server.

## Acceptance

A page `docs/analysis/substrate-tool-spike.md` with the commands and their output for the three
steps of docs/analysis/2026-10-09-substrate-benefits.md § 5.4, and a go/no-go for: the tool route, and gates run through the in-process host on
an adopted tree. Each step records:
- `b10x-harness tools … --substrate-embedded` under a delegated scope: `run` published or withheld;
- a scratch binary on `b10x-harness-substrate`: `git status`, `cargo test --offline -p conductor`
  (wall time, CPU, memory peak), a write outside `target` refused or not;
- a throwaway `claude --bg --tools "SendMessage,Agent,Read,Grep,Glob" --strict-mcp-config
  --mcp-config <file>`: the verbs seen, SendMessage delivered, listed in `claude agents --json`,
  a long `run` blocking a sub-agent or not, the MCP client's timeout on a 15-minute call.

## Safety on a shared host

This host runs conductor and its controllers under the same Claude Code daemon. The spike never
restarts, stops or moves that daemon, and never stops a session it did not start. Every session it
starts runs in a scratch directory under `~/.cache/conductor-spikes/`, and is stopped and removed
(`claude stop`, `claude rm`) before the page is written; the page lists their ids. A way that can
only be tried by touching the running daemon is tried with a separate daemon (its own
`CLAUDE_CONFIG_DIR`), or recorded as not tried and why.

## Scope

`docs/analysis/substrate-tool-spike.md` (new); scratch code under `~/.cache/conductor-spikes/`,
not committed, built only with 10G or more free on `/` (`df -h /`).
