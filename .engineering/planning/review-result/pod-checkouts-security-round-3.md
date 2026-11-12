---
format: aep.planning-md/3
id: review-result:pod-checkouts-security-round-3
kind: review-result
status: active
title: Fix of round 2, security review round 3
relations:
- reviews: story:pod-source-checkouts
- reviews: story:instance-secret-state
revision: 1
---
needs-change

unit: security review round 3 of 8061c254a ("Stop a snapshot's commands with it…") in `k8s-checkouts-fix`, plus my one uncommitted case
verdict: one note, no blocker: every round 2 finding is closed, and my new red case shows a small regression that I could not show anything in a pod reaching
cases: executed 74→75, red 1 (74 is the same six test files without my new case, counted from the run below)
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 6 paths (listed below)
needs-coordinator: none

1. **Worktree changes:** `git diff --stat` shows `crates/conductor/tests/sec_pod_checkouts.rs | 39 +`, one appended case. No implementation file was touched.

2. **Case added:** `a_helper_left_on_a_timed_out_clones_pipes_does_not_outlive_the_snapshot`, **red** when run alone: "a process the clone left on its pipes (process 2818709) still ran after the snapshot ended: exit Some(0) …".

3. **Suite run:** `zsh …/sec3/gate.sh`. Clippy exit 0. nextest on `sec_pod_checkouts`, `sec_instance_secrets`, `pod_checkouts`, `instance_secrets`, `isolated_home` and `daemon_pods`: `75 tests run: 74 passed, 1 failed`; the one failure is my new case; nextest exit 100.

**Claims**
- **A. Holds.** My round 1 and round 2 cases all pass, including the three that were red. Compared with my scratch copies, the committed files differ only by the `use std::io::Read as _;` line, which was mine. Nothing was removed; the implementor added five cases.
  - F1: `in_group` is set at `snapshot.rs:986`.
  - F2: the trust walk now checks coverage (`trust.rs:356`).
  - F3: `withheld` now holds every `env` entry's secret name.
  - F4: the paths moved from the error bodies to the log line.
  - F5: `pods.rs:409`.
- **B. Mostly holds.**
  - Only `snapshot start-snapshot` sets `IN_GROUP`; its only caller is the CLI dispatch (`cli.rs:266`).
  - Inside a snapshot nothing calls `killpg`; a timeout calls `stop_tree` (`collect/mod.rs:736`) or `child.kill()`.
  - A snapshot started by hand on a workstation never signals its own group, so the user's shell and terminal group are safe. Ctrl-C now also reaches its git, gh and glab.
  - Pid reuse: the command's own pid is not yet reaped, so it is safe. Each descendant pid is read after its parent got `SIGSTOP`, so the parent cannot reap it in between. That leaves a window of a few microseconds in which a parent not yet stopped reaps a child, and the pid would have to wrap around in that time. If it ever did, it would hit only a process of the same user in the pod (the runtime or a session).
  - Not holding (my red case): a process that left the command's tree survives the snapshot when it ends normally. Two paths reach this: the reaped path (`collect/mod.rs:679`, only `child.kill()`), and a helper that was re-parented before the bound. Before this commit the command's own group was stopped, and the process with it. I found no git, gh or glab helper in the pod that does this (the commit's own "Not shown here").
- **C. Holds (same reach as the guard).**
  - A `local` source or a forge source without `include` on the same root makes `LeftOut.include` `None`, so every git directory under the root is trusted. A mapping can write that.
  - It widens nothing beyond the base: such a source already makes every checkout a workspace member (`instance_checkouts`), so trust would follow anyway.
  - The implementor's cases `a_start_trusts_a_checkout_under_a_local_source` and `…_without_include` cover it.
- **D. Holds.**
  - Exact names are filtered, including under `globals` (`keyed` plus `withheld`).
  - A reference with other letter case passes `config::secret_name` (`config.rs:2216`), but the store's name parse refuses capitals, so it reads `set: false` and has no time.
  - A `secret:` inside a longer string is no reference.
  - The bodies hold no path. The log line holds paths and the io error, never a value. The caller's `{name}` reaches that line only on a `500` or `503`.
- **E. Holds.** `pods.rs:409` matches any `env` entry by name, a `secret` entry included. A mapping cannot add pod environment.
- **F. Holds.** A refused project is dropped without a report only where no `include` entry covers it (`gitlab.rs:642`), so it was never included. An included one is still reported, and a covering entry with no other answer still raises "no repository answers".

**Findings**
- **F1** (note, INFEASIBLE, introduced): `collect/mod.rs:679`. This is my red case. INFEASIBLE because nothing in a pod was shown to reach it. Suggested fix: on that path, stop the processes still holding the command's pipes, or run each command in a sub-group that the snapshot also stops.

**Written outside the worktree** (all under `<the review scratch directory>/`):
- `expected_pod.rs`, `sec_pod_checkouts.committed.rs`, `append.rs`, `gate.sh`, `gate.log`, `build.log`
- The red case keeps its directory under the shared target: `…/target/conductor-362621f27d1a30dd/tmp/sec_pod_checkouts/helper-on-pipes`

```findings
[
{"file":"crates/conductor/src/collect/mod.rs","line":679,"category":"concurrency","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"inside a snapshot a timed-out command that already exited is only killed by pid, so a process it left on its pipes or re-parented outlives a snapshot that ends normally, where the command's own process group used to stop it"}
]
```
