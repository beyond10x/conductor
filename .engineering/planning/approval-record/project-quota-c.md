---
format: aep.planning-md/3
id: approval-record:project-quota-c
kind: approval-record
status: draft
title: The operator chose no project quotas (C)
relations:
- decides: decision-blocker:project-quota-filesystem
revision: 1
---
## The question

Where project quotas come from on this host (`decision-blocker:project-quota-filesystem`): (A) the
ext4 `project` feature and a remount of `/` with `prjquota`; (B) a separate volume for the worktrees
and build directories; (C) no quotas, the build-slot and cleanup rules police disk.

## The decision

The operator, 2026-10-11, to conductor-dev, shown the options with C recommended: "all as
recommended".

## What follows

The blocker is cleared and `story:instance-storage-quota` is archived, citing this record.
`story:quota-support-report` stays: `conductor doctor` still says whether a host could enforce
project quotas.
