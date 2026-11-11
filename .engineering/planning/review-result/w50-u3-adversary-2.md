---
format: aep.planning-md/3
id: review-result:w50-u3-adversary-2
kind: review-result
status: active
title: Wave 50 U3 box-crash-loop-reads-failed, adversary pass 2
relations:
- reviews: story:box-crash-loop-reads-failed
revision: 1
---
```
unit: U3 wave 50, story:box-crash-loop-reads-failed, correction commit 8af6916fe (on 5bb941997, 5c4716794), worktree w50-u3
verdict: CONFIRMED
cases: executed 138→148, red 5
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 6 paths, all in the assigned scratch directory
needs-coordinator: none
```

The correction holds on the two blockers (F1/F2) and on F3, F6, F7 and F8 as asked. Five new cases are red: two warnings and three notes, none a blocker. `executed <before>` is the suite run of part 3 minus my 10 cases; I made no run with them deselected. The suite here adds `--test instance_upgrade`, which pass 1 did not run.

## 1. `git --no-pager diff --stat`

```
 crates/conductor/tests/instance_upgrade.rs |  48 ++++
 crates/conductor/tests/placement.rs        | 431 +++++++++++++++++++++++++++++
 2 files changed, 479 insertions(+)
```

Both are test files, appended `mod adversary2` blocks, uncommitted. No implementation file and no earlier case was touched.

- **One case outside the brief's file:** the upgrade case is in `tests/instance_upgrade.rs`, because the upgrade fixtures live there.
- **Not formatted:** I did not run `cargo fmt` on the new blocks.
- **Hook block:** one of my commands used a heredoc and was blocked by the guard hook before it ran; nothing was written.
- **Lease:** I took no worktree session lease.

## 2. Cases added

Red output is from the runs of the cases alone, before the suite.

**R1 `adversary2::a_runtime_that_stops_59_s_after_its_start_does_not_end_the_row`: red.** Ticks every 2 s; the runtime stops at 59 s and is found at the 60 s look.
```
panicked at crates/conductor/tests/placement.rs:4197:9:
assertion `left == right` failed: a runtime that ran 59 s stopped short of the 60 s and waits: [Done { instance: "alpha", action: Start, reason: LostRuntime, result: Succeeded, detail: None }]
  left: Succeeded
 right: Refused
```

**R2 `adversary2::an_instance_upgraded_just_after_a_failed_start_reads_upgrading_until_it_settles` (`tests/instance_upgrade.rs`): red.** A failed start, the retry holds, an upgrade is asked 2 s after the retry, and the upgraded runtime does not answer yet.
```
panicked at crates/conductor/tests/instance_upgrade.rs:673:9:
assertion `left == right` failed: the upgrade has not settled, and its waiter reads any other state as its end
  left: String("Unreachable")
 right: "Upgrading"
```

**R3 `adversary2::doctor_waits_one_bound_for_a_silent_substrate_daemon_whatever_the_boxes`: red.** Three boxed instances, one socket that never answers. Its first run failed on my own row count (the verdict line also carries the refusal name); I fixed the count and ran it alone again before the suite:
```
panicked at crates/conductor/tests/placement.rs:4502:9:
doctor waited 17.364872747s for one silent daemon with three boxes
```

**R4 `adversary2::the_record_of_a_failed_start_carries_no_more_than_the_4096_bytes_it_reads`: red.** A log line of 4095 bytes of `0x01`.
```
panicked at crates/conductor/tests/placement.rs:4420:9:
the record carries 12285 bytes of a log of 4096
```

**R5 `adversary2::no_character_that_reorders_or_breaks_the_line_reaches_the_record`: red.**
```
panicked at crates/conductor/tests/placement.rs:4471:9:
in the record: ["U+202E", "U+202C", "U+2066", "U+2069", "U+2028", "U+2029", "U+200F", "U+061C"]
```

**Green cases** (all in `tests/placement.rs`, `mod adversary2`):

| Case | What it holds |
|---|---|
| `two_instances_keep_rows_of_their_own_and_a_waiting_one_never_reads_running` | Over 700 s of 2 s ticks, crash-looping `alpha` never reads `Running` and `beta` always does. `alpha` reaches the 300 s wait. `beta`, after running 700 s, is started at once, and its first failed start waits 10 s. |
| `an_instance_removed_and_added_again_is_stopped_once_and_starts_a_new_row` | The removal pass records one `Stop` that `Succeeded` and closes the dead box once. Added again, the instance starts at once and its next failed start is "1 in a row, 10 s". |
| `a_sparse_log_of_3_gib_and_a_pipe_for_its_directory_do_not_hold_the_pass` | The tail of a 3 GiB sparse log is read in under 5 s. A named pipe in place of `sessions` gives "is no regular file" without waiting. |
| `no_escape_sequence_with_parameters_and_no_c1_control_reaches_the_record` | CSI with parameters, OSC ended by `ESC \` or by U+009C, U+009B, U+0085, DEL, U+0090 and a trailing ESC: no control character reaches the record, and the text around them does. |
| `doctor_ends_within_its_bound_on_a_socket_that_answers_a_byte_a_second` | Doctor ends in under 9 s with `placement.substrate-unreachable … within 5 s`. |

## 3. The suite, after the cases existed

Command: `conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test placement --test daemon --test instance_api --test instance_upgrade --test doctor --test setup --no-fail-fast`, exit 100.

```
    Starting 148 tests across 6 binaries
     Summary [  25.401s] 148 tests run: 143 passed, 5 failed, 0 skipped
        FAIL [   1.628s] ( 47/148) conductor-cli::instance_upgrade adversary2::an_instance_upgraded_just_after_a_failed_start_reads_upgrading_until_it_settles
        FAIL [   1.155s] ( 82/148) conductor-cli::placement adversary2::a_runtime_that_stops_59_s_after_its_start_does_not_end_the_row
        FAIL [   0.937s] ( 85/148) conductor-cli::placement adversary2::the_record_of_a_failed_start_carries_no_more_than_the_4096_bytes_it_reads
        FAIL [   1.645s] ( 88/148) conductor-cli::placement adversary2::no_character_that_reorders_or_breaks_the_line_reaches_the_record
        FAIL [  15.665s] (147/148) conductor-cli::placement adversary2::doctor_waits_one_bound_for_a_silent_substrate_daemon_whatever_the_boxes
error: test run failed
```

The only failures are my five. Every case of the unit, of the correction and of pass 1 passes, including pass 1's eight that were red.

## 4. Findings (all cover commit 8af6916fe)

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| G1 | `crates/conductor/src/daemon/reconcile.rs:1041` | R1: `ran` is taken at the look that finds the runtime stopped. A runtime that stops between 58 s and 60 s after its start is found at the 60 s look, ends the row and is started again at once, with no backoff. | A runtime that dies just under a minute every time. This is F4's defect moved to the 60 s boundary; the page (`the-host.md:109`) says "before it ran 60 s". The fix F4 got would be `RAN_BOUND` plus one tick. | CONFIRMED | introduced |
| G2 | `crates/conductor/src/daemon/reconcile.rs:1422` | R2: the `unproven` arm is matched before `upgrading`, and `upgrade` leaves `attempts` untouched. An instance upgraded less than 12 s after a start that followed a failed one reads `Unreachable`, not `Upgrading`. | `wait_upgraded` (`daemon/upgrade.rs:739`) and `remote.rs:404` take any state but `Upgrading` as the end. The row already shows the new `binary_version`, so `conductor instance upgrade` prints "upgraded to …" and exits 0 before the upgrade settled, and `--all` moves to the next instance. Only for a runtime in a unit (`upgrade` asks `units.active`), in a 12 s window. Swapping the two arms would fix it. | CONFIRMED | introduced |
| G3 | `crates/conductor/src/doctor.rs:1549` | R3: `admits` is called once per boxed instance, each with its own 5 s. Three boxes on one silent daemon took 15 to 17 s. | A host with many boxed instances and a stuck substrate daemon: 5 s times the number of boxes. The page says doctor "asks the substrate daemon for its facts" and names one 5 s. | CONFIRMED | introduced |
| G4 | `crates/conductor/src/daemon/reconcile.rs:223` | R4: the 4096-byte bound is applied to the read, before replacement. Each replaced byte becomes a three-byte U+FFFD, so the record's text reaches 12285 bytes. | A log tail of control or invalid bytes. It goes to conductord's output and the store row. Bounded at three times `SAID_BYTES`. | CONFIRMED | introduced |
| G5 | `crates/conductor/src/daemon/reconcile.rs:224` | R5: bidirectional overrides, isolates and marks (U+202A–202E, U+2066–2069, U+200E/F, U+061C) and U+2028/2029 reach the record. | `printable`'s own doc says "nothing a terminal acts on". These reorder the record's one line in a viewer that honours bidi. They are outside what F8 asked for (C0, DEL, C1, ESC sequences), which holds. | NEEDS-CHANGE | introduced |

Three answers to the brief that are not findings:

- **Config change during a wait:** it ends the wait and nothing else; the failure count carries on. It also starts every waiting instance, not only the one whose config changed.
- **The 12 s on the page:** the brief says it is not there. It is (`the-host.md:108` and `:113`), and it matches `start_window`. The 10 s, 60 s and 5 minutes match the code, apart from G1's boundary.
- **Hard link in place of the log:** `open_log` would read it, since it is a regular file. What stops it is substrate's confinement, which I could not test here: each root as its own bind mount gives `EXDEV`, and Landlock needs `REFER` on a file the box can name. The box can only link a file it can already see.

## 5. Attacked and not broken

- **`open_log` components:** `sessions` and the log are each opened with `O_NOFOLLOW`; the type check is `fstat` on the opened descriptor, so a swap after the open changes nothing. `log_len` uses the same opener. The state root itself is followed as a path; the box does not write its parent in the layouts the tests and pages use. A config nesting `state` under another writable root would be open; not tested.
- **Large, sparse and odd logs:** 3 GiB sparse is read by seek, under 5 s. A directory, a link, a pipe for the log or for `sessions` each give "no regular file" with no wait. `O_NONBLOCK` on a regular file changes no read, and the descriptor is used nowhere else.
- **`printable`:** no ESC and no control character survives any sequence I built. An unterminated OSC swallows the rest of its line.
- **Runs past 60 s:** the unit's own case holds the 60 s restart at once and the new row after it.
- **`Unreachable` window:** a runtime that exits after its 12 s look reads `Running` for less than one tick. No instance reads `Running` during a wait in 700 s of ticks.
- **Two instances:** rows are per instance; `looked` is shared and only read against each instance's own start.
- **Removed instance:** closed once, recorded as `Stop` `Succeeded`, no further close at later passes (pass 1's R3 holds 800 s). Remove and re-add cannot share a pass, since one pass reads one config; across two passes the row starts new.
- **Doctor's bound:** a byte-a-second answer ends at 5 s. The remote branch returns before `admits`, so nothing is sent; the row reads "… writable roots on the remote daemon's host (not checked here, that daemon is not asked)".
- **The two edited tests:** both advance by `RAN_BOUND` in place of `START_BOUND` and assert the same outcome; not weakened. The correction's edits inside `mod adversary` are re-wrapping only.

## 6. Paths written outside the worktree

All in `~/.cache/scratch/041da894-c3d7-4881-90c9-e7f85e9b66c9/w50/u3/adv2/`:
- `adversary2_block.rs`
- `upgrade_block.rs`
- `alone.log`
- `alone2.log`
- `suite.log`
- the directory itself

The test runs also left case directories under the shared target's `tmp/placement/` (`adv2-*`, `d3s`, `ddrip`) and `tmp/instance-upgrade/adv2-unproven`. `adv2-sparse` holds a 3 GiB sparse file that uses almost no disk.

## 7. Findings block

```findings
- file: crates/conductor/src/daemon/reconcile.rs
  line: 1041
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the 60 s that end a row are measured at the look that finds the runtime stopped, so a runtime that stops 58 to 60 s after each start ends the row and is started again at once with no backoff, against the host page''s "before it ran 60 s"'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 1422
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'an instance upgraded within 12 s of a start that followed a failed one reads Unreachable instead of Upgrading, so conductor instance upgrade takes the upgrade as ended and prints upgraded before it settled'
- file: crates/conductor/src/doctor.rs
  line: 1549
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'doctor asks a silent local substrate daemon once per boxed instance with 5 s each, so three boxes took 15 to 17 s where the page names one 5 s bound'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 223
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the 4096-byte bound is applied before replacement, so a log tail of control or invalid bytes enters the record as up to 12285 bytes'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 224
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'printable lets bidirectional overrides, isolates and marks and the line and paragraph separators into the record, against its own doc of nothing a terminal acts on'
```
