---
format: aep.planning-md/3
id: story:build-run-exit-on-failed-tests
kind: story
status: archived
title: build run never reports a failed test run as exit 0
summary: a gate reading only the wrapper's exit took red for green
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-11-07T16:19:02Z", actor: "human:timo", revision: 3}
---
## Why

Observed 2026-11-07 on a second instance: a controller reported that `conductor build run` exited 0 on a run where cargo printed `test result: FAILED` (a red-first run of an implementor). A gate that reads only the wrapper's exit takes red for green. The report names no command line; `crates/conductor/src/build.rs:1322` passes the child's code through, so the cause may be a pipe after `build run` (`| tail`) in the caller rather than the wrapper. Unverified.

## Acceptance

- A test runs `conductor build run -- cargo test` over a crate with one failing test and asserts exit non-zero (101).
- When the run is a pipeline (`build run -- sh -c "cargo test | tail"`), `build run` prints one line on standard error naming that the exit code is the pipeline's last command, or the controller profile tells controllers to run `set -o pipefail`; the chosen fix is named on the story.
- A run whose output contains `test result: FAILED` and whose exit is 0 is recorded on the grant as `exit 0 with a failed test result`.

## Withdrawn (2026-11-07)

Not a conductor defect. The controller ran `conductor build run --reserve auto -- cargo test ... 2>&1 | tail -30`, and the 0 was the exit of `tail` (host supervisor, from the implementor transcript). `build run` passes the child's code through (`crates/conductor/src/build.rs:1322`).
