---
format: aep.planning-md/3
id: epic:session-confinement
kind: epic
status: draft
title: Harness sessions run in a box conductor controls
relations:
- implements: architecture-decision-record:external-confinement
revision: 1
---
## Outcome

Every harness session conductor starts runs unconfined inside and inside a box conductor controls
from outside: CPU, memory and process limits per instance and session, writes limited to the
session's own places, usage attributed per session, and a session that cannot be boxed either not
started or started and reported as unboxed, as the instance's config says (architecture-decision-record:external-confinement, the operator's
decision, following metaharness).

## Acceptance

With a test instance on this host, each checked by a test or a recorded run:
- a controller session runs in its own cgroup under the instance's slice, with the configured
  `MemoryMax`, `CPUWeight` and `TasksMax` (`systemctl --user show`), and still answers SendMessage
  and appears in `claude agents --json`;
- a Bash write by that session outside its checkout, trees, build cache and scratch fails with a
  filesystem error;
- the dashboard shows that session's CPU time and memory from its own cgroup;
- with `confinement.mode: strict` and no provider available, `conductor:start` refuses.

Substrate is a later provider: today it cannot host a controller session (docs/analysis/2026-10-09-substrate-sessions.md; docs/analysis/2026-10-09-substrate-benefits.md § 3); the
upstream blockers name what it lacks.

## Findings this rests on

- `claude --bg` sessions are children of Claude Code's daemon (`claude daemon run`, parent pid 1),
  not of the command that starts them; wrapping the start command confines nothing (docs/analysis/2026-10-09-substrate-benefits.md § 2).
- A foreground `claude -p` runs inside a substrate exec with enforced memory and pids limits
  (exit 0 under 1 GiB / 64 processes, `exec.memory-limit` under 64 MiB) (docs/analysis/2026-10-09-substrate-sessions.md).
- Plain Linux gives the same kernel counters and limits through a delegated `systemd-run --user`
  scope or unit, and bubblewrap 0.12.0 is installed (docs/analysis/2026-10-09-substrate-benefits.md § 1, § 2).
