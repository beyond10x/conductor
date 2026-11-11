---
format: aep.planning-md/3
id: review-result:w50-u5-adversary-1
kind: review-result
status: active
title: Wave 50 U5 gc-idle-targets, adversary pass 1
relations:
- reviews: story:gc-idle-targets
revision: 1
---
Adversary pass 1 on `story:gc-idle-targets`, commit 64edec9a7 on base 4d39c3c5c.

Verdict: NEEDS-CHANGE. Five findings, all introduced.

Cases added to `crates/conductor/tests/gc.rs` (run alone first: `5 tests run: 3 passed, 2 failed`):

- red `a_directory_another_tool_tagged_is_not_cleaned_as_a_cargo_target`: a `.venv` in a managed tree carrying a `CACHEDIR.TAG`, aged 240 minutes, is removed as an idle cargo target.
- red `a_target_a_process_names_through_a_link_is_kept`: a target named through a symlink by `CARGO_TARGET_DIR` and one by a command-line path are both removed.
- green `a_program_running_from_a_target_keeps_it`
- green `a_write_in_place_three_levels_down_keeps_a_target`
- green `nothing_is_cleaned_when_free_disk_cannot_be_read`

Suite after the cases (`gc`, `reclaim`, `gc_trees`): `36 tests run: 34 passed, 2 failed, 0 skipped`.

Findings:

- F1 (blocker, `gc/targets.rs:197`): `is_cargo` accepts any directory holding a `CACHEDIR.TAG` file.
- F2 (warning, `:390`): only the target side is canonicalized, so a target a process names through a symlink is not held.
- F3 (warning, infeasible to test, `:316`): a process whose `/proc` links are unreadable or in another mount namespace counts as holding nothing.
- F4 (note, `:513`): `clean` checks and then removes without a lock.
- F5 (note, `:448`): no case holds the owner check or the second check in `clean`.

Attacked and not broken: a working directory in a subdirectory, an open file at any depth, a relative
`CARGO_TARGET_DIR`; a long test run (held by its exe) and a long link step (held by cargo's open lock
file); a symlink named `target` and a symlinked root are never walked; a target in a primary checkout
is skipped; `.keeper-hold` as file, directory or link; window boundaries, a future modification time;
equal times fall back to path order; the dry run deletes nothing; shared targets with a lock file are
skipped. Not tried: a mount inside a target, non-UTF-8 paths.

```findings
- file: crates/conductor/src/gc/targets.rs
  line: 197
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"is_cargo accepts any directory holding a CACHEDIR.TAG file, so a uv virtual environment, .pytest_cache, .mypy_cache or .ruff_cache in a tree is removed as an idle cargo target"'
- file: crates/conductor/src/gc/targets.rs
  line: 390
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: '"a target a process names through a symlink, in CARGO_TARGET_DIR or on its command line, is not held and is removed, because only the target side is canonicalized"'
- file: crates/conductor/src/gc/targets.rs
  line: 316
  category: judgement
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: '"a process whose /proc links are unreadable or whose paths are in another mount namespace counts as holding nothing, leaving only the write window and the owner check"'
- file: crates/conductor/src/gc/targets.rs
  line: 513
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"clean checks and then removes without a lock, so a cargo that starts in a tree target between the two loses files mid-build"'
- file: crates/conductor/src/gc/targets.rs
  line: 448
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"no case holds the owner check or the second check in clean, so deleting either leaves the suite green"'
```
