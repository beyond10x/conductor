---
format: aep.planning-md/3
id: epic:disk-protocol
kind: epic
status: draft
title: 'Disk is a protocol the binary runs: reserve, reclaim, watch, escalate, enforce'
revision: 1
---
## Why

On 2026-10-13 work across the instance stopped twice on free disk. Build slots are granted in
conversation (a session asks conductor, conductor reads `df` and answers), nothing frees space when
a floor is crossed, and a build that does not fit simply waits. A standing rule (records item 113)
asks conductor to free disk, but a rule depends on a session remembering it; the operator: "a
standing rule is not enough — we need some sort of protocol", "the level of autonomy is not
enough".

## Outcome

Disk is a protocol the `conductor` binary runs, not a conversation:
1. **Reserve.** Every build, gate or test run holds a disk reservation from one host ledger for its
   expected peak (`conductor build run --reserve <size> -- <command>`). The binary grants it at
   once when free disk minus live reservations minus the floor covers it; nobody is asked.
2. **Reclaim.** When it does not fit, the binary reclaims before it answers: it frees what needs no
   decision (build output of idle managed trees, build output in worktree archives, caches past
   their age, snapshots past retention) until it fits, and grants.
3. **Watch the run.** The command runs in its own process group under a disk watchdog: under
   `gate_stop` it is paused and reclaim runs; over `gate_resume` it resumes. Its peak is measured
   and recorded with the reservation, and the reservation is released when it exits, killed or not.
4. **Escalate only what needs hands.** When reclaim cannot reach the size, the binary refuses with
   the shortfall, records an operator item (`decision hands-todo`) naming what is left by path and
   size and the one command only a person can run, and the watch reports it once.
5. **Enforce.** The guard denies a session's build, gate or test command (`cargo build|test|check|
   clippy|nextest`, `task check|gate|install`, `npm run build`) that does not run under `conductor
   build run`, and its denial names the command to use. The watch runs reclaim once per fall under
   `disk_admit`, so space is freed before anyone asks.

Conductor's profile stops granting build slots by message; it reads the ledger instead.

## Stories

- `story:disk-protocol-spec` — the host ledger, reservation lifecycle, reclaim and their refusals,
  specified first (from `spec/drafts/host.yaml`).
- `story:disk-reclaim` — `conductor gc reclaim`, and the watch running it under the floor.
- `story:build-run` — `conductor build run`: reserve, reclaim, run under the watchdog, measure,
  release (supersedes `story:gate-run`).
- `story:guard-requires-build-run` — the guard's denial of an unreserved build.
- `story:disk-escalation` — the operator item on a shortfall, and the profiles' change.

## Not covered

CPU and memory reservations (`story:session-limits-systemd`); several hosts.
