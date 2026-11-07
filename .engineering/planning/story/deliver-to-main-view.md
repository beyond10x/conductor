---
format: aep.planning-md/3
id: story:deliver-to-main-view
kind: story
status: draft
title: session deliver types into the main conversation, never a focused sub-agent view
summary: Messages reached a sub-agent view the pane had open
revision: 2
---
## Why

The host supervisor, 2026-11-07: `conductor session deliver` types its text into whichever view the Claude Code pane in the session's tmux server has focused. With a sub-agent's transcript view open, three messages in one evening reached the sub-agent, which declined each as not its role; the session itself never saw them until the supervisor pressed Escape by hand.

## Acceptance

- `conductor session deliver` returns the pane to the main conversation before it types (Escape, then a check that the main prompt is focused) and refuses with one line naming the session when it cannot confirm that view, exit 1.
- A test over a fake tmux pane: with a sub-agent view focused, the delivered text lands in the main view; a pane that stays in another view is refused and named.
- The delivery record names the view it typed into.
