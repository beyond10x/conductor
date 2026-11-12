---
format: aep.planning-md/3
id: review-result:pod-checkouts-security-round-3-recheck
kind: review-result
status: active
title: Fix of round 3's note, security re-check
relations:
- reviews: story:pod-source-checkouts
revision: 1
---
needs-change

Recorder's note: the verdict word above is the recorder's reading of one confirmed note with a failing case; the reviewer's own verdict line follows. The reviewed text is the uncommitted change over `c09f43ae0` that became the commit "Stop what a snapshot's commands left behind when the snapshot ends".

**My round 3 note is closed.** The fix makes the snapshot a child subreaper (`collect/mod.rs:739`) and stops everything still under it when `Leftovers` drops (`collect/mod.rs:748`). It brings one new note: a process a `git fetch` deliberately detaches is now killed when the snapshot ends.

unit: round 3 re-check of the uncommitted delta in `collect/mod.rs`, `snapshot.rs` and `tests/sec_pod_checkouts.rs` on `c09f43ae0`
verdict: one new note (introduced); nothing blocks
cases: my 2 re-check runs: 3 cases (2 passed, 1 failed), then the corrected new case alone (red). I ran no full gate this round because your gate holds the tree.
wrote-outside-worktree: 9 paths (listed below); the tree is unchanged by me, and my temporary test file `sec3_recheck_reviewer.rs` is removed
needs-coordinator: whether killing git's detached maintenance at the snapshot's end is acceptable

**Claims**
- **A. Holds.** My case is byte-identical to what I wrote (`cmp` against my scratch copy). It passes, as does the implementor's `a_helper_a_timed_out_clone_detached_does_not_outlive_the_snapshot`.
- **B. Holds.**
  - `_leftovers` is dropped on a normal return and on every early `?` return of `start_snapshot`.
  - No profile sets `panic = "abort"`, so a panic unwinds through the same drop.
  - Neither `snapshot.rs` nor `collect/` calls `process::exit`.
  - Not covered: the snapshot itself being killed by a signal. That is claim D's path.
- **C. Holds.**
  - The drop walks only this process's own children, and a subreaper only adopts its own descendants, so nothing outside the snapshot can be reached.
  - Adopted children are never reaped, so their pids stay held until the signal. Grandchildren keep the same microsecond `SIGSTOP` window as before.
  - Only `snapshot start-snapshot` calls `in_group`, and only the binary's `main` dispatches it (`cli.rs:266`). The runtime and the daemon do not get this behaviour.
  - A snapshot run by hand never touches the shell's group or terminal. On a workstation it does now kill helpers its own git started that are meant to persist, such as an ssh ControlMaster with ControlPersist, or a `credential-cache` daemon.
  - As process 1 of a container, setting the subreaper is a no-op; it stops the container's orphans at its end, which the container's exit would do anyway.
- **D. Holds.** A killed snapshot behaves as at `c09f43ae0`: the outer `killpg` takes its group, and processes it adopted in another session go to the next reaper, as before.
- **E. Does not hold** (red case `a_process_a_fetch_detaches_is_not_stopped_when_the_snapshot_ends`, scratch file `recheck-case.rs`):
  - A process a fetch of the pass detaches, standing in for git's daemonize (`setsid -f`, streams on `/dev/null`), "was stopped when the snapshot ended". The pass took 2.5 s, so it was killed, not timed out.
  - Real git 2.56 does this: `GIT_TRACE` of `git fetch --quiet origin` shows `git maintenance run --auto --quiet --detach` (probe `autogc.sh`). Once the thresholds are met (default 6700 loose objects or 50 packs), gc detaches into its own session. It is now adopted by the snapshot and killed when the snapshot ends, where at `c09f43ae0` it ran to its end.
  - In a long-lived pod this means a large repository's repack is killed again on every pass. It can leave temporary packs and a stale `packed-refs.lock`, and a failed gc writes `gc.log`, which stops auto gc for a day.
  - Clone, fetch and the pod's credential helper (a shell function, no daemon) are not broken; repository maintenance and disk use degrade.
  - Two possible fixes: exempt processes that started a new session from the end-of-snapshot stop, or have the collectors' fetch pass `--no-auto-maintenance` and leave maintenance to its own step.

**Written outside the worktree**, all under `<the review scratch directory>/`:
- `recheck-case.rs`: the case, ready to append to `tests/sec_pod_checkouts.rs`
- `autogc.sh`, `probe-autogc/`, `recheck.log`, `recheck2.log`
- The red case's directory under `<the shared build target>/tmp/sec_pod_checkouts/fetch-detached`

```findings
[
{"file":"crates/conductor/src/collect/mod.rs","line":748,"category":"concurrency","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the snapshot as child subreaper stops every adopted process when it ends, including the maintenance git fetch detaches into its own session, so a long repack in a pod is killed at every pass where before it ran to its end"}
]
```
