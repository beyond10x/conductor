---
format: aep.planning-md/3
id: review-result:w49-u7-adversary-2
kind: review-result
status: active
title: Wave 49 U7 status-current-snapshot, adversary pass 2
relations:
- reviews: story:status-current-snapshot
revision: 1
---
unit: story:status-current-snapshot, commit 61c4e3711 (impl/status-current-snapshot) plus one uncommitted test file
verdict: CONFIRMED
cases: executed 1611→1616, red 1
origin: introduced 1 / pre-existing 0 / undecided 1
wrote-outside-worktree: 3 logs in the unit's scratch adv2/, and the cases' state directories under the shared test target's tmp/
needs-coordinator: whether host load alone explains the status 2 s miss (F2): settling it needs a run on a quieter host or one that times the base

The correction mostly holds. No list I built answers differently from the old whole-feed fold. Two findings: a leftover copy of erased bodies (only on databases indexed by the unreleased 2f5d3607f build), and the status 2 s test failing even when run alone at load 52 to 67.

**1. `git --no-pager diff --stat`**: empty, because the only change is a new file. `git status --short`: `?? crates/conductor/tests/adv2_status_index.rs`. Test files only.

**2. Cases** in `crates/conductor/tests/adv2_status_index.rs`. I ran the file alone first, before the suite:
- `adv2_rows_of_the_earlier_build_are_read_at_their_version_and_replaced_on_the_next_write` (green): rows written by the 2f5d3607f projector, then G2 redacted. The list answers only G1 and `check()` reports G2 as Undecodable. After G1 is written again and indexed, its row holds only `first` and `version`.
- `adv2_a_redaction_leaves_no_copy_of_the_erased_body_in_rows_of_the_earlier_build` (**red**): with those same old rows, G2 redacted, then this build's catch-up and a list. The erased title is still in the index table. The red output from the lone run:
  ```
  FAIL [   0.850s] (3/5) conductor-cli::adv2_status_index adv2_a_redaction_leaves_no_copy_of_the_erased_body_in_rows_of_the_earlier_build
  panicked at crates/conductor/tests/adv2_status_index.rs:423:5:
  the erased title of G2 is still held by the index after the redaction and a catch-up of this build: [("conductor.direction.Goal/G2", Object {"data": Object {... "title": String("Erased on request")}, "first": Object {"at": String("1794268111867206663"), "seq": Number(2)}, "name": String("stored"), "redacted": Bool(false), "schema_version": Number(1), "version": Number(1)})]
  Summary [  12.835s] 5 tests run: 4 passed, 1 failed, 0 skipped
  ```
- `adv2_a_list_over_rows_ahead_of_the_index_position_answers_the_feed` (green): the rows are ahead of the `~position` row (asserted). That is what a list sees when a catch-up commits while `indexed()` pages. It covers an update, a delete, a re-creation and a stream stored after the position. The list matches the feed, on a fresh handle and inside `at_once` (twice).
- `adv2_a_version_the_log_no_longer_holds_hides_only_its_stream` (green): G2's newest event is removed with raw SQL. G2 is left out and reported, and G1 is still answered.
- `adv2_the_indexed_list_equals_the_whole_feed_fold_through_redactions_and_catch_ups` (green): 160 seeded steps of puts, deletes, redactions (newest and older) and catch-ups. The list matches a whole-feed fold that treats redaction the same way, and `check()` reports a redacted newest record exactly when there is one.

**3. Suite**, run after the cases existed: `conductor build run -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100.
```
FAIL [   2.206s] ( 160/1616) conductor-cli::adv2_status_index adv2_a_redaction_leaves_no_copy_of_the_erased_body_in_rows_of_the_earlier_build
FAIL [  31.739s] ( 513/1616) conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger
FAIL [1864.357s] (1616/1616) conductor-cli::status status_answers_within_2_s_on_an_instance_of_50k_tree_files_reading_the_newest_snapshot_only
  `conductor status --format json` took 9.062909202s (bound 2s)
Summary [1990.372s] 1616 tests run: 1613 passed (3 slow), 3 failed, 7 skipped
```
The `build_run` failure is the known one that also fails on the base. I reran the status case alone, `conductor build run -- cargo nextest run -p conductor-cli --test status status_answers_within_2_s`, exit 100:
```
`conductor status --full` took 3.47383338s (bound 2s)
Summary [ 270.673s] 1 test run: 0 passed, 1 failed, 18 skipped
```
Load average was 52 to 67 on 20 cores. In the suite run the case's process was stalled on disk writes, with the host's full I/O stall averaging about 38 % over 300 s.

**4. Findings**
- **F1** `crates/conductor/src/store/index.rs:9` (doc) and `:85` (the projector's upsert). Covers 61c4e3711.
  - What was measured: the red case above. Rows the 2f5d3607f build wrote keep a copy of each stream's newest body. This build never reads that copy, but only replaces the row when the stream is written again. So a redacted body stays in the database, which contradicts the module doc's "The index holds only what a redaction cannot change: no body".
  - What reaches it: nothing found. 2f5d3607f is only on `impl/status-current-snapshot` (`git branch -a --contains` lists nothing else), so it needs a state directory that a binary built from that commit indexed.
  - Verdict INFEASIBLE, origin introduced.
  - Fix, not applied: when `index()` meets a row with `data`, call `rebuild_projection` once, or rename the projection and drop the old table.
- **F2** `crates/conductor/tests/status.rs:1329`. Covers 61c4e3711.
  - What was measured: the acceptance test of 50k tree files and 14 snapshots fails alone (3.47 s) and in the suite (9.06 s).
  - What reaches it: this host under its normal multi-session load.
  - Cause undecided. Status lists only small entities (snapshots, dispatches, requests, controllers). The 7000 grants are never listed, so the correction adds only tens of per-stream reads. That points at host load rather than the change, but I measured no baseline.
  - The same case's total time went from 61–127 s before the correction to 187–311 s in the implementor's gates and 1864 s here.
  - Verdict CONFIRMED, origin undecided.

**5. Could not break**
- The exclusive `after_version` read at version−1 (matches the SQLite `read_stream`).
- Old rows read by version alone.
- Rows ahead of the position, and pages taken while a catch-up runs.
- A stream written between reading its row and reading its event.
- A missing version (the stream is left out and reported).
- Redaction of the newest or an older event through catch-ups.
- Prefix paging (it uses `starts_with`, not LIKE).
- Other tenants and the host ledger fall back to the feed.
- `open_grants` and `read()` are not affected by the correction.

**6. Paths written outside the worktree**
- `<scratch>/cases.log`
- `<scratch>/suite.log`
- `<scratch>/status-alone.log`
- `<shared target>/tmp/adv2_status_index/` (5 case directories)
- `<shared target>/tmp/status/sized/` (rewritten by the lone rerun of an existing case)

I took my own worktree lease  and have released it.

```findings
- file: crates/conductor/src/store/index.rs
  line: 9
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: rows the 2f5d3607f build wrote keep each stream's newest body, and this build replaces a row only on that stream's next write, so a later redaction leaves the erased body in conductor_streams, against the module doc's "no body" (red case adv2_status_index::adv2_a_redaction_leaves_no_copy_of_the_erased_body_in_rows_of_the_earlier_build); only a database indexed by a binary built from that branch-only commit reaches it
- file: crates/conductor/tests/status.rs
  line: 1329
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: the 50k-file, 14-snapshot status acceptance test misses its 2 s bound when run alone (3.47 s for --full at load 52-67) and in the suite (9.06 s), though status lists only small indexed entities, so host load rather than the per-stream reads is the likelier cause
```
