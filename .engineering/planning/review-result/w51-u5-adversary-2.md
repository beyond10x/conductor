---
format: aep.planning-md/3
id: review-result:w51-u5-adversary-2
kind: review-result
status: active
title: Wave 51 U5 daemon-probes-and-metrics, adversary pass 2
relations:
- reviews: story:daemon-probes-and-metrics
revision: 1
---
```
unit: U5 wave 51, story:daemon-probes-and-metrics, worktree w51-u5 at 234e91458 plus one uncommitted test file
verdict: NEEDS-CHANGE
cases: executed 1847→1850, red 2
origin: introduced 3 / pre-existing 0 / undecided 2
wrote-outside-worktree: 3 paths
needs-coordinator: none
```

The 1847 is the count the implementor reported. No suite run came before my cases.

## 1. `git --no-pager diff --stat`

```
 crates/conductor/tests/adv_daemon_probes.rs | 226 ++++++++++++++++++++++++++++
 1 file changed, 226 insertions(+)
```

Only a test file changed. I ran `cargo fmt -p conductor-cli` once the run was over: it reflowed only my lines, and `--check` is now clean.

## 2. Cases added (`crates/conductor/tests/adv_daemon_probes.rs`, prefix `adversary2_`)

All three run the reconciler in process. They use a stand-in service manager (`Manager`) and the store's `refuse_writes` test switch, so none reaches systemd. Run 1 (these three cases alone): `3 tests run: 1 passed, 2 failed`. The line numbers below are from before fmt; after fmt they are about :528 and :563.

- **A. `…_a_pass_refused_half_way_reaches_the_rest_at_the_next_tick_and_does_nothing_twice`, green.** It guards the re-run. The store refuses alpha's record. At the next tick beta is started and alpha is not started again, and the store holds exactly 2 records.
- **B. `…_a_config_change_pass_refused_half_way_ends_the_wait_when_it_runs_again`, red:**
  ```
  panicked at crates/conductor/tests/adv_daemon_probes.rs:525:5:
  the pass run again was a pass for Some(Cadence), not for the config change: beta still waits out its back-off (starts ["conductor_alpha-runtime.service", "conductor_beta-runtime.service", "conductor_alpha-runtime.service"])
  ```
- **C. `…_a_daemon_stopped_while_it_holds_a_start_does_not_start_that_runtime_again`, red:**
  ```
  panicked at crates/conductor/tests/adv_daemon_probes.rs:559:5:
  assertion `left == right` failed: the next daemon started a runtime that ran; the store holds 1 reconciliation(s)
    left: ["conductor_alpha-runtime.service", "conductor_alpha-runtime.service"]
   right: ["conductor_alpha-runtime.service"]
  ```

## 3. Suite run, after the cases existed

Command: `conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`. Exit 100.

```
Summary [ 735.341s] 1850 tests run: 1844 passed (10 slow), 6 failed, 12 skipped
  FAIL adv_daemon_probes adversary2_a_daemon_stopped_while_it_holds_a_start_does_not_start_that_runtime_again
  FAIL adv_daemon_probes adversary2_a_config_change_pass_refused_half_way_ends_the_wait_when_it_runs_again
  FAIL build_queue_convoy waiting_runs_take_no_lock_while_the_ledger_does_not_change
  FAIL build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger
  FAIL build_short_lane a_command_without_a_short_history_waits_for_the_regular_slot
  FAIL status status_answers_within_2_s_on_an_instance_of_50k_tree_files_reading_the_newest_snapshot_only
```

- `build_queue_convoy` and `build_run` are on the known list of failures under load.
- `build_short_lane` (an empty slow-p90) and `status` (3.57 s against a 2 s bound) are timing bounds outside this unit's files. I did not re-run them alone.
- Clippy (`-p conductor-cli --all-targets -- -D warnings`): exit 0.

## 4. Findings (tree 234e91458)

| # | file:line | Finding | Verdict | Origin |
|---|---|---|---|---|
| 1 | `crates/conductor/src/daemon/reconcile.rs:1412` | The re-run after a refused record is not the same pass. `pass()` takes the new config text at :887 before it acts, and `due()` answers `Cadence` whenever `last_pass` is `None`. So a config-change pass refused half-way runs again as a cadence pass. A runtime waiting out its back-off is not started by the config change, although `failed_start` says a config change ends the wait. Its records also name `Cadence`. | NEEDS-CHANGE | undecided |
| 2 | `crates/conductor/src/daemon/reconcile.rs:1007` | Held records live only in memory, and the refused pass stops before `save()` (:1036). A daemon stopped while it holds a start (unit restart, reboot) drops that record and does not know the runtime it started; the next daemon starts it again. With real systemd, `wait_gone` refuses that start ("still runs"). The runtime then runs untracked: its row reads `Stopped` and removing it from the config never stops it. | NEEDS-CHANGE | undecided |
| 3 | `website/docs/concepts/the-host.md:62` | Two parts of the new sentence do not hold. (a) "kept and written when the store takes writes again" is false across a daemon stop (finding 2). (b) "until then the daemon acts on nothing" is false for upgrades: `daemon.answer(request)` (`mod.rs:237`) runs before `record_held()` (`mod.rs:246`), so an upgrade request still fetches the release. Each such request adds one held record and answers 502. | CONFIRMED | introduced |
| 4 | `crates/conductor/src/daemon/reconcile.rs:1412` | No existing test fails when `self.last_pass = None` is deleted. Case A of pass 1 and `a_refused_record_is_not_ready…` both trigger later passes by touching the config. My case A would catch that mutant (`due` would answer `None`). | CONFIRMED | introduced |
| 5 | `crates/conductor/src/daemon/metrics.rs:260` | No test reads a scrape after the keep. A mutant that never refreshes the kept answer (`at.elapsed() < KEEP` replaced by `true`) stays green: `metrics_hold…` uses a fresh `Api` per scenario. | CONFIRMED | introduced |

What was measured and what reaches it:

- **Finding 1.**
  - Measured: case B, exit 100.
  - Reached by: an operator's config edit during a write the store refuses (another SQLite writer holding the lock longer than 5 s, or a Postgres hiccup) while some instance is in a failed-start back-off. That is a narrow overlap.
  - Origin: the base also aborted on the record (base `reconcile.rs:1003`, before `save` at :1032) and later took a cadence pass. I read that and did not run it, hence `undecided`.
  - Fix: keep the refused pass's reason for the re-run, and have `due()` return it when `last_pass` is `None`.
- **Finding 2.**
  - Measured: case C, exit 100.
  - Reached by: a restart of `conductord` while the store refuses writes.
  - Origin: the abort before `save` has the same order on the base (read, not run).
  - Fix: write the runtimes file before the record, or keep the held records on disk.
- **Finding 3.** Read from the code, no case.
- **Findings 4 and 5.** Read only. I ran neither mutant: building a mutated copy needs a second target directory, which the wave rules forbid. Both mutants of the brief (keep at zero, kind filter dropped) are red in the implementor's logs (`r2/focused-keep-mutant.log`: "the instance was asked per scrape"; `r2/red.log`: `left: Some(4)`). For my own mutant of the queue filter (excluding only one non-Disk kind), the fixture's `ControllerSlot` row catches it, by reading.

## 5. Attacked and not broken

- **Held set is bounded.** It grows only through upgrade requests while the store refuses, because `record_held` runs first on every tick and the pass stops at its first refused record. Records are written in the order taken, before any newer one.
- **Retry of a write that landed.** A new handle reads the stream version and appends to the same stream (`store.rs:1553`). It is not refused, and there is no second row. Nothing stays held for ever.
- **Back-off of wave 50 holds.** `failed_start` sets `retry_at` before its record (`reconcile.rs:1082`), so the re-run waits. A refused start (`start_runtime` Err) gets one extra attempt at the next tick, as the implementor stated.
- **No action runs twice in the re-run.** Checked for start, restart and the stop of a removed instance; case A confirms start.
- **Probes do not block on the store.** They read an atomic and two mutexes, none held during store I/O.
- **`/v1/` reads during an outage.** They are served from the last publication while records are held. Rows stay stale because `observe` is skipped.
- **One journal line per tick** while a record is held (2 s tick plus the 5 s SQLite busy wait). The base loop does the same for any failure that repeats each tick, so I did not count it as a finding.
- **Readiness flapping.**
  - Re-publication swaps the whole `Published` under one lock, so a config change or upgrade has no moment that reads unpublished.
  - A record refused once and written at the next tick gives 503 `unrecorded` from the end of that tick to the end of the next one: at most one scrape.
- **503 callers.**
  - `conductor status` falls back to its local read.
  - The TUI, the dashboard and `remote::instance_upgrade` show `could not read …: 503` or the `error` field.
  - `conductor login` gets its 401 before the gate.
  - `conductor doctor` reads only `systemctl is-active`, so it still ends `ready`.
  - The body is `{"error": <reason>}`, the shape the callers parse.
- **Gated writes have no effect when refused.** `ask()` and `start_role()` run after the gate (`api.rs:1296`, `:1317`). `admit_write` only checks.
- **Reason strings.** They are fixed sentences, and `unpublished` computes them with the same `readiness()` as `/readyz`.
- **`instance.yaml:213` `UNMAPPED:` note.** It matches `api.rs:756`.

## 6. Paths written outside the worktree

- `<scratch>/w51/u5/adv2/run1-adversary2-alone.log`
- `<scratch>/w51/u5/adv2/run2-package.log`
- `<scratch>/w51/u5/adv2/run3-clippy.log`

All builds went through `conductor build run` into the shared target directory. I took no worktree lease.

## 7. Findings block

```findings
- file: crates/conductor/src/daemon/reconcile.rs
  line: 1412
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: 'a pass refused half-way runs again as a Cadence pass, so a config change no longer ends the back-off of a runtime it did not reach and its records name the wrong reason'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 1007
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: 'held records and the started runtime live only in memory because the refused pass stops before save, so a daemon stopped while it holds a start loses the record and the next daemon starts the runtime again'
- file: website/docs/concepts/the-host.md
  line: 62
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the page says a refused record is kept until the store takes writes and the daemon acts on nothing meanwhile, but the record does not survive a daemon stop and an upgrade request is still fetched and answered before the held records'
- file: crates/conductor/src/daemon/reconcile.rs
  line: 1412
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'no existing test fails when the reset of last_pass is deleted; adversary2_a_pass_refused_half_way_reaches_the_rest_at_the_next_tick_and_does_nothing_twice is the guard, not run against the mutant'
- file: crates/conductor/src/daemon/metrics.rs
  line: 260
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'no test reads a scrape after the keep, so a kept metrics answer that never refreshes stays green'
```
