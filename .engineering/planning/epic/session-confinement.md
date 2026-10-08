---
format: aep.planning-md/3
id: epic:session-confinement
kind: epic
status: draft
title: Harness sessions run in a box conductor controls
relations:
- implements: architecture-decision-record:external-confinement
revision: 2
---
## Outcome

Every Claude Code session conductor starts (conductor, conductor-dev, controllers and the workers
they spawn) runs with full permissions inside a box that conductor sets up and measures from outside
(architecture-decision-record:external-confinement, the operator's decision, following metaharness):
- CPU, memory and process limits per instance and per session;
- writes limited to the session's own places;
- usage attributed per instance and per session;
- disk limited per instance, once the host's filesystem supports project quotas;
- a session that cannot be boxed is not started (`strict`), or started and reported as unboxed
  (`report`), as the instance's config says.

Later, through substrate: builds and tests run as substrate execs, and a session's commands run
inside substrate through a tool server conductor owns, while the harness itself stays outside (docs/analysis/2026-10-09-substrate-benefits.md
§ 5). Codex sessions are out of scope here (`story:codex-controller-profile` has its own sandbox).

## Acceptance

With a test instance on this host, each checked by a test or a recorded run:
- a controller session runs in its own cgroup under the instance's slice, with the configured
  `MemoryMax`, `CPUWeight` and `TasksMax` (`systemctl --user show`), and still answers SendMessage
  and appears in `claude agents --json` (story `session-limits-systemd`);
- a Bash write by that session outside its checkout, trees, build cache and scratch fails with a
  filesystem error, and one inside succeeds (story `session-writes-bwrap`);
- the dashboard shows that session's CPU time and memory (story `instance-usage-metrics`);
- with `confinement.mode: strict` and no provider available, the start refuses and no session runs
  (story `session-envelope-seam`);
- `conductor doctor` says whether the host can enforce project quotas (story `quota-support-report`);
  the quota itself waits on `decision-blocker:project-quota-filesystem`.

The substrate items (`substrate-tool-spike`, `substrate-client`, `substrate-gate-provider`,
`substrate-tool-server`) close the "later" line; two of them wait on upstream blockers, so this
epic can complete its first part without them.

## Findings this rests on

- `claude --bg` sessions are children of Claude Code's daemon (`claude daemon run`, parent pid 1),
  not of the command that starts them; wrapping the start command confines nothing (docs/analysis/2026-10-09-substrate-benefits.md § 2).
- A foreground `claude -p` runs inside a substrate exec with enforced memory and pids limits (docs/analysis/2026-10-09-substrate-sessions.md).
- Plain Linux gives the same kernel counters and limits through a delegated `systemd-run --user`
  scope or unit, and bubblewrap 0.12.0 is installed (docs/analysis/2026-10-09-substrate-benefits.md § 1, § 2).
- metaharness runs Claude Code unconfined and refuses substrate for it; nothing yet runs Claude
  Code's tools inside substrate, but the parts exist (docs/analysis/2026-10-09-substrate-benefits.md § 5).
