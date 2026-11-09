---
format: aep.planning-md/3
id: story:context-watch-by-envelope
kind: story
status: active
title: The context watch labels only sessions conductor started
summary: Label a session by its envelope under <state>/sessions/, never by its bare name, so a foreign session never triggers a controller's hand-over
relations:
- decomposes: epic:deterministic-steps
scope:
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: crates/conductor/tests/run.rs
- confidence: inferred
  path: crates/conductor/tests/watch.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T01:25:57Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T01:25:58Z", actor: "human:timo", revision: 4}
---
## Why

The run loop's context watch labels a session by its repository directory, else by its bare name (`crates/conductor/src/watch.rs`, doc near line 1250), and `crates/conductor/src/run/handlers.rs:168` delivers `[HANDOVER <repo>]` to that repository's controller. On 2026-11-08 the journal at 02:17:39 read `context: codespec 215k tokens (session 0d74d27e)`; 0d74d27e is a session not started by conductor, named `codespec`, with its cwd at the checkouts root (284,820 tokens at 02:22). The codespec controller, 55f21597, was at 118,519 tokens at 02:19, got 4 HANDOVER deliveries and handed over at 02:18 with its wave half done (conductor-teacher, relayed by the host supervisor).

It recurred: the journal at 02:41:34 read `session gone: codespec 0d74d27e` and `context: codespec 314k tokens (session 7fd2f7c2)`, another session named `codespec` in the checkouts root (started 01:58); at 02:43:38 the loop `stopped beyond10x-codespec (55f21597-…)` through `hand_over` and `restart_with`, and at 02:44:18 its respawn was refused `AlreadyRunning` (controller 01a1b8d3, session `beyond10x-codespec`). The live controller is 8721d069, the id its envelope `<state>/sessions/beyond10x-codespec.envelope.json` records under `session.session_id` (host supervisor, 2026-11-08).

The fix has two sides: the watch labels no session outside a repository's directory by its name (`watch.rs`, `Watch::context`), and the loop delivers or restarts only for a `context:` line whose session id is the one the controller's envelope records (`handlers.rs`, `measured_is_enveloped`); a controller without an envelope id keeps the old behaviour.

## Acceptance

- A `[HANDOVER <repo>]` delivery and a hand-over restart come only for a `context:` line whose session id is the one the controller's envelope under `<state>/sessions/` records, never by a bare name; a run test with a foreign session's line holds it.
- A session outside every repository's directory gives no `context:` line, whatever its name (a role's session excepted); a watch test with a foreign session named like a repository and a controller of that repository holds it.
- The `context:` line names the session id it measured.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/src/run/handlers.rs`, tests.
