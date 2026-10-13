---
format: aep.planning-md/3
id: story:store-open-under-lock
kind: story
status: implemented
title: The store is opened under its lock, so a concurrent commit never reads as corruption
scope:
- confidence: cited
  path: crates/conductor/src/store.rs
- confidence: cited
  path: crates/conductor/tests/decision.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T02:40:52Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T02:40:52Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T02:50:09Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

CI run 38336492098 (`task check` on `e7dd4f7`) failed `six_concurrent_records_are_all_recorded`:
the sixth of six concurrent decision records was refused with "tree store is corrupt: a group whose
event is missing". Conductor opens the tree store without its lock (`crates/conductor/src/store.rs`
`Store::at` and `reopened`), and eventlog's lock-free open lists event files and group files in two
passes; a commit landing between them reads as corruption (eventlog-tree `history::load`). Reported
by conductor, 2026-10-13. The eventlog fix is dispatched to its repository; this story is
conductor's side: in production a record command fails when a controller or the watch writes at the
same moment.

## Acceptance

- Opening the store holds the tree store's lock for the whole open: a test takes
  `state/tree/.lock`, starts `conductor decision record-conductor-decision`, sees it still running
  after a short wait, releases the lock and sees it record.

## Scope

`crates/conductor/src/store.rs`, `crates/conductor/tests/decision.rs`.
