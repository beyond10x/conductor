---
format: aep.planning-md/3
id: story:guard-all-tools
kind: story
status: draft
title: Every tool call reaches the guard, and an unknown tool is denied
revision: 1
---
## Why

The guard sees five tools: Edit, Write, NotebookEdit, SendMessage and Bash. The settings files
route only those five to the hook (`.claude/conductor-settings.json`,
`.claude/controller-settings.json`), and `crates/conductor/src/guard.rs` allows any other tool name
it is given. A write-capable tool outside the five, such as a filesystem or forge MCP tool, or a
tool Claude Code adds later, is never checked. Found by the pre-publication security review.

## Acceptance

- The settings route every tool to the hook (matcher `*`).
- The guard allows a tool outside its ruled set only if it is on a read-only allowlist the config
  can extend per role; every other tool is denied with a reason naming the allowlist.
- A test sends a payload for an unknown tool and expects a denial; one for an allowlisted read
  tool and expects an allow.
- The allow path for the five ruled tools does not get slower (`tests/guard_latency.rs`).

## Scope

`crates/conductor/src/guard.rs`, `crates/conductor/src/guard/rules.rs`, `spec/domains/config.yaml`
(the allowlist), `.claude/*-settings.json`, `crates/conductor/tests/guard_hook.rs`.
