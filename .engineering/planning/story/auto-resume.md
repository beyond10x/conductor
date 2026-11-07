---
format: aep.planning-md/3
id: story:auto-resume
kind: story
status: active
title: The watch resumes limit-stopped controllers itself
relations:
- decomposes: epic:deterministic-steps
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:11:40Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:11:40Z", actor: "human:timo", revision: 3}
---
## Why

Rules item 64 asks conductor to resume every limit-stopped controller at once; conductor does it by prompt.

## Acceptance

- The watch finds a controller whose transcript tail holds a rate-limit error or that `claude agents` shows blocked with no pid, and runs `claude respawn <id>` (or the documented resume) once per stop, on the same model; a second 429 is reported in one line; tests with a fake `claude`.

## Scope

`crates/conductor/src/watch.rs`, tests, `.agents/conductor.md`.
