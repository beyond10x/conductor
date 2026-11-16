---
format: aep.planning-md/3
id: story:box-config-change-reaches-runtime
kind: story
status: draft
title: A config change reaches a boxed runtime without a restart, or says it waits for one
summary: The box reads the config file as it is on the host now, not the inode it was bound to at start; where it cannot, config put says the change waits for the box's next start
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: cited
  path: crates/conductor/src/config/put.rs
- confidence: inferred
  path: crates/conductor/src/daemon/reconcile.rs
revision: 1
---
## Why

Agentbox friction log F29 (2026-11-15). `thresholds.disk_path` was changed at 13:00, and builds
in the box were still refused at 13:09 under the old value. Only a runtime restart at 13:11
applied it, and that restart stopped every controller.

Read at v1.13.0 (mechanism likely, not verified on the box):

- The box reads the config as one read-only file (`placement.rs:1635`, the `config` row of the
  mounts table).
- `conductor config put` replaces the file by rename (`crates/conductor/src/config/put.rs:327`,
  `:342`), as most editors do.
- A file bind keeps the inode it was bound to, so the box reads the old file until it is
  opened again.

## Outcome

Either the box reads the current file, by binding the config's directory or by the daemon
handing a changed config to the runtime over the dial-in connection, or every config write
for a boxed instance says that the change reaches it at the box's next start. The restart
that applies it then keeps controller sessions resumable.

## Acceptance

- After `conductor config put` changes a threshold of a boxed instance, the runtime reads the
  new value within one tick, or the put's output names the box restart the change waits for.
  A placement test holds whichever is built.
