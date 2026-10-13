# Wave 14

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Approved up front (rules item 99).
One managed tree, `conductor-wave-14`; one unit, written without a local build and gated by CI.

## Unit

| unit | story | commit |
|---|---|---|
| U1 | store-open-under-lock | `b421b5c` |

## Why

Conductor reported (2026-10-13) a flaky `six_concurrent_records_are_all_recorded`: a decision record
was refused with "tree store is corrupt: a group whose event is missing" while another process
committed. The diagnosis: conductor opened the tree store without its lock, and eventlog's lock-free
open lists event files and group files in two passes, so a commit landing between them reads as a
group whose events are missing. Conductor now holds the tree's lock (the file eventlog locks for
commits) through every open; the root fix in eventlog (snapshot the group list before the events)
is dispatched to that repository, and conductor repins eventlog when it is released.

## Outcome

`task check` and the Gates job green in CI
(https://github.com/beyond10x/conductor/actions/runs/38338412828), the new deterministic lock test
included. Story moved to `implemented` on recorded evidence.
