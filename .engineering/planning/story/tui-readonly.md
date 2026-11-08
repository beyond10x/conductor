---
format: aep.planning-md/3
id: story:tui-readonly
kind: story
status: implemented
title: conductor tui lists instances and sessions over the global API, attaches and watches
summary: 'a terminal UI over the daemon: instances, sessions, attach, watch, log'
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:federated-views
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:24Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:25Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-08T19:46:04Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Operator, 2026-11-07: a TUI wrapper around tmux to switch between instances, see settings and enter agents. Design §4.3; the public page's "The TUI" section.

## Acceptance

- `conductor tui` (and `conductor` with no arguments at a terminal) shows the instances pane (state, open decisions, KPI flags), the sessions pane of the selected instance (role, state, context used, current work) and the settings pane (read-only in this story), from the global API.
- `Enter` attaches the selected session through `GET /v1/instances/{name}/sessions/{session}/attach` (a PTY stream); `ctrl-]` then `d` returns to the TUI with the session untouched; `w` watches; `l` follows the typescript.
- It works from any machine that reaches the daemon (`--daemon <url>`).
