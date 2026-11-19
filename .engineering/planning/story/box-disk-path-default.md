---
format: aep.planning-md/3
id: story:box-disk-path-default
kind: story
status: implemented
title: A boxed instance's disk thresholds read the filesystem its builds write
summary: Absent thresholds.disk_path, a box measures free disk on its checkouts root, not on the box's tmpfs root
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#201
relations:
- decomposes: epic:architecture-next
- informed_by: epic:disk-protocol
scope:
- confidence: cited
  path: crates/conductor/src/build.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: crates/conductor/tests/config.rs
- confidence: inferred
  path: crates/conductor/tests/doctor.rs
- confidence: inferred
  path: crates/conductor/tests/free_space.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T14:13:12Z", actor: "human:unknown", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-18T14:13:13Z", actor: "human:unknown", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-19T17:02:27Z", actor: "human:unknown", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #201 (agent box friction F16, F25). In a box `/` and the home directory are a tmpfs of about
15 GiB. Snapshots read `disk_free 16453287936` against `disk_admit` 20G; the first build was refused
for disk and escalated class H while the checkouts root it builds in had 131G free on the host disk.
The operator set `thresholds.disk_path` to the checkouts root.

Read on the wave 56 branch:

- `thresholds.disk_path` defaults to `/` (`crates/conductor/src/config.rs:401`, `:2202`, read at
  `:5699-5702`); builds read it (`crates/conductor/src/build.rs:388`).
- The snapshot ignores it: `start-snapshot` reads `statvfs` on the constant `DISK = "/"`
  (`crates/conductor/src/snapshot.rs:123`, `:996`), and the watch's disk probe reads `/` too
  (`crates/conductor/src/watch.rs:449`). So even with `disk_path` set, the snapshot's `disk_free` and
  the watch's `disk low` lines measure the box's tmpfs.

## Outcome

- One resolved disk path per instance: `thresholds.disk_path` when set; else, for an instance whose
  placement opens a box, its checkouts root; else `/`. The build gate, the snapshot's `disk_free`
  and the watch's disk probe all read it, and the watch's lines name it instead of `/`.
- `conductor doctor` names a `disk_path` that is a tmpfs inside the box.
- The `thresholds.disk_path` documentation in `spec/domains/config.yaml` states the boxed default.

Decision (class C, controller, 2026-11-18): the snapshot and the watch follow `disk_path` for every
instance, boxed or not, rather than only in a box, so one key means one filesystem. Undone by pinning
them to `/` again.

## Acceptance (tests that fail on the current code)

- `tests/config.rs` `a_boxed_instance_with_no_disk_path_reads_its_checkouts_root`.
- `tests/free_space.rs` `the_snapshot_reads_free_disk_on_the_instance_s_disk_path`.
- `tests/doctor.rs` `doctor_names_a_disk_path_on_the_box_s_tmpfs`.
