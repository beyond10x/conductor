---
format: aep.planning-md/3
id: decision-blocker:project-quota-filesystem
kind: decision-blocker
status: open
title: The host has no filesystem with project quotas
relations:
- blocks: story:instance-storage-quota
revision: 2
---
## Question

Where do project quotas come from on this host? `/` is ext4 mounted `rw,noatime`, without `prjquota` (`findmnt -no FSTYPE,OPTIONS /`). Options: (A) enable the ext4 `project` feature and remount `/` with `prjquota` (root, a reboot or remount, affects the whole system); (B) a separate volume (XFS with `pquota`, or ext4 with `prjquota`) mounted for `~/.local/state/worktree/trees` and build directories; (C) no quotas: the host ledger and the watch police storage. Operator decision (root access, a change to the machine).

## What clears it

The operator's choice, recorded as an `approval-record` related `decides` to this blocker. A or B:
`conductor doctor` (story `quota-support-report`) prints `project quotas: yes` for the trees root,
then this blocker moves to `cleared` and `story:instance-storage-quota` proceeds. C: this blocker
moves to `cleared` and `story:instance-storage-quota` is archived with the record cited.
