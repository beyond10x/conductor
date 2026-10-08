---
format: aep.planning-md/3
id: epic:session-confinement
kind: epic
status: draft
title: Harness sessions run in a box conductor controls
relations:
- implements: architecture-decision-record:external-confinement
revision: 3
---
## Outcome

Every Claude Code session conductor starts (conductor, conductor-dev, controllers, and the workers
a controller spawns as sub-agents inside its own process tree) runs with full permissions inside a
box that conductor sets up and measures from outside (architecture-decision-record:external-confinement, the operator's decision, following
metaharness):
- CPU, memory and process limits per instance and per session;
- writes limited to the session's own places;
- usage attributed per instance and per session;
- disk limited per instance, once the host's filesystem supports project quotas;
- a session that cannot be boxed is not started (`strict`), or started and recorded as unboxed
  (`report`), as the instance's config says.

Later, through substrate, with the harness itself staying outside (docs/analysis/2026-10-09-substrate-benefits.md § 5): a build or test can run
as a substrate exec (`conductor gate run`), and a session's commands can run inside substrate through
a tool server conductor owns. Codex sessions are out of scope (`story:codex-controller-profile` has
its own sandbox).

## Acceptance

First part, with a test instance on this host, each checked by a test or a recorded run:
- a controller session runs in its own cgroup under the instance's slice, with the configured
  `MemoryMax`, `CPUWeight` and `TasksMax` (`systemctl --user show`), and still answers SendMessage
  and appears in `claude agents --json` (story `session-limits-systemd`);
- conductor's own session runs boxed the same way, and a worker sub-agent's processes sit in its
  controller's cgroup (story `session-limits-systemd`);
- a Bash write by a controller outside its checkout, trees, build cache and scratch fails with a
  filesystem error, and one inside succeeds (story `session-writes-bwrap`);
- the dashboard shows a live session's CPU time and memory (story `instance-usage-metrics`);
- with `confinement.mode: strict` and no provider available, `session start` refuses and no session
  runs (story `session-envelope-seam`);
- `conductor doctor` says whether the host can enforce project quotas (story `quota-support-report`).

The disk limit itself is `story:instance-storage-quota`, which waits on
`decision-blocker:project-quota-filesystem`.

Later part, each by its story's recorded run:
- `story:substrate-tool-spike`: a go or no-go for the tool route;
- `story:substrate-gate-provider`: a gate run as a substrate exec, with usage printed; waits on
  `upstream-blocker:substrate-exec-in-checkout`;
- `story:substrate-tool-server`: a controller whose `run` calls execute in substrate; waits on the
  spike's go.

The epic is done when the first part's six checks pass and each later story is implemented, or
archived on a no-go recorded by the spike.

## Findings this rests on

- `claude --bg` sessions are children of Claude Code's daemon (`claude daemon run`, parent pid 1),
  not of the command that starts them; wrapping the start command confines nothing (docs/analysis/2026-10-09-substrate-benefits.md § 2).
- A foreground `claude -p` runs inside a substrate exec with enforced memory and pids limits (docs/analysis/2026-10-09-substrate-sessions.md).
- Plain Linux gives the same kernel counters and limits through a delegated `systemd-run --user`
  scope or unit, and bubblewrap 0.12.0 is installed (docs/analysis/2026-10-09-substrate-benefits.md § 1, § 2).
- metaharness runs Claude Code unconfined and refuses substrate for it; nothing yet runs Claude
  Code's tools inside substrate, but the parts exist (docs/analysis/2026-10-09-substrate-benefits.md § 5).
