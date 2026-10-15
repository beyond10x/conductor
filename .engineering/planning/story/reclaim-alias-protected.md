---
format: aep.planning-md/3
id: story:reclaim-alias-protected
kind: story
status: draft
title: Reclaim never deletes a protected path reached through a symlink
revision: 1
---
# Story: reclaim-alias-protected

## Outcome

`conductor host reclaim` cannot delete the instance's records, state or any other protected path, whatever symlink leads into it.

## Context

This is P1 of the operator-requested external review of 0.14.0 (0e7ce756), at crates/conductor/src/gc/reclaim.rs:483. In the probe `reclaim_preserves_own_records_reached_through_a_scratch_root_symlink`, reclaim exited 0 after deleting the configured records' NORTHSTAR.md through a symlinked cache root.

The live host was checked on 2026-10-15: no scratch root is a symlink, and no entry directly under one is a symlink.

## Acceptance

- Before deleting, reclaim canonicalizes both every candidate path and every protected path. It refuses a candidate whose canonical path is a protected path, lies under one, or contains one.
- A scratch root that is itself a symlink is skipped and named on standard error.
- A symlink entry is removed as a link, never followed.
- The review's probe is in the suite and passes, plus one case per form: a root symlink, an entry symlink, and a nested symlink.

## Out of Scope

Changing which roots reclaim scans.

## Ambiguities

None.
