---
format: aep.planning-md/3
id: epic:process-supervision
kind: epic
status: draft
title: Every process that outlives a tool call runs supervised, owned and visible
revision: 1
---
# Epic: process-supervision

## Outcome

Every long-lived process a conductor session starts (a dev server, a browser, a watcher, a build) is a Substrate host-process record. Each has an owner session, a dispatch, a lease and logs, and is visible in one list and on the dashboard. A forgotten one ends when its lease expires.

## Why

The operator's process-supervision plan of 2026-10-16. On the host that day, sessions had left user units and timers, detached processes, unawaited background wait loops (23 of them conductor-dev's) and Docker containers that restart themselves, some of them weeks old. A written rule had not stopped any of them. The operator chose Substrate as the single owner: its host-process family, run by `substrate-daemon` as a user service with `Delegate=yes`.

## Stories (to draft; the order is the plan's)

- **C1 `conductor proc start|ls|stop|renew|logs`:** a built-in wrapper over the Substrate SDK. It fills the owner from the controller or worker session and the dispatch id. Spec first, against the Substrate ADR draft. It lands after Substrate's S1–S7 ship.
- **C5:** conductor installs and keeps `substrate-daemon.service` running, the one sanctioned user service, and `conductor doctor` checks it.
- **C2:** `conductor build run` keeps its behaviour but runs its command as a Substrate host process, so builds appear in `proc ls`. Its disk-watchdog pause then uses the cgroup freeze (story:build-run-pause-visible).
- **C3:** a Processes panel on the dashboard, from `proc ls --json`. The owner links to its session; expired leases and `--keep` records older than 24 h are highlighted.
- **C4:** the controller and worker profiles require `conductor proc` for anything that outlives one tool call.
- **C6:** the watch sweeps `proc stop --expired` and reports `--keep` records.
- **C7 watchers:** `conductor watch add --on <trigger> --deadline <dur> [--notify <session>]`. The triggers are `exit:<proc>`, `log:<proc>:<regex>`, `file:<path>:<regex>` and `port:<n>`. A watcher always resolves exactly once: it matched, the process exited without a match, or the deadline passed. A watcher without a deadline is refused. It comes after C1 and Substrate's S12, the per-process event stream.
- **C8:** the resolution arrives in the owning session as a message over the cross-session route. It carries the watcher id, the outcome, the matched line or exit code, the elapsed time, and on failure the last 20 log lines.
- **C9:** the controller and worker profiles forbid `until`/`sleep`/`inotifywait`/`tail --pid` wait loops. A session registers a watcher and ends its turn. This is the product form of rules items 139 and 143.

Evidence for C7–C9, 2026-10-15 and 2026-10-16: 23 wait loops of conductor-dev's sub-agents ran for a day, and one loop spun on a missing `inotifywait`.

## Depends on

Substrate's host-process ADR, which the operator accepts, then its S1–S7 and S10 releases. Conductor tells conductor-dev of each.

## Out of Scope

The operator's global Claude Code configuration (the plan's part G), and the substrate repository's own work.
