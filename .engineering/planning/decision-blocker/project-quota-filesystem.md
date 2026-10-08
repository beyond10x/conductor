---
format: aep.planning-md/3
id: decision-blocker:project-quota-filesystem
kind: decision-blocker
status: open
title: The host has no filesystem with project quotas
relations:
- blocks: story:instance-storage-quota
revision: 1
---
## Question

Where do project quotas come from on this host? `/` is ext4 mounted `rw,noatime`, without `prjquota` (`findmnt -no FSTYPE,OPTIONS /`). Options: (A) enable the ext4 `project` feature and remount `/` with `prjquota` (root, a reboot or remount, affects the whole system); (B) a separate volume (XFS with `pquota`, or ext4 with `prjquota`) mounted for `~/.local/state/worktree/trees` and build directories; (C) no quotas: the host ledger and the watch police storage. Operator decision (root access, a change to the machine).
