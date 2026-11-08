---
format: aep.planning-md/3
id: story:run-context-maintainer
kind: story
status: draft
title: The run loop's context handler acts on the maintainer's line
summary: 'The maintainer ran 4 h over its threshold: the handler looked for a controller and dropped the line'
revision: 2
---
## Why

The host supervisor, 2026-11-07: the run journal held `context: conductor-dev 203k tokens (session …)` at 19:24:45 and nothing after it for four hours, while the maintainer session ran on to 347k tokens. The line is emitted at `crates/conductor/src/watch.rs:1274`; its handler, `crates/conductor/src/run/handlers.rs:168-170`, asks `tick.running(repo)` for a controller named like the session, finds none for the maintainer, and returns `Ok(false)`, so neither the `[HANDOVER]` delivery nor the 1.25x restart runs.

## Acceptance

- The `context:` handler recognises the maintainer's session (its role in `host.roles`) and, over its threshold, delivers the hand-over request once per session, as it does for a controller.
- Over 1.25x the threshold, with a hand-over file newer than the session's start that names `pushed: yes` on every branch, the loop stops the session and starts it with `conductor session start --role maintainer`; without that file it reports the session to the supervisor instead of stopping it.
- A handler that matches no session names the line in the journal (`context: <name> matched no session`), never returns silently.
- Tests over fixtures: a maintainer line gets one delivery; a line matching nothing is named.
