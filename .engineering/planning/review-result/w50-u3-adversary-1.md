---
format: aep.planning-md/3
id: review-result:w50-u3-adversary-1
kind: review-result
status: active
title: Wave 50 U3 box-crash-loop-reads-failed, adversary pass 1
relations:
- reviews: story:box-crash-loop-reads-failed
revision: 1
---
```
unit: U3 wave 50, story:box-crash-loop-reads-failed, commit 5c4716794 on base f13a87bd3 (worktree w50-u3)
verdict: CONFIRMED
cases: executed 109→125, red 8
origin: introduced 10 / pre-existing 0 / undecided 0
wrote-outside-worktree: 12 paths, all in the assigned scratch directory
needs-coordinator: none
```

`executed <before>` is the suite run of part 3 minus my 16 cases; I made no separate run with them deselected.

## 1. `git --no-pager diff --stat`

```
 crates/conductor/tests/daemon.rs    |  59 ++++
 crates/conductor/tests/placement.rs | 526 ++++++++++++++++++++++++++++++++++++
 2 files changed, 585 insertions(+)
```

Both are test files, appended `mod adversary` blocks, uncommitted. No implementation file was touched. I took no worktree session lease.

## 2. Cases added

All red cases are in `~/.local/state/worktree/trees/b10x/conductor/w50-u3/crates/conductor/tests/placement.rs`, `mod adversary`. The red output is from the run of `adversary::` alone, before the suite.

**R1 `a_failed_start_does_not_record_a_host_file_its_log_links_to`: red.** `runtime.log` is replaced by a link to a file outside every root of the box.
```
panicked at crates/conductor/tests/placement.rs:3474:9:
the record carries a file of the host the box cannot read: the runtime of alpha exited within 10 s of its start (failed start 1 in a row, the next start in 10 s): host-only-value-0123456789
```

**R2 `a_failed_start_whose_log_is_a_named_pipe_does_not_hang_the_pass`: red.** `runtime.log` is a named pipe.
```
panicked at crates/conductor/tests/placement.rs:3512:9:
the pass did not return within 20 s: it waits to open the pipe …/adv-pipe/alpha/state/sessions/runtime.log
```

**R3 `an_instance_removed_while_its_failed_start_waits_has_its_box_closed`: red.** The instance is taken out of the config during the wait; ticks follow up to 800 s.
```
panicked at crates/conductor/tests/placement.rs:3569:9:
assertion `left == right` failed: the box of the removed instance is still open on substrate
  left: []
 right: ["ws_1"]
```

**R4 `a_runtime_that_exits_nine_seconds_after_its_start_is_a_failed_start`: red.** Ticks every 2 s; the runtime exits at 9 s and is found at the 10 s tick.
```
panicked at crates/conductor/tests/placement.rs:3412:9:
assertion `left == right` failed: a runtime that exited 9 s after its start is a failed start, not a start that succeeded: [Done { instance: "alpha", action: Start, reason: LostRuntime, result: Succeeded, detail: None }]
  left: Succeeded
 right: Refused
```

**R5 `the_start_after_a_failed_start_does_not_read_running_at_once`: red.** After a failed start, the retry's start is read before the next look.
```
panicked at crates/conductor/tests/placement.rs:3440:9:
assertion `left != right` failed: a crash-looping runtime reads Running after each of its starts
  left: String("Running")
 right: "Running"
```

**R6 `doctor_does_not_look_on_this_host_for_the_roots_of_a_box_on_a_remote_daemon`: red.**
```
panicked at crates/conductor/tests/placement.rs:3633:9:
a directory of the remote host is asked for on this one: box alpha: placement.writable-root-missing: the writable root state-alpha is no directory: create …/adv-remote-roots/alpha/state
```

**R7 `doctor_ends_when_the_substrate_socket_takes_the_connection_and_never_answers`: red.** The socket is bound and never accepted or answered. Its first run failed on my own typo (socket path over `SUN_LEN`); I shortened the path and ran it alone again before the suite:
```
panicked at crates/conductor/tests/placement.rs:3613:9:
conductor doctor did not end within 45 s on a substrate socket that never answers
```

**R8 `a_failed_start_carries_no_control_character_of_the_log`: red.**
```
panicked at crates/conductor/tests/placement.rs:3543:9:
a control character of the runtime's log is in the record: "the runtime of alpha exited within 10 s of its start (failed start 1 in a row, the next start in 10 s): conductor: instance serve: \u{1b}[2J\u{1b}]0;title\u{7} the store\rdoes not open"
```

**Green cases for mutants the unit's suite does not catch.** No mutant was built or run; each is argued from the code.

| Case | Mutant it would catch |
|---|---|
| `the_wait_doubles_from_the_start_bound_and_stops_at_the_cap` | `retry_after` with no cap or another cap. The unit's `a_crash_looping_boxed_runtime_is_started_ever_less_often` passes with the cap removed: its 30 minutes hold one start after 1200 s either way. |
| `a_crash_loop_is_started_at_least_every_cap` | the same through `tick`: gaps 12, 22, 42, 82, 162, 302, then always 302 s over two hours |
| `a_config_change_pass_starts_a_runtime_that_waits` | `!asked` dropped from the wait check; no unit case ended a wait by a config change |
| `a_failed_start_found_by_a_config_change_pass_is_started_at_the_next_tick` | the `Duration::ZERO` branch |
| `a_runtime_that_ran_past_the_bound_ends_the_row_of_failed_starts` | `attempt.failures = 0` dropped |
| `the_record_carries_the_last_eight_lines_of_at_most_4096_bytes_and_a_replaced_log` | `SAID_LINES`, `SAID_BYTES`, the `from > len` branch |
| `doctor_names_an_unserved_family_and_a_root_that_is_no_directory` | `unmet` dropped from `Sdk::unserved` as doctor sees it; `roots_missing` for a file and a dangling link; nothing written |
| `tests/daemon.rs` `adversary::a_runtime_in_a_unit_that_stops_within_the_start_bound_is_a_failed_start` | the `boxed` branch of `failed_start`; no unit case held a failed start of a runtime in a unit. First run in the suite, not alone. |

Mutants the unit's own cases already catch:
- `>= START_BOUND` flipped to `>`: the two edited tests.
- The retry term dropped from `due`: `due(retry)` in the unit's first case.
- The failure count not carried over a retry: the crash-loop case.
- `roots_missing` dropped: the records-removed step of the doctor case.

## 3. The suite, after the cases existed

Command: `conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test placement --test daemon --test instance_api --test doctor --test setup --no-fail-fast`, exit 100.

```
    Starting 125 tests across 5 binaries
     Summary [  46.969s] 125 tests run: 117 passed, 8 failed, 0 skipped
        FAIL [   0.300s] ( 67/125) conductor-cli::placement adversary::doctor_does_not_look_on_this_host_for_the_roots_of_a_box_on_a_remote_daemon
        FAIL [   0.618s] ( 71/125) conductor-cli::placement adversary::a_failed_start_does_not_record_a_host_file_its_log_links_to
        FAIL [   0.693s] ( 72/125) conductor-cli::placement adversary::a_failed_start_carries_no_control_character_of_the_log
        FAIL [   0.819s] ( 79/125) conductor-cli::placement adversary::a_runtime_that_exits_nine_seconds_after_its_start_is_a_failed_start
        FAIL [   0.861s] ( 82/125) conductor-cli::placement adversary::an_instance_removed_while_its_failed_start_waits_has_its_box_closed
        FAIL [   0.799s] ( 94/125) conductor-cli::placement adversary::the_start_after_a_failed_start_does_not_read_running_at_once
        FAIL [  20.196s] (124/125) conductor-cli::placement adversary::a_failed_start_whose_log_is_a_named_pipe_does_not_hang_the_pass
        FAIL [  45.180s] (125/125) conductor-cli::placement adversary::doctor_ends_when_the_substrate_socket_takes_the_connection_and_never_answers
error: test run failed
```

The only failures are my eight; every case the unit shipped passes. `build run` ended with "the ledger was not free within 30s … grant 01a1c7d7-e42d-73c5-a6ae-c188ab714a74 stays Running, and the next command that reads the ledger releases it".

## 4. Findings (all cover commit 5c4716794)

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| F1 | `crates/conductor/src/daemon/reconcile.rs:112` | R1: `runtime_said` follows a link at `<state>/sessions/runtime.log` and copies up to 8 lines / 4096 bytes of the target into the record | Any process that writes the state root plants the link and exits within 10 s. The record goes to conductord's standard output and to the sqlite store, whose directory is itself a writable root of the box, so the box reads it back. Today only conductor's runtime and what it runs are in the box; sessions run unboxed. | CONFIRMED | introduced |
| F2 | `crates/conductor/src/daemon/reconcile.rs:112` | R2: `File::open` on a named pipe blocks the pass, and with it conductord's one loop for every instance and the API's published state | Same writer as F1: `mkfifo` in the state root, then exit | CONFIRMED | introduced |
| F3 | `crates/conductor/src/daemon/reconcile.rs:865` | R3: the removed instance's box is never closed; its entry stays in `boxes.json`, its leases are renewed, its model-token slot stays held | An operator removes a crash-looping instance from the config during the wait of up to 300 s. The line is older than the unit; before it the dead box was closed at the next tick, so only a same-tick race reached it. The unit's choice to hold the box open makes this the normal path. | CONFIRMED | introduced |
| F4 | `crates/conductor/src/daemon/reconcile.rs:904` | R4: the bound is taken at the tick that finds the runtime stopped, not at its exit. A runtime that exits between about 8 s and 10 s is restarted at once as `Succeeded`, the count reset, every 10 s without backoff. | The host page line 106 says "exits within 10 s". Any start error that takes 8 to 10 s, a connect or lock timeout for one; I did not find such a timeout in the runtime. Start time is the pass's `now`, before the box opens, which narrows the window further. | CONFIRMED | introduced |
| F5 | `crates/conductor/src/daemon/reconcile.rs:1271` | R5: after each retry's start the instance reads `Running` until the next look, though the daemon does not count that start as held | The story's own case: a runtime exiting at once reads `Running` for up to 2 s after each start. The unit's crash-loop test skips exactly those ticks (`if !started`). Closing it needs `Unreachable` held until the bound has passed after a failed start, or a state the model lacks. | CONFIRMED | introduced |
| F6 | `crates/conductor/src/doctor.rs:1542` | R6: for a box on a remote daemon, doctor checks the roots as directories of this host and answers `not ready`, advising to create them here | Any `placement: {daemon: https://…}` instance whose records or cache are not also on the local host; conductord itself creates only the state directory locally | CONFIRMED | introduced |
| F7 | `crates/conductor/src/doctor.rs:1547` (through `placement.rs:1831`, no timeout for a local daemon) | R7: `conductor doctor` did not end within 45 s; the code has no bound | A substrate-daemon process that is stopped or stuck: the kernel completes the connect on a listening socket. Doctor did not open the socket before this unit. Conductord has the same unbounded wait, which is older. | CONFIRMED | introduced |
| F8 | `crates/conductor/src/daemon/reconcile.rs:119` | R8: ESC, BEL and CR of the log reach the record | Conductord's standard output (`daemon/mod.rs:218`) and the store row. I found no dashboard, `status` or API reader of a reconciliation's `detail`. | CONFIRMED | introduced |
| F9 | `website/docs/concepts/the-host.md:106` | Not a case: "A runtime that ran longer and stopped is started again at once" also means a runtime that dies at 11 s loops every ~12 s, recorded `Succeeded`, with no backoff (20 starts in 4 minutes against the story's 91) | Any failure slower than 10 s | NEEDS-CHANGE | introduced |
| F10 | `SETUP.md:134`, `website/docs/getting-started.md:146` | Not a case: doctor now exits 1 with `not ready: box <name> (…)` from the moment the config writes `placement: systemd` until substrate serves. SETUP step 8 runs doctor before step 9 starts substrate; getting-started step 3 says doctor "now needs substrate-daemon" and nothing of the box row. | A reader of either page. SETUP's check is a printed line, so the reader is not stopped, but an exit-status reader is. | NEEDS-CHANGE | introduced |

Two smaller observations, not findings:
- The failed start's error text reaches no API route. An operator who sees `Unreachable` in `conductor status` gets the cause only from conductord's journal or the store.
- The record does not redact; no other reconciliation record does either.

## 5. Attacked and not broken

- **Backoff state:** per instance, in memory, dropped when the instance leaves the config. A conductord restart starts once at once, then backs off again from 10 s.
- **Starving others:** a waiting instance's pass returns nothing and does not hold up another instance's start in the same pass.
- **Clock:** `Instant` with saturating arithmetic; a step backwards is not reachable.
- **Log offset:** `log_to` appends, so the offset holds. A replaced shorter log, a 10 kB single line and the 8-line bound behave as documented.
- **Wrong log path:** the path is derived from the instance's own `state`.
- **Edited tests:** both now advance by exactly `START_BOUND` and one gained a `Succeeded` assertion; neither was weakened.
- **Doctor writes nothing:** a root that is a file or a dangling link is named and the tree is unchanged; substrate is asked only `GET /v1/machine`. An unserved family is named.
- **`daemon.store` skip:** conductord refuses to start on a file without `daemon.store` (`reconcile.rs:1405`), so the skip cannot give `ready` for a box conductord would refuse.
- **`check_in`:** unchanged; `box_lines` is called from `doctor_with` only.
- **Changes during the wait:** a config-change pass closes the dead box before the new start. Conductord has no shutdown handler; `boxes.json` carries the box to the next start, which closes it.
- **`instance_serve` early error:** the unit's case holds it in the log with empty standard error.

## 6. Paths written outside the worktree

All in `~/.cache/scratch/041da894-c3d7-4881-90c9-e7f85e9b66c9/w50/u3/adv/`:
- `adversary_block.rs`
- `daemon_block.rs`
- `run.sh`
- `wait.sh`
- `w.sh`
- `chain.sh`
- `adv1.log`
- `adv1.exit`
- `adv2.log`
- `adv2.exit`
- `suite.log`
- `suite.exit`

The test runs also left case directories under the shared target's `tmp/placement/` (`adv-*`, `dmu`) and `tmp/daemon/adv-unit-failed`, as every case of those binaries does.

## 7. Findings block

```findings
- file: crates/conductor/src/daemon/reconcile.rs
  line: 112
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: 'runtime_said follows a link at the box-writable runtime.log and copies another host file''s tail into the reconciliation record, which the box reads back from the store root'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 112
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: 'a named pipe in place of runtime.log blocks File::open and with it conductord''s whole reconcile loop'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 865
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'an instance removed from the config while its failed start waits keeps its dead box, leases and model-token slot open for ever, because the gone loop closes only a runtime that still runs'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 904
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the 10 s bound is measured at the tick that finds the runtime stopped, so a runtime that exits 8 to 10 s after its start is restarted at once as Succeeded with the count reset, against the host page''s "exits within 10 s"'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 1271
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'after each retry''s start a crash-looping instance reads Running until the next look, and the unit''s crash-loop test skips exactly those ticks'
- file: crates/conductor/src/doctor.rs
  line: 1542
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'doctor checks the writable roots of a box on a remote daemon as directories of the local host and answers not ready, advising to create them here'
- file: crates/conductor/src/doctor.rs
  line: 1547
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'conductor doctor waits without bound on a local substrate socket that takes the connection and never answers, since block_on bounds only a remote daemon'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 119
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'control characters and escape sequences of runtime.log reach the record''s detail and conductord''s standard output unescaped'
- file: website/docs/concepts/the-host.md
  line: 106
  category: judgement
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'a runtime that dies just past the bound is restarted at once every time with no backoff and recorded Succeeded, so a slower crash loop still runs about 20 starts in 4 minutes'
- file: SETUP.md
  line: 134
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'SETUP step 8 and getting-started step 3 run doctor before substrate is started and do not say it now exits 1 with a not ready box row until step 9'
```
