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
- The watch tick also reads the visible text of each controller running in a tmux pane; on the banner `Usage limit reached` while the usage probe shows an account with headroom, it runs the rotation (`usage.rotate`, today `ais promote --best claude`) once, then resumes the pane (Escape, then a continue message); a pane still on the banner at the next tick is reported in one line. Tests with a fake `tmux` and a fake probe hold both paths.

## Evidence (host supervisor, 2026-11-08 01:45 CET)

Five controllers in tmux panes (beyond10x-atlas, beyond10x-ess, bf-frontends-manager, bf-services-backend, aap-specs) showed `Usage limit reached · limit resets 6am` while `ais` held usable accounts, and the loop reported nothing. The supervisor promoted the best account by hand and resumed each pane with Escape and a message.

## Scope

`crates/conductor/src/watch.rs`, tests, `.agents/conductor.md`.
