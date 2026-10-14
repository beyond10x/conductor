---
format: aep.planning-md/3
id: story:install-restarts-dashboard
kind: story
status: draft
title: An install restarts the services that run the old binary
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

On 2026-10-14 the dashboard showed "could not read /data.json" and "config (unreadable)": its
service still ran conductor 0.10.4, installed days earlier, which did not know a config key the
installed 0.13.0 had added. `task install` replaces the binary and leaves running services on the
old one.

## Acceptance

- `conductor doctor` reports each running `conductor` process (dashboard and watch services) whose
  executable is older than the installed binary (`/proc/<pid>/exe` deleted or a different inode),
  with the unit to restart; a test with a fake process table holds it.
- `task install` ends by restarting each systemd user unit that runs `conductor dashboard serve`
  or `conductor watch`, and prints which; a test in `crates/conductor/tests/taskfile.rs` holds the
  step.

## Scope

`crates/conductor/src/doctor.rs`, `crates/conductor/tests/`, `Taskfile.yml`,
`crates/conductor/tests/taskfile.rs`.
