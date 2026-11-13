---
format: aep.planning-md/3
id: story:two-cases-wait-on-conditions
kind: story
status: draft
title: Two test cases wait on a condition, not on a fixed time
tags:
- from-finding
revision: 1
---
## Why

Each case failed once in one local run of every test at `930a71fd1c` with a full build pool,
and passed alone at `fe4a5599a6` (as reported; not re-run for this record). Neither test file
changes between `v1.6.0` and `v1.7.0` (`git diff --stat v1.6.0..v1.7.0` prints nothing for
either). Read at `v1.7.0` (`951330f4f4`).

- `conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger`
  (`crates/conductor/tests/build_run.rs:3449`) failed with "build run did not exit within 30
  seconds of the signal".
  - After the first `SIGTERM` the case sleeps a fixed 300 ms, written as "The run now waits for
    the ledger", and checks only that the run has not exited (`build_run.rs:3471-3476`).
  - The second `SIGTERM` follows (`build_run.rs:3477-3478`). `Running::exit` polls the child for
    a fixed 30 s and then panics with the text above (`build_run.rs:2398-2410`, `:2406`); the
    case then wants the exit within 10 s of that signal (`build_run.rs:3479-3484`).
  - Why the run did not exit, and whether it waited for the ledger when the second signal came:
    not read (the signal handling of `build run` was not read).
- `conductor-cli::dashboard_swr only_one_recompute_runs_at_a_time`
  (`crates/conductor/tests/dashboard_swr.rs:326`) failed with "eight requests, one recompute:
  left 1, right 2".
  - The case reads the recompute counter once, right after the eight requests return, with no
    wait (`dashboard_swr.rs:344-348`). Left 1 is the count from before the write, so the
    recompute had not started yet.
  - The case above it waits for the same counter up to `SLOW` (3 s, `dashboard_swr.rs:36`), with
    a comment that on a loaded runner the recompute may begin after the stale answer was
    served (`dashboard_swr.rs:311-318`).

## Acceptance

- `a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger` sends its second signal only
  once the run is seen waiting for the ledger, and waits on the run's exit, with no fixed sleep
  standing for a state.
- `only_one_recompute_runs_at_a_time` waits on the recompute counter to reach one more than
  before, bounded, and still fails when two recomputes start.
- Both cases pass in a run of every test with a full build pool.

## Notes

- `build_run.rs` holds three more fixed 300 ms sleeps (`:1518`, `:1569`, `:2532`); whether each
  stands for a state the case could observe: not read.
