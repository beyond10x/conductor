---
format: aep.planning-md/3
id: story:append-cost-test-under-load
kind: story
status: draft
title: The append-cost test does not fail on a loaded host
revision: 1
---
## Why

`crates/conductor/tests/store_tree.rs` `appends_at_5000_events_cost_less_than_three_times_appends_at_500`
compares two wall-clock timings. On a loaded host it fails while the code is unchanged: in one day
it failed three full gates (load average 30: 11.1 s vs 3.4 s; 19.6: 10.9 s vs 2.99 s; 37.9:
6.92 s vs 1.93 s) and passed each time when run alone. A red gate on a busy machine costs a full
rerun.

## Acceptance

- The cost check does not depend on wall-clock time under concurrent load: it counts work (for
  example records or bytes read per append), or runs as an `#[ignore]`d measurement that a separate
  task runs alone.
- `task check` on a host at load average 30 passes when the code does.

## Scope

`crates/conductor/tests/store_tree.rs` (cited), possibly `Taskfile.yml` (inferred).
