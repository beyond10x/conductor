---
format: aep.planning-md/3
id: story:watch-session-waiting
kind: story
status: draft
title: A controller held at a permission prompt is a watch line
summary: A controller sat at a guard prompt unseen until a pane scan
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
# Story: watch-session-waiting

## Why

Host teacher, relayed by the supervisor, 2026-11-08: a controller stood `waiting` in
`claude agents --json` on a permission prompt ("Dangerous rm operation on possibly-empty variable
path") until a person scanned its pane at 11:00 local time. The watch prints `session silent:`
from transcript modification times (`crates/conductor/src/watch.rs`), and a session held at a
permission prompt has no line of its own.

## Acceptance

1. When a controller's pane shows a permission prompt, the watch prints one journal line
   `session waiting: <name> <id> <prompt text, first line>`, once per prompt. A test with a
   typescript fixture holding a prompt.
2. A pane without a prompt prints no such line. Test.

## Out of Scope

Answering the prompt; that stays with the supervisor (its profile).

## Ambiguities

None.
