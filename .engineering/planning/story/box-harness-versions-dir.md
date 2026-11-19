---
format: aep.planning-md/3
id: story:box-harness-versions-dir
kind: story
status: implemented
title: A box reads the harness versions directory, so a host-side update still starts
summary: Mount the link target's parent; a harness start error is a decider fault
refs:
- provider: github
  reference: beyond10x/conductor#192
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/decide/handle.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: cited
  path: crates/conductor/tests/placement.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-18T08:24:08Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-18T08:24:08Z", actor: "human:unknown", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-19T23:50:02Z", actor: "human:unknown", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #192. A box reads the harness link's target as one file (`placement.rs:1636`, `:1651-1656`, row
`:1491`); after Claude Code updates itself on the host the link points at a new version file the box
cannot read, `claude -p` fails ENOENT (`decide/model.rs:135`) and `decide/handle.rs:1315` reports it
as "decider gave no answer".

## Outcome

- When the harness on `PATH` is a link, the box reads the target's parent directory (the versions
  directory) instead of one file, within `READ_ONLY_ROOTS_BOUND`.
- A decider whose harness cannot start is reported as a decider fault naming the start error, not "no answer".

## Acceptance (fail on main)

- `tests/placement.rs` `a_box_reads_the_harness_versions_directory_so_a_host_update_resolves`.
- a test in `decide/handle.rs` or `tests/` that a start error is named as such.
