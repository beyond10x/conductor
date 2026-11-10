---
format: aep.planning-md/3
id: review-result:w49-status-current-snapshot-adversary-1
kind: review-result
status: active
title: Wave 49 U7 adversary, pass 1
relations:
- reviews: story:status-current-snapshot
revision: 1
---
unit: story:status-current-snapshot, commit 2f5d3607f (impl/status-current-snapshot) plus one uncommitted test file
verdict: CONFIRMED
cases: executed 1609→1611, red 1
origin: introduced 1 / pre-existing 0 / undecided 1
wrote-outside-worktree: 4 paths: two logs in the unit's scratch adv/, and the cases' state directories under the shared test target's tmp/
needs-coordinator: none

Cases in the new file crates/conductor/tests/adv_status_index.rs:
- adv_a_record_redacted_after_the_index_caught_up_is_not_answered_from_the_index (red): goals
  G1 and G2, index caught up by retain(0), then G2 v1 redacted in the database; the Goals list
  answers ("G2", "Draft", "Erased on request") and check() is Ok; the store before this change
  left G2 out and reported it unreadable (as adv_store::adv_one_record_the_store_cannot_read_hides_only_itself).
- adv_the_indexed_list_equals_the_whole_feed_fold_through_puts_deletes_and_catch_ups (green):
  120 seeded steps of puts, deletes, re-creations and catch-ups; the Goals list equals a fold of
  the whole feed after every step, on a fresh handle and on one held inside at_once.

Suite: 1611 run, 1609 passed, 2 failed (the red case and build_run, which fails on the base too).

F1 store/index.rs:79-80: the index copies each stream's newest body and redacted flag at
catch-up; a later eventlog redaction adds no feed event, so the index row never changes.
Reached on every instance with a completed snapshot (retain builds the index each cycle); the
redaction comes from outside conductor. Introduced: the base has no index and folds the feed.
F2 store/host.rs:139: ledger.open's feed position is not tied to its database; nothing found
that reaches it.

Could not break: list order and content against the whole-feed fold, at_once handles, equal
positions, a crash between index rows and the position row (one transaction), duplicate
versions, deleted streams and snapshots, observations_of without the snapshot filter, the usage
forecast's newest two readings, racing ledger.open writers, grant keys.

```findings
- file: crates/conductor/src/store/index.rs
  line: 80
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the stream index keeps a copy of each stream's newest body that a later eventlog redaction never reaches, so every list answers a redacted record's erased body as readable and check() reports nothing (red case adv_status_index::adv_a_record_redacted_after_the_index_caught_up_is_not_answered_from_the_index)
- file: crates/conductor/src/store/host.rs
  line: 139
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: ledger.open's feed position is trusted without tying it to the database it came from, so a recreated store.sqlite beside a surviving file would skip grants up to the old position and keep the file's grants open; nothing found that reaches it
```
