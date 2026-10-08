---
format: aep.planning-md/3
id: review-result:mi-design-round-2
kind: review-result
status: active
title: Design critic, multi-instance-host, round 2
relations:
- reviews: epic:multi-instance-host
- reviews: story:instance-usage-metrics
- reviews: story:instance-session-names
revision: 1
---
needs-revision

- story:instance-usage-metrics — its acceptance reads each session's own cgroup v2 files, but all ten live sessions I checked share `session-3.scope`, so a per-session or per-instance cgroup exists only once story:instance-cgroup-limits (or a substrate session) creates one, and no `depends_on` edge runs from it to story:instance-cgroup-limits. Add that edge, which conflicts with the epic leaving cgroup-limits out of its acceptance. Or name a source that exists today, such as summing `/proc` accounting over each session's process tree. — `.engineering/planning/story/instance-usage-metrics.md:30-32`, `.engineering/planning/story/instance-cgroup-limits.md:29-31`, `claude agents --json` with `/proc/<pid>/cgroup` (10 of 10 live sessions read `0::/user.slice/user-1000.slice/session-3.scope`)
- story:instance-session-names — its acceptance renames conductor to `<prefix>-conductor` while the guard allows a controller to message only the name `conductor`, so landing this story alone leaves controllers unable to reach conductor until story:cross-instance-message-guard lands. The recipient rule is half of the same rename. Either move the guard's recipient change into this story, or state that conductor keeps its address `conductor` until the guard story has landed. — `.engineering/planning/story/instance-session-names.md:38-39`, `crates/conductor/src/guard/rules.rs:82,723,729`

What I read: 12 artifacts, plus the 3 files I opened outside the set. I ran `aep plan artifact show` on each of the 11 set items and on `story:install-prerequisites`, then `relations`, `graph` and `validate` (valid, only the unrelated scope warnings on other stories). I also read `review-result:mi-design-round-1`, `spec/drafts/host.yaml`, `Taskfile.yml`, `guard/rules.rs` and `collect/sessions.rs`, and read `/proc/<pid>/cgroup` for the live sessions.

Round 1:
- Finding 1 (SessionName ownership) is fixed. `instance-session-names.md:33-37` now owns `SessionPrefix` and `SessionName` in `config.yaml`, and `host-grants.md:30-31` imports them.
- Finding 2 (missing edge) is fixed. `host-grants.md:9` has `depends_on: story:instance-session-names`.
- Finding 3 (storage-quota doing two things) is fixed. `story:quota-support-report` is split out and unblocked, and the `blocks` edge now covers enforcement only (`instance-storage-quota.md:24`).

Edges and acceptance reads:
- I walked all 41 edges in `aep plan artifact graph`, including the `reviews` edges and the `story:quota-support-report` to `story:install-prerequisites` edge, which leave the set.
- There is no cycle. The longest `depends_on` chain is 2 (cgroup-limits, then scoped-tasks, then session-names), so nothing serialises.
- I traced 8 acceptance reads to a producer inside the set. Edges record 7 of them. The unrecorded one is the first finding. The seam in the second finding is an ordering and breakage issue that the edge already records.

What I could not establish:
- Whether `story:instance-scoped-tasks` is two things in one item (session-scoped tasks plus a per-instance dashboard port). This is the same unease as round 1, and I still cannot name a seam.
- `spec/drafts/host.yaml:16-25` still declares `conductor.host.SessionPrefix` and `SessionName`, which contradicts the stories now that `config.yaml` owns them. No story's scope lists the draft file, so nobody is told to remove them.
- `instance-storage-quota` scope lists only `quota.rs` and `config.yaml`, so no call site applies the quota to trees. This is for the scope or acceptance critic.
- Out of my lane, for parallel-safety: `config.yaml` and `config.rs` are still edited by session-names, scoped-tasks, usage-metrics, storage-quota and cgroup-limits, and only some of those pairs carry an edge.

```findings
- file: .engineering/planning/story/instance-usage-metrics.md
  line: 31
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads each session's own cgroup v2 files, but all live sessions share one cgroup (session-3.scope) and a per-instance or per-session cgroup is created only by story:instance-cgroup-limits, with no depends_on edge from this story to it; add the edge (conflicting with the epic leaving cgroup-limits out of its acceptance) or name a source that exists today such as /proc accounting over each session's process tree"
- file: .engineering/planning/story/instance-session-names.md
  line: 38
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance renames conductor to <prefix>-conductor while the guard allows a controller to message only the name conductor (crates/conductor/src/guard/rules.rs:82,723,729), so this story alone leaves controllers unable to reach conductor until story:cross-instance-message-guard lands; move the guard's recipient change into this story or state that conductor keeps the address conductor until the guard story has landed"
```
