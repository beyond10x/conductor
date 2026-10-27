---
format: aep.planning-md/3
id: story:slice-remove
kind: story
status: implemented
title: An instance's empty systemd slice can be removed
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T15:45:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T15:45:13Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-16T15:45:13Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: slice-remove

## Outcome

The operator can list each instance's systemd slice and remove an empty one left behind by an instance that is gone.

## Context

Live runs of story:session-limits-systemd (commit 82508c0c; docs/waves/wave-37.md U3) leave `conductor_<prefix>.slice` loaded after the instance's last session stops. No `conductor` command removes it, so a removed or renamed instance keeps its slice.

## Acceptance

- `conductor host slices` lists each instance's slice with its unit count.
- `conductor host slices --gc` stops each empty slice of an instance absent from the config, and never one that still holds a unit. Tested with a fake provider.

## Out of Scope

Removing a slice that still holds a unit; slices of other tools.

## Ambiguities

None.
