---
format: aep.planning-md/3
id: review-result:w50-u5-adversary-2
kind: review-result
status: active
title: Wave 50 U5 gc-idle-targets, adversary pass 2
relations:
- reviews: story:gc-idle-targets
revision: 1
---
Adversary pass 2 on `story:gc-idle-targets`, commit 538478387 (the correction) on base 4d39c3c5c.

Verdict: NEEDS-CHANGE. Four findings, all introduced by the correction.

Cases added at the end of `crates/conductor/tests/gc.rs` (run alone first: `4 tests run: 2 passed, 2 failed`):

- red `a_build_that_takes_cargos_lock_while_a_target_is_removed_loses_nothing_after_it`
- red `a_cargo_target_below_a_directory_another_tool_tagged_is_still_listed`
- green `a_directory_another_tool_tagged_is_no_target_even_with_cargos_layout`
- green `a_cross_compile_target_is_found_by_its_triples_lock_and_kept_while_it_is_held`
- ignored measurement, red alone: `the_dry_run_is_not_slowed_by_a_process_with_long_command_line_words`
  (16.8 s beside one process with ten long words, 0.34 s without)

Suite after the cases (gc, reclaim, gc_trees): `43 tests run: 41 passed, 2 failed, 1 skipped`.

Findings (all in `crates/conductor/src/gc/targets.rs`):

- G1 (warning, :666): the removal unlinks the `.cargo-lock` files gc holds, so a cargo that starts
  mid-removal creates the lock anew and builds into a target gc keeps deleting.
- G2 (note, safe direction, :313): a cargo target below another tool's tagged directory is never listed.
- G3 (note, :277): resolving a command-line word once per component makes the dry run slow beside a
  process with very long words.
- G4 (note, :660, :260): no case goes red when `lock()` is dropped from `clean` or when the
  `.rustc_info.json` alternative is dropped.

Attacked and not broken: a running build holds `.cargo-lock` shared, so the exclusive try fails;
a custom profile, a triple-only layout, `cargo doc` only and a missing `.rustc_info.json` all leave
a profile lock; a symlink or FIFO `.cargo-lock` is neither opened nor followed; locks are dropped
on every error path; skipping gc's own pid hides no holder; a symlink loop and a deleted cwd
terminate; the edited fixtures only gained cargo's layout.

```findings
- file: crates/conductor/src/gc/targets.rs
  line: 666
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"the removal unlinks the .cargo-lock files gc holds, so a cargo that starts mid-removal creates the lock anew, takes it at once and builds into a target gc keeps deleting; gc then fails with Directory not empty"'
- file: crates/conductor/src/gc/targets.rs
  line: 313
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: '"the walk stops at any directory holding a CACHEDIR.TAG, so a cargo target below another tool''s tagged directory is never listed or cleaned; no workflow was found that builds one there"'
- file: crates/conductor/src/gc/targets.rs
  line: 277
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: '"resolved_far canonicalizes a command-line word once per component, so one process with ten 120 kB words of 60000 components takes the dry run from 0.34 s to 16.8 s, once per scan and per cleaned target"'
- file: crates/conductor/src/gc/targets.rs
  line: 660
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"dropping lock() from clean, or the .rustc_info.json alternative in is_cargo at line 260, leaves the suite green: a lock holder also has the file open and every fixture carries both shapes"'
```
