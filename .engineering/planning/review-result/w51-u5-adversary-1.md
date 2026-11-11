---
format: aep.planning-md/3
id: review-result:w51-u5-adversary-1
kind: review-result
status: active
title: Wave 51 U5 daemon-probes-and-metrics, adversary pass 1
relations:
- reviews: story:daemon-probes-and-metrics
revision: 1
---
```
unit: U5 wave 51, story:daemon-probes-and-metrics, worktree w51-u5 at 427dc4370 (base 61787f015) plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 1841→1843, red 2
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths
needs-coordinator: none
```

`executed <before>` is the implementor's reported 1841; no suite run preceded my cases.

## 1. `git --no-pager diff --stat`

Empty: no tracked file changed. `git status --short` shows the one added file, uncommitted:

```
?? crates/conductor/tests/adv_daemon_probes.rs
```

## 2. Cases added

File: `<trees>/w51-u5/crates/conductor/tests/adv_daemon_probes.rs`. Both cases are red now; clippy on the file with `-D warnings` is clean (run 3).

**A. `after_a_write_the_store_refused_the_daemon_records_again_or_reads_not_ready` (red)**

A real `daemon serve` runs on a SQLite store with the stand-in `PATH`. The test takes the store's write lock (`BEGIN IMMEDIATE`) from a second connection, triggers one pass, and releases the lock. It triggers another pass and asserts that within 40 s either a reconciliation is recorded or `/readyz` is not 200. Run 1, alone, verbatim apart from the cut `systemd-run` line:

```
thread 'after_a_write_the_store_refused_the_daemon_records_again_or_reads_not_ready' panicked at crates/conductor/tests/adv_daemon_probes.rs:299:5:
/readyz answers 200 [{"reachable":true,"reason":null}] while the daemon records nothing: 1 reconciliations in the store, 1 when the store took writes again (a pass failed while it took none: true; /readyz answered 200 meanwhile). The daemon printed:
secrets: keychain namespace default (answers)
conductord: config …/tmp/adp/refused-write/conductor.yaml, serving the dashboard and /v1/ on http://127.0.0.1:40129/
alpha: Start (Cadence): Refused: start the unit conductor_alpha-runtime.service: `systemd-run --user …` exited 1: this case starts no unit
conductor: daemon serve: record the reconciliation of alpha: the store failed on conductor.instance.Reconciliation "01a1cb17-afe9-728f-936a-6576be3ad75b": event store is unavailable: database is locked
conductor: daemon serve: record the reconciliation of alpha: the store failed on conductor.instance.Reconciliation "01a1cb17-afe9-728f-936a-6576be3ad75b": event store is unavailable: database is locked
```

The second failure line was printed after the lock was released and names the first reconciliation's id: the handle answers its first failure for ever.

**B. `readiness_is_not_200_before_the_reconciler_published` (red)**

In-process `Api`, `.starting()`, then `started()` and `publish_store(true)`: the state `daemon_serve` is in after a store that was down at start answers. Run 1, alone:

```
thread 'readiness_is_not_200_before_the_reconciler_published' panicked at crates/conductor/tests/adv_daemon_probes.rs:359:5:
assertion `left == right` failed: /readyz answers 200 [{"reachable":true,"reason":null}] while the reconciler published nothing and /v1/instances answers []
  left: 200
 right: 503
```

The run-1 line numbers 299 and 359 are 296 and 327 in the file as it stands. After run 1 I removed a third case that passed (see part 5).

## 3. Suite run, after the cases existed

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast --success-output final`, exit 100:

```
Summary [ 179.570s] 1843 tests run: 1841 passed (4 slow), 2 failed, 12 skipped
    FAIL [   1.180s] ( 137/1843) conductor-cli::adv_daemon_probes readiness_is_not_200_before_the_reconciler_published
    FAIL [  58.468s] ( 785/1843) conductor-cli::adv_daemon_probes after_a_write_the_store_refused_the_daemon_records_again_or_reads_not_ready
error: test run failed
```

The two failures are the added cases. The three failures the implementor reported all passed in this run.

## 4. Findings (tree 427dc4370)

| # | file:line | Finding | Verdict | Origin |
|---|---|---|---|---|
| 1 | `crates/conductor/src/daemon/metrics.rs:193` | `/readyz` reads only the watcher's own handle (`metrics.rs:119-134`), which is reopened on failure. The reconciler's handle (`reconcile.rs:459`) is opened once and refuses every write after its first failure (`store.rs:1353`). After one failed write the daemon answers `/readyz` 200 and `/healthz` 200 for ever, each pass stops at its first record (`reconcile.rs:1003`) before `save()`, and no probe ever gets it restarted. | NEEDS-CHANGE | introduced |
| 2 | `crates/conductor/src/daemon/metrics.rs:102` | `not_ready` knows nothing of the reconciler's publication. In the store-down-at-start branch (`mod.rs:218-228`) `/readyz` turns 200 up to one tick plus `Daemon::open` before `publish`, while `/v1/instances` answers `200 []`. The commit message says "ready once the store answers and the reconciler has published once"; the code does not do that. | NEEDS-CHANGE | introduced |
| 3 | `crates/conductor/tests/daemon_probes.rs:269` | While the store does not answer, `/v1/instances` answers `200 []` for a config that names instances, and the unit's tests pin it (also `:323`, `:455`). The unit's own comment at `api.rs:120` calls those rows "not empty but unread", and its metrics leave an unread value out. On the base nothing listened in that state. | CONFIRMED | introduced |
| 4 | `crates/conductor/src/daemon/metrics.rs:58` | No test fails when `KEEP` is 0: nothing asserts that two scrapes inside 5 s ask the instances once, which is the only bound on the fan-out a caller without a bearer can cause. `store/host.rs:83` likewise: the queue fixtures hold only `Disk` grants, so dropping the kind filter stays green. Neither was run against a mutated copy. | CONFIRMED | introduced |

**What was measured and what reaches it**

- **Finding 1.** Measured: case A, `adv_daemon_probes.rs:296`, exit 100. Reached by any pass that records while a write fails: another writer holding the SQLite store longer than the 5 s busy wait, or a Postgres outage during a start, stop or restart. A steady-state cadence pass writes nothing, so an outage with no reconcile action does not poison the handle. The non-recovery itself is on the base too (`store.rs:1349` there); the 200 that hides it, and a liveness that never restarts it, are this unit's. The fix is to reopen the reconciler's store when `check()` fails, or to make readiness (or liveness) follow the reconciler's handle; case A accepts either.
- **Finding 2.** Measured: case B, `adv_daemon_probes.rs:327`. Reached by a store that is down at start and then answers; the case builds that state in process, the code path is `mod.rs:218-228`. The fix is `not_ready` also requiring `Published.reconciled`.
- **Finding 3.** Not a new case: the implementor's own assertions show it. Reached by any client that does not gate on `/readyz` (the loopback dashboard, the CLI, a direct pod address).
- **Finding 4.** Read from the tests, not measured.

## 5. Attacked and not broken

- Bearer rule: the probe match is exact-path after the query is cut; `//healthz`, `/metrics/`, `/HEALTHZ`, `/healthz/../v1/…` and percent-encoded forms fall through to the old branches, and other methods get 405. Read from the code, not run.
- Host check: it precedes the probe branch on the loopback listener (`api.rs:632-653`); the TLS listener has none, as on the base.
- Body leak: `/readyz` reasons are three fixed strings; metric labels are positions and `[a-z0-9_]` cut to 40 characters; the store's error text goes to stderr only.
- Unknown store scheme (`mysql://…`): my third case expected the daemon to keep running; it exits non-zero naming `daemon.store` (`config.rs:2604`). Case removed.
- Watcher creating a SQLite store: it calls the same `open_url` (`Mode::Create`, parent directories) as the base's `Daemon::open`; no new path or file mode.
- Operators before the store answers: same source (`daemon.operators` of the file) and same `admit_write` rule as the base.
- Writes while the store is down: `PUT /v1/config` is 500, upgrade is 503, a proxied write is 404; nothing is accepted and lost.
- Absent, never `0`: no metric found that prints `0` for an unread source in a reachable state; `data.json` holds the two fields the scrape reads (`dashboard.rs:1589-1592`).
- Parity fixture: the table printed by the run is identical to the committed fixture, row order included.

## 6. Paths written outside the worktree

- `<scratch>/w51/u5/adv1/run1-adv-alone.log`
- `<scratch>/w51/u5/adv1/run2-package.log`
- `<scratch>/w51/u5/adv1/run3-clippy.log`
- `<scratch>/w51/u5/adv1/parity-printed.md`

The three runs built into the shared target directory through `conductor build run`. My worktree lease (`w51-u5-adv1`) is released.

## 7. Findings block

```findings
- file: crates/conductor/src/daemon/metrics.rs
  line: 193
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '/readyz answers from the watcher''s own store handle, so after one failed write on the reconciler''s handle the daemon reads ready and alive for ever while every pass stops at its first record and nothing is recorded again'
- file: crates/conductor/src/daemon/metrics.rs
  line: 102
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'readiness turns 200 when the store answers and before the reconciler published, so a store that was down at start reads ready while /v1/instances still answers an empty list'
- file: crates/conductor/tests/daemon_probes.rs
  line: 269
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'while the store does not answer /v1/instances answers 200 with an empty list for a config that names instances, and the unit''s tests pin that answer'
- file: crates/conductor/src/daemon/metrics.rs
  line: 58
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'no test fails when the 5 s keep of a /metrics answer is zero or when the Disk filter of the queue count is dropped'
```
