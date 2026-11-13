---
format: aep.planning-md/3
id: story:build-run-second-signal-on-shared-host
kind: story
status: draft
title: The case of a second signal to a waiting build run fails on a shared host for a reason nobody has read
relations:
- informed_by: story:build-run-waiting-ends-on-signal
scope:
- confidence: cited
  path: crates/conductor/src/build.rs
- confidence: cited
  path: crates/conductor/tests/build_run.rs
revision: 2
---
## Why

`build_run::a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger`
(`crates/conductor/tests/build_run.rs`) failed on a shared development host on 2026-11-13, on two
branches that change none of `src/build.rs`, `src/host.rs` and `tests/build_run.rs`, in the whole
package's run and alone, three of three runs:

```
FAIL [  31.603s] (1/1) conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger
thread 'a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger' panicked at crates/conductor/tests/build_run.rs:2404:13:
build run did not exit within 30 seconds of the signal
Summary [  31.688s] 1 test run: 0 passed, 1 failed, 65 skipped
```

- One of the two trees passed the same case in a whole-package run earlier the same day
  (`2268 tests run: 2268 passed`), before any change to those files.
- The hosted check of the same commits passes it.
- The runs went through `conductor build run`, so the case's own `build run` is a child of
  another one. Whether that, the host's load (the build pool was full) or something else on the
  host decides it is not known.

The case holds a behaviour operators rely on: a second stopping signal ends a `build run` that
waits for the ledger. A case that fails for a reason nobody has read hides the day it fails for
the real one.

## What

- Find why the run does not exit within the bound on that host: what the second signal reaches,
  and what the run waits on.
- Then either the code ends the run in that state too, or the case stops depending on what
  differed (an inherited variable, the outer run's process group, a timing bound).

## Acceptance

- The cause is written in this story with the output that shows it.
- The case passes ten of ten runs alone inside a `conductor build run` on a host whose build pool
  is full, or the story records why that cannot be held and what the case asserts instead.
- No bound is raised without the reading that shows time was the cause.
