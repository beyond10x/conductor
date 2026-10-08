---
format: aep.planning-md/3
id: story:guard-all-tools
kind: story
status: draft
title: Every tool call reaches the guard, and an unknown tool is denied
relations:
- depends_on: story:instance-session-names
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/guard.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_hook.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 4
---
## Why

The guard sees five tools: Edit, Write, NotebookEdit, SendMessage and Bash. The settings files
route only those five to the hook (`.claude/conductor-settings.json`,
`.claude/controller-settings.json`), and `crates/conductor/src/guard.rs` allows any other tool name
it is given. A write-capable tool outside the five, such as a filesystem or forge MCP tool, or a
tool Claude Code adds later, is never checked. Found by the pre-publication security review.

## Acceptance

- The settings in this repository route every tool to the hook (matcher `*`). The instance's own
  settings files (`~/.b10x/conductor/<instance>/settings/`) are outside the repository: the wave's
  close copies the matcher there and reports it.
- The guard allows a tool outside its ruled set only if it is on a read-only allowlist the config
  can extend per role; every other tool is denied with a reason naming the allowlist.
- A test sends a payload for an unknown tool and expects a denial; one for an allowlisted read
  tool and expects an allow.
- Conductor may write `<records>/docs/analysis/**` with file tools (taken over from the archived
  `story:guard-bash-write-targets`): `RECORD_DIRS` in `guard/rules.rs` gains `docs/analysis`; a test
  allows a Write there and still denies one to `<records>/other/`.
- The allow path for the five ruled tools does not get slower (`tests/guard_latency.rs`).

## Scope

`crates/conductor/src/guard.rs`, `crates/conductor/src/guard/rules.rs`, `spec/domains/config.yaml`
(the allowlist), `crates/conductor-model/`, `.claude/*-settings.json`,
`crates/conductor/tests/guard_hook.rs`. After `instance-session-names` (shared `guard/rules.rs`,
`config.yaml`).
