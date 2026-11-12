---
format: aep.planning-md/3
id: review-result:pod-checkouts-security-round-3-final
kind: review-result
status: active
title: Committed fix of round 3, security review last pass
relations:
- reviews: story:pod-source-checkouts
revision: 1
---
approve

Recorder's note: the verdict word above is the recorder's reading of a pass with no finding; the reviewer's own text follows. The reviewed commit is `b9cdce88d`.

All three points are confirmed; I found nothing new to report. On `b9cdce88d`, `--test sec_pod_checkouts` ran `13 tests run: 13 passed, 0 skipped`, exit 0. I changed no file in the tree and wrote no temporary test file this time.

1. **Confirmed.** `a_process_a_fetch_detaches_is_not_stopped_when_the_snapshot_ends` matches my scratch `recheck-case.rs` once whitespace is removed, and it passes. My pipes case and the implementor's detached case pass too.

2. **Confirmed, with one small window.** The start of the walk is safe: it begins with the snapshot's own children, which it never reaps, so their pids cannot be reused. It then checks a process's group, stops it, and checks the group again before killing it (`collect/mod.rs:766`, kill at `:783`).
   - The window: `SIGSTOP` takes effect asynchronously, so a process can still run for a few microseconds after the second check. If it calls `setsid` just then, it is killed although it left the group. If a parent not yet stopped reaps a child in that moment and the pid is reused, the reuse is checked against the group first. For it to hit anything, the reused pid would have to land in the snapshot's group: a snapshot descendant, or, for a hand-run snapshot in a shell without job control, the shell's group.
   - In practice: git's daemonize (fork, then `setsid`) happens microseconds after a fetch, and all fetches are over long before the snapshot ends, so nothing real lands in that window.

3. **B, C and D still hold.**
   - B: the guard is dropped on return, on `?` and on unwind (`collect/mod.rs:754`; no `panic = "abort"` anywhere).
   - C: only the snapshot's own descendants are walked, only processes in its group are signalled, and only `snapshot start-snapshot` gets the guard.
   - D: a killed snapshot is unchanged; its starter stops the whole group.
   - Shielding: a leftover can protect itself and the processes it starts later by leaving the group (`setsid` or `setpgid`). A child it forked *before* leaving stays in the group but is not walked, so it survives a snapshot that ends normally.
   - That is no worse than any earlier state. At `c09f43ae0` such a child always survived. At the original base, only a command that hit its own bound had its group stopped. A command's own bound still kills the whole tree under it, whatever groups they are in (`collect/mod.rs:804`). The commit describes this behaviour, and no real git, gh or glab helper forks a child before `setsid`.

Written outside the worktree (all under `<the review scratch directory>/`): `final.log`, `c1.txt`, `c2.txt`.

```findings
[]
```
