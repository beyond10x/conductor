---
format: aep.planning-md/3
id: story:context-watch-by-envelope
kind: story
status: active
title: The context watch labels only sessions conductor started
summary: Label a session by its envelope under <state>/sessions/, never by its bare name, so a foreign session never triggers a controller's hand-over
relations:
- decomposes: epic:deterministic-steps
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T01:25:57Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T01:25:58Z", actor: "human:timo", revision: 4}
---
## Why

The run loop's context watch labels a session by its repository directory, else by its bare name (`crates/conductor/src/watch.rs`, doc near line 1250), and `crates/conductor/src/run/handlers.rs:168` delivers `[HANDOVER <repo>]` to that repository's controller. On 2026-11-08 the journal at 02:17:39 read `context: codespec 215k tokens (session 0d74d27e)`; 0d74d27e is a session not started by conductor, named `codespec`, with its cwd at the checkouts root (284,820 tokens at 02:22). The codespec controller, 55f21597, was at 118,519 tokens at 02:19, got 4 HANDOVER deliveries and handed over at 02:18 with its wave half done (conductor-teacher, relayed by the host supervisor).

## Acceptance

- A `context:` line and a `[HANDOVER <repo>]` delivery come only from a session conductor started, matched by its envelope under `<state>/sessions/` (its session id), never by a bare name; a session with no envelope produces no `context:` line. A watch test with a foreign session named like a repository and a controller of that repository holds it.
- The `context:` line names the session id it measured.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/src/run/handlers.rs`, tests.
