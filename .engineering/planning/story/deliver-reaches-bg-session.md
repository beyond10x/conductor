---
format: aep.planning-md/3
id: story:deliver-reaches-bg-session
kind: story
status: implemented
title: session deliver reaches a claude --bg session, and a refused delivery does not stop a live controller
scope:
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T23:54:16Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T23:54:16Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-11T03:28:26Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from conductor 1.0.1, a trial instance, placement
`none`. Every lead runs from `claude --bg`: at v1.0.1, crates/conductor/src/confine/mod.rs:215-226
serves only the `none` provider (`Unboxed`) and `systemd` is `Unserved`, and spawn.rs:271 launches
`claude --bg`. But `session deliver` types only through `<state>/sessions/<name>.tmux`
(session.rs:322), which a `--bg` session never has. The runtime journal shows `session deliver:
refused: SessionNotReachable: session example-ui: its launcher socket <state>/sessions/example-ui.tmux is
absent`, and the loop then records the live controller stopped. `conductor controller
controllers` then prints "no controller is Running or Paused" while three leads run and keep
writing to the mailbox, a [DECISION-REQUEST] and two [NEED]s among them; no answer reaches them,
and the next cycle will hit SessionRunning when it respawns them.

story:sessions-start-with-claude-bg (archived) moved sessions to `claude --bg` and said delivery
goes through the harness's message route, not keystrokes; delivery was not moved with it.

## Acceptance

- `session deliver` reaches a session started with `claude --bg` without a tmux socket, and the
  delivery is recorded; a test with a fake `claude` holds it.
- A refused delivery to a session `claude agents` shows live does not record its controller
  stopped; the controller stays Running and the refusal is one line.

## Fix variants the reporter accepts

- `spawn` gives the lead a launcher that `session deliver` can type into; or
- a delivery refused for a live `claude --bg` session falls back to the session inbox and does
  not record the controller stopped.

The trial instance's director asked on 2026-11-09 for this story to go first in the next wave.
