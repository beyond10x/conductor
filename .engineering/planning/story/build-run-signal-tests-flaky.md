---
format: aep.planning-md/3
id: story:build-run-signal-tests-flaky
kind: story
status: implemented
title: The build-run signal tests never time out at random
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T15:45:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T15:45:11Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-16T15:45:12Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: build-run-signal-tests-flaky

## Outcome

`sig{int,term,hup}_withdraws_a_queued_request` in crates/conductor/tests/build_run.rs pass on every run, so a red gate always means a real defect.

## Context

On v0.15.0 code these cases timed out at random while waiting for the request to queue: 3 of 36, then 1 of 36, then 0 of 36 on 2026-10-16. Two of them also failed once in the onboarding-fix unit's base run. The test holds the head's run lock in the binary's host directory and waits with a fixed deadline, under host load of 20 or more.

## Acceptance

- The cases wait on a condition the binary signals: a line the queued run prints, or the grant row's state polled with a deadline that scales with load. They never use a fixed sleep.
- 50 consecutive runs of the three cases pass under `nice -n 19` at host load of 20 or more, recorded as evidence.

## Out of Scope

The signal handling itself.

## Ambiguities

None.
