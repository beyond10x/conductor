---
format: aep.planning-md/3
id: review-result:mi-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, multi-instance-host, round 2
relations:
- reviews: epic:multi-instance-host
- reviews: story:instance-storage-quota
revision: 1
---
needs-revision

story:instance-storage-quota — the only observable check, "a test on a loop-mounted XFS image with `pquota` writes past the limit and observes `EDQUOT`", needs root to loop-mount and name no way the test gets it, and it exercises XFS even though option A is ext4 `prjquota`, so the acceptance can't be checked as written on this host — .engineering/planning/story/instance-storage-quota.md:28

**What I read:** all 12 ids, each with `aep plan artifact show` or `cat -n` on its store file: epic:multi-instance-host, specification:multi-instance-host, instance-session-names, instance-scoped-tasks, host-grants, cross-instance-message-guard, substrate-session-spike, instance-usage-metrics, instance-storage-quota, quota-support-report, instance-cgroup-limits and decision-blocker:project-quota-filesystem. I also read review-result:mi-acceptance-round-1 and `aep plan artifact kinds`. I checked the tree with `git grep build_size` and checked tool and privilege availability with `which setquota mkfs.xfs xfs_quota losetup`, `id -u` and `sudo -n true`.

**Round-1 findings:**
- Fixed (4 of 5): instance-session-names (lines 40-42), host-grants (37-40), instance-usage-metrics (30-33) and epic:multi-instance-host (a bounded Acceptance section, lines 14-24).
- Partly fixed (1 of 5): instance-storage-quota. The check is now a concrete `EDQUOT` test, but it can't be run as written (the finding above).

**What I could not establish:**
- Whether the gate or CI runs as root, or can use a user namespace, to loop-mount. On this host `sudo -n` asks for a password, `id -u` is 1000, and `setquota` isn't installed. `mkfs.xfs`, `xfs_quota` and `losetup` are present.
- Whether `setquota -P` works on an XFS image. That is a design question, not mine.
- Out of my lane, so it set no verdict:
  - instance-scoped-tasks lists only two tests, restart and the second dashboard's port. The "refuses when a name resolves to more than one session" and "pid file per instance" clauses have none. Both are observable, so I didn't flag them.
  - epic:multi-instance-host promises a refusal "beyond the floor or slot count", but host-grants test (1) covers only the floor. The slot-count refusal is observable but has no named test.
  - Multi-bullet acceptance lists follow the store's precedent, as in round 1.
  - Scope and design questions were not judged. These are the open decision-blocker and whether instance-session-names should relate to substrate-session-spike.

```findings
- file: .engineering/planning/story/instance-storage-quota.md
  line: 28
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the only observable check, 'a test on a loop-mounted XFS image with `pquota` writes past the limit and observes `EDQUOT`', needs root to loop-mount and names no way the test gets it (sudo needs a password and setquota is absent on this host), and it exercises XFS while option A is ext4 prjquota, so the acceptance cannot be checked as written"
```
