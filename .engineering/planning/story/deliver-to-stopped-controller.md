---
format: aep.planning-md/3
id: story:deliver-to-stopped-controller
kind: story
status: active
title: A delivery to a stopped controller is kept for its next start, not retried
summary: SessionNotReachable deliveries to controllers stopped hours earlier
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:11:40Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-07T13:11:40Z", actor: "human:timo", revision: 5}
---
## Why

Observed 2026-11-07 on a running instance: the run unit's journal held `deliver to
<prefix>-<repo>: conductor session deliver ... refused: SessionNotReachable` at 09:08:29Z and
again at 09:13:33Z, for controllers recorded stopped at 04:16Z.

Verified in the code: the loop delivers to the controller whose store state is `Running`
(`Tick::running`, `crates/conductor/src/run.rs:581`); a delivery that fails is noted and the item
kept in the pending file, marked refused by that controller (`deliver_or_keep`,
`crates/conductor/src/run/handlers.rs:504`, `:535`). Verified the same day: `conductor controller controllers` on that instance listed 6 controllers
`Running` whose sessions had stopped at 04:16Z and had no tmux session left, so the loop kept
trying them. Whether the two lines are one item retried or
two items is reported, not verified.

## Acceptance

- A delivery refused `SessionNotReachable` marks that controller stopped in the store, so the next
  watch tick does not deliver to it again; one journal line says so.
- The item is kept for the repository's next controller start, and delivered then; a test drives
  a refused delivery, a second tick (no delivery attempt) and a spawn (the item delivered).
