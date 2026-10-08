---
format: aep.planning-md/3
id: story:guard-write-forms-and-config-scope
kind: story
status: implemented
title: The guard reads redirections and the config scope precisely
tags:
- session-review
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:18:00Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T20:18:00Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:46Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Found by a review of an instance's sessions after its records moved out of the product checkout (two independent reviewers). Four false denials in one hour: `2>&1` and a `>` inside a quoted awk program were read as
write forms, and the config rule covers all of `~/.b10x/conductor/`, which since the move holds the
records directory too, so a read-only `ls … 2>&1` of a records path was denied.

## Acceptance

- A redirection counts as a write only outside quotes and only when its target is a file
  (`2>&1`, `>&2`, `>/dev/null` are no writes).
- The config rule covers the config file and the instance's state directory, not its records
  directory, which keeps its own rule (conductor's to write).
- Tests for each case in `crates/conductor/tests/guard_rules.rs`, and the earlier denials kept.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/src/guard/shell.rs`, `crates/conductor/tests/guard_rules.rs`.
