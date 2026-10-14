---
format: aep.planning-md/3
id: story:usage-resume
kind: story
status: draft
title: Every session resumes by itself when a usage window resets
relations:
- decomposes: epic:usage-continuity
- depends_on: story:usage-probe
revision: 1
---
## Why

After the limit lifted on 2026-10-14, every session waited until the operator told conductor-dev,
which messaged fourteen sessions by hand. Conductor itself was stopped too, so the resume cannot
depend on a model turn.

## Acceptance

- When every usable account is exhausted, `conductor usage block` (run by the watch) records the
  block and the earliest reset, and arms a systemd user timer for it; at the timer, `conductor usage
  resume` (no model involved) runs the configured rotation command (`usage.rotate`), checks the
  probe shows headroom, restarts the conductor session if it is stopped, and leaves a resume
  message for conductor to send every live session; if the probe still shows none, it re-arms for
  the next reset. Tests with a fake probe, a fake rotate command and a fake `claude` hold each path.
- `.agents/conductor.md` says that on that message conductor tells every live session to continue
  where it left off, and re-dispatches any that does not move within one cycle; a profile test holds
  it.

## Scope

`spec/domains/session.yaml`, `crates/conductor-model`, `crates/conductor/src/cli.rs`,
`crates/conductor/src/usage.rs` (new), `crates/conductor/src/watch.rs`, `crates/conductor/tests/`,
`.agents/conductor.md`, `Taskfile.yml`.
