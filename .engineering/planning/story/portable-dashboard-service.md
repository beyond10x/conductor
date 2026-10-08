---
format: aep.planning-md/3
id: story:portable-dashboard-service
kind: story
status: implemented
title: The dashboard tasks run without systemd
tags:
- first-user-review
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:03:12Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T20:03:12Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:45Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Reported by a first user on macOS (Darwin, BSD userland) against `21734cd`, one local repository, observe-only first cycle. `task dashboard` and `dashboard:stop` use `systemctl --user` / `systemd-run`; macOS has
neither.

## Acceptance

- Where `systemctl` is absent, `task dashboard` starts `conductor dashboard serve` in the
  background with its pid in the instance's state directory, and `dashboard:stop` stops that pid;
  where systemd is present the user service stays.
- A test runs both tasks with a PATH that has no `systemctl`.

## Scope

`Taskfile.yml` tasks `dashboard`, `dashboard:stop`; `crates/conductor/tests/taskfile.rs`.
