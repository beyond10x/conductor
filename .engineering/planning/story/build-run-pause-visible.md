---
format: aep.planning-md/3
id: story:build-run-pause-visible
kind: story
status: draft
title: A build paused for disk is frozen without interrupting its syscalls, and says it was paused
revision: 1
---
# Story: build-run-pause-visible

## Outcome

A test that reads a socket with a timeout does not turn red because conductor paused its build for disk. When a run was paused, its exit says so, so a red run during a pause is recognised as one.

## Context

A controller found on 2026-10-16 that a socket read with a timeout was interrupted (EINTR) and taken as a failure, giving an intermittent 502 in a gate. A SIGSTOP and SIGCONT of the test process reproduced it 4 times in 10.

The disk watchdog in `conductor build run` pauses the run's process group with SIGSTOP and resumes it with SIGCONT (crates/conductor/src/build.rs:37-38 and 834-838). It counts pauses (`paused`), but prints only the pause line, not a summary at exit.

## Acceptance

- When the run has its own cgroup (the systemd provider), the watchdog freezes it with `cgroup.freeze` (systemd `freeze`/`thaw`), which interrupts no syscall. Without one, it keeps SIGSTOP.
- At exit, a run that was paused prints `conductor: build run: paused <n> times for <s>s during this run; a failure may come from the pause` on standard error, and records the count on its grant.
- Tests:
  - a fake cgroup path: freeze and thaw are written, and no signal is sent;
  - SIGSTOP fallback: the exit line names the count.

## Out of Scope

Making tests themselves robust to EINTR; that is each repository's.

## Ambiguities

None.
