---
format: aep.planning-md/3
id: story:box-disk-path-default
kind: story
status: draft
title: A boxed instance's disk thresholds read the filesystem its builds write
summary: Absent thresholds.disk_path, a box measures free disk on its checkouts root, not on the box's tmpfs root
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: epic:disk-protocol
scope:
- confidence: cited
  path: spec/domains/config.yaml
- confidence: inferred
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/build.rs
- confidence: inferred
  path: crates/conductor/src/doctor.rs
revision: 1
---
## Why

Agentbox friction logs F16 and F25 (2026-11-15). In a substrate box `/` and the box's home are
a tmpfs, 15 GiB on this host. `thresholds.disk_path` defaults to `/`
(`website/docs/reference/config.md:109`, `conductor.example.yaml:42`). So every snapshot
reported `disk_free 16453287936` against `disk_admit` 20G, and the first build in the box was
refused for disk and escalated as class H (request `disk-01a1df49-…`). The checkouts root it
builds in had 131G free. The operator's fix was `thresholds.disk_path` = the checkouts root.
That value cannot be read off the host, and nothing tells an operator to set it.

## Outcome

- For an instance at a box placement whose config sets no `disk_path`, the thresholds read the
  filesystem of its checkouts root.
- `conductor doctor` reports a `disk_path` that is a tmpfs inside the box.

## Acceptance

- A boxed instance with no `thresholds.disk_path` reads its checkouts root's free space. A
  config whose `disk_path` resolves to the box's tmpfs gets a doctor row that names the key. A
  config test and a doctor test hold both.
