---
format: aep.planning-md/3
id: story:sessions-outside-root-explained
kind: story
status: implemented
title: Conductor reads a redacted session row as by design, and its own session is placed
tags:
- first-user-review
scope:
- confidence: cited
  path: crates/conductor/src/collect/sessions.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T19:55:35Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T19:55:35Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:45Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Reported by a first user on macOS (Darwin, BSD userland) against `21734cd`, one local repository, observe-only first cycle. The conductor session reported "the sessions collector records name, cwd, repository null for
every session" as a defect. Those sessions ran outside `checkouts.root` and are redacted by design
(`crates/conductor/src/collect/sessions.rs` module doc). Conductor's own session runs in the
records directory, outside `checkouts.root`, so it is always redacted in its own snapshot.

## Acceptance

- `.agents/conductor.md` says in one line that a redacted row is a session outside the checkouts
  root and the managed trees.
- The sessions collector places a session whose working directory is the instance's records
  directory as role `conductor` (name and id kept), and a test holds it.

## Scope

`.agents/conductor.md`, `crates/conductor/src/collect/sessions.rs`, `crates/conductor/tests/`.
