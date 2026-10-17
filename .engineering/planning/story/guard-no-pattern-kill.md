---
format: aep.planning-md/3
id: story:guard-no-pattern-kill
kind: story
status: implemented
title: No session kills another's processes by pattern; a stopped build run never hangs on the ledger
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T04:41:36Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-17T04:41:36Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-17T04:41:37Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
# Story: guard-no-pattern-kill

## Outcome

No session on the host can kill another session's processes by pattern, and a `build run` that is stopped never hangs on the ledger lock.

## Context

A conductor NEED (2026-10-17). The root cause was found from transcripts and the journal (scratch evidence in conductor-dev's job tmp, sigterm/):
- At 02:16:38Z and 03:54:34Z on 2026-10-17, a thalamus and then a loom sub-agent ran `pkill -f "conductor build run"`. It killed ess's batch, thalamus's gate and eight mandate builds of other controllers within 125 ms (exit 143/144).
- On 2026-10-16 at 22:43Z, a `pkill -f "xtask conform"` killed a sibling's run.
- After the SIGTERM, thalamus's `build run` blocked in a plain `flock` on `host.lock`, in `record_run_of`/`release` after the command stopped (build.rs:984-1001, host.rs:461-480). Its signal handlers only set a flag, and SA_RESTART restarts the flock, so a second SIGTERM was ignored until it was SIGKILLed.
- `stop-controller` and `spawn` were cleared: they stop only their own unit, and every run lives in its controller's cgroup.

rules.md item 147 is the stopgap.

## Acceptance

1. The guard's PreToolUse rules deny `pkill`, `killall`, and `kill` with a pattern, a name, `-1` or a negative group that the caller did not start. The denial names item 147 and the safe form (TaskStop, or `kill <pid>`). Tests in `crates/conductor/tests/guard_rules.rs` cover each form and one allowed `kill <pid>`.
2. In `build run`'s path after the command stops, the ledger is taken with a bounded try-lock loop (at most 30 s). On timeout or a second stopping signal it exits 128+signal and leaves the grant to `Ledger::sweep`, which already releases a Running grant whose group is gone (`ProcessGroupGone`). A test holds `host.lock`, signals the run twice, and sees it exit within the bound.
3. The profiles (repo-controller, conductor-dev, the worker agents) state item 147's rule, and a profile test holds the sentence.

## Out of Scope

Changing the cgroup layout.

## Ambiguities

None.
