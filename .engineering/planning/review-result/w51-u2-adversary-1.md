---
format: aep.planning-md/3
id: review-result:w51-u2-adversary-1
kind: review-result
status: active
title: Wave 51 U2 handover-restart-by-controller-id, adversary pass 1
relations:
- reviews: story:handover-restart-by-controller-id
revision: 1
---
```
unit: U2 wave 51, story:handover-restart-by-controller-id, head c8859688b plus 133 uncommitted test lines in the worktree
verdict: NEEDS-CHANGE
cases: executed 1839→1842, red 3
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 4 paths
needs-coordinator: whether F1 (acceptance 2 via the usage-limit path, reproduces on base) holds this unit; whether dropping 1.2.0 entries (F3) was a decision taken with you
```

## 1. Diff stat of the worktree

```
 crates/conductor/tests/run_handover.rs | 133 +++++++++++++++++++++++++++++++++
 1 file changed, 133 insertions(+)
```

Test file only. A probe was added to `tests/session_attach.rs`, run once and removed again; its diff is kept in scratch (part 6).

## 2. Cases added

All three are in `<trees>/w51-u2/crates/conductor/tests/run_handover.rs` and are red now. They were run alone before anything else: `cargo nextest run -p conductor-cli --test run_handover --no-fail-fast adversary_`, 3 run, 0 passed, 3 failed, exit 100.

**F1, `:757` `adversary_a_session_started_after_the_delivery_is_not_stopped_by_the_usage_limit_path`**
- **Asserts:** a session asked, then marked by a usage limit, then ended and started again by hand after the delivery, is not stopped by the next tick with headroom.
```
panicked at crates/conductor/tests/run_handover.rs:777:5:
assertion `left == right` failed: a session started after the delivery is never stopped: ["sent", "controller-restarted"] []
  left: ["s0000001-0000"]
 right: []
```

**F2, `:791` `adversary_a_session_of_the_same_name_elsewhere_does_not_drop_the_request`**
- **Asserts:** a second live session of the same name, in a directory outside the instance's checkouts, does not drop the request to the controller's own session.
- The variant with the stray listed after the controller's session passes; the one with it listed first fails.
```
panicked at crates/conductor/tests/run_handover.rs:813:9:
assertion `left == right` failed: adversary-stray-listed-before: the request to the controller's own session stands: ["conductor run: hand-over of ex-alpha: it was asked of the session s0000001, and the session a0000000-0000 holds the name now; the request is dropped and nothing is stopped"]
  left: Null
 right: String("01a1cad8-e2a4-73d8-b744-75b83ed5560d")
```

**F3, `:827` `adversary_a_request_of_an_earlier_version_to_the_running_session_is_not_lost`**
- **Asserts:** an entry as 1.2.0 wrote it, for the session the running controller still runs (started before the delivery, hand-over file written, idle), leads within two ticks to a restart or a new `[HANDOVER]`.
```
panicked at crates/conductor/tests/run_handover.rs:852:5:
the controller asked before the upgrade is neither restarted at its hand-over nor asked again: ticks [] [], journal ["conductor run: hand-over of ex-alpha: the request kept names no controller, as an earlier version wrote it; the request is dropped and nothing is stopped"] [], deliveries [], kept {}
```

## 3. Suite run, after the cases existed

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100:

```
Starting 1842 tests across 152 binaries (12 tests skipped)
FAIL conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger
FAIL conductor-cli::run_handover adversary_a_request_of_an_earlier_version_to_the_running_session_is_not_lost
FAIL conductor-cli::run_handover adversary_a_session_started_after_the_delivery_is_not_stopped_by_the_usage_limit_path
FAIL conductor-cli::run_handover adversary_a_session_of_the_same_name_elsewhere_does_not_drop_the_request
Summary [ 157.364s] 1842 tests run: 1838 passed (1 slow), 4 failed, 12 skipped
```

The `build_run` failure is the known one under load. The `<before>` of 1839 is the implementor's reported count.

## 4. Findings (tree: c8859688b plus the cases above)

| # | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| F1 | `crates/conductor/src/run/handlers.rs:599` | NEEDS-CHANGE | pre-existing | Case `:757`: the tick wrote `sent`, `controller-restarted` and called `claude stop s0000001-0000` on a session whose `startedAt` lies after the delivery. | `pending()` runs `limited()` (`:85`) before `handovers()` (`:91`), so the entry is used before `superseded` can drop it. A session asked, stopped at a limit, restarted by hand once the limit lifts, and the next tick has headroom. |
| F2 | `crates/conductor/src/run/handlers.rs:410` | CONFIRMED | introduced | Case `:791`: the entry is dropped with "the session a0000000-0000 holds the name now" while the session asked is live under the same name; the result depends on list order. | `live_entry` (`crates/conductor/src/confine/mod.rs:913`) takes the first live entry of the name with no directory or instance filter. The comment at `crates/conductor/src/watch.rs:1637` records such a same-named session outside the repository directory on 2026-11-08. |
| F3 | `crates/conductor/src/run/handlers.rs:397` | CONFIRMED | introduced | Case `:827`: the 1.2.0 entry for the running session is dropped; two ticks later nothing is restarted and nothing delivered. | Every controller asked before the upgrade and still running when the loop restarts on the new binary. |
| F4 | `crates/conductor/src/run/handlers.rs:426` | INFEASIBLE | introduced | By reading only: `withdrawn()` (`:509`) ends and resumes a session under the same id without `forget`; if the harness lists the resumed process's start as `startedAt`, the entry is dropped as "started after it was asked". | `conductor leave` while a hand-over is pending. Not shown: what `startedAt` is after `--resume`. |

- **F1:** the base has the same restart (`asked.get(session).is_some()` at base `handlers.rs:492`, read with `git show`, not run). I marked it pre-existing by that reading, but it is this unit's acceptance 2 ("never stops a session started after the entry's delivery") that it breaks, and the unit edited this branch (`stands`) without the start check. Fix named, not applied: `stands` applies the same session checks as `superseded`, or `handovers()` runs before `limited()`.
- **F2 and F3 share one consequence:** a dropped request to a session that is still the same session is never made again. The watch prints `context:` once per session id (`crates/conductor/src/watch.rs:1629`; nothing removes a line from `context-reported`; pinned by `crates/conductor/tests/watch.rs:706` and `:798`). The answer to your question "is the controller ever asked again" is no.
- **F2 fix named:** drop only when no live entry of the name carries the asked id.
- **F3 contradicts the implementor's own case** `an_entry_that_names_no_controller_stops_nothing_and_is_forgotten` (`crates/conductor/tests/run_handover.rs:652`), which pins the drop for that same legitimate session. The story's acceptance does not ask for it. The 1.2.0 entry's `line` already names the session (`(session s0000001)`), so it can be tied to the live session, or the tick can ask again when it drops.

## 5. Attacked and not broken

- **Real-tmux case:** a probe typing the base way (one buffer `conductor`, one paste file) failed in 4 of 4 rounds with `paste-buffer … exited 1: no buffer conductor`; the unit's case passed in the same run. It fails on the base for the stated reason and did not pass by timing here.
- **Buffer-name normalisation in `tests/deliver.rs`:** it does not hide a shared name. The new case reads the raw calls, requires one buffer per delivery, two different ones, and the `conductor-<pid>-<count>` shape; dropping the pid or the counter fails it.
- **Paste file:** removed before the `load-buffer` result is checked, so also when tmux fails. A file left by a killed process is overwritten by a later process with the same pid and count, so there is no collision.
- **`startedAt` on this host's harness:** a 13-digit number, so milliseconds as the code assumes; `sessionId` is the full id the watch's eight characters come from. I read key names and shapes only.
- **`startedAt` equal to the delivery:** the comparison is strict, so the entry stands.
- **Empty session list:** nothing is superseded and the tick goes on as on the base.
- **Failing session list:** the tick errors and stops nothing; every base path read the list too, so no restart is lost that the base made.
- **Daemon restart:** the entry and its wall-clock delivery are on disk; nothing depends on process state.
- **Clock stepping back:** a false drop needs the delivery within the step size of the session's start; not reachable in practice.
- **Mutants of `superseded`:** the unit's cases catch the controller branch (the journal must name the old controller id) and the millisecond factor in both directions.

## 6. Paths written outside the worktree

- `<scratch>/w51/u2/adv1/cases-alone.log`
- `<scratch>/w51/u2/adv1/probe-one-buffer.log`
- `<scratch>/w51/u2/adv1/probe-one-buffer.patch`
- `<scratch>/w51/u2/adv1/suite.log`

The builds went to the shared target directory through `conductor build run`. No commit and no git write was made.

## 7. Findings block

```findings
- file: crates/conductor/src/run/handlers.rs
  line: 599
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: 'limited() runs before handovers() and restarts through a standing entry without the start check, so a session started after the delivery is stopped, against acceptance 2'
- file: crates/conductor/src/run/handlers.rs
  line: 410
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'superseded() takes the first live session of the name from an unfiltered list, so a same-named session elsewhere drops the request to the controller''s own live session, which the watch never asks again'
- file: crates/conductor/src/run/handlers.rs
  line: 397
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'an entry written by 1.2.0 for the session the running controller still runs is dropped and the controller is neither restarted at its hand-over nor asked again'
- file: crates/conductor/src/run/handlers.rs
  line: 426
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'the loop''s own resume in withdrawn() keeps the entry and may make the session look started after the delivery, dropping the request; the harness''s startedAt after a resume was not shown'
```
