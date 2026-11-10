---
format: aep.planning-md/3
id: story:shipped-newest-complete-shared
kind: story
status: draft
title: shipped reads the newest complete snapshot through the shared helper
tags:
- from-finding
relations:
- informed_by: review-result:w49-kpi-scope-active-repositories-adversary-1
revision: 1
---
## Why

Wave 49 (story:kpi-scope-active-repositories, review-result:w49-kpi-scope-active-repositories-adversary-1 and the implementor's correction report) made `metric compute` and `repository activity` pick the later-stored of two complete snapshots with equal starts through `repository::newest_complete`. `crates/conductor/src/shipped.rs` lines 91-100 still keep the first stored (a strict `>`), so on equal starts it reads a different snapshot than the rest. Nothing found produces equal starts; the disagreement predates the wave.

## Acceptance

- `shipped.rs` picks its snapshot through the shared helper, and a test with two complete snapshots of equal start shows it reads the later stored.
