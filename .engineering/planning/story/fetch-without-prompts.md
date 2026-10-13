---
format: aep.planning-md/3
id: story:fetch-without-prompts
kind: story
status: draft
title: Collector git never prompts and one failed fetch does not fail the snapshot
relations:
- depends_on: story:snapshot-member-failures
scope:
- confidence: cited
  path: crates/conductor/src/collect/blockers.rs
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
- confidence: cited
  path: crates/conductor/src/collect/repositories.rs
- confidence: cited
  path: crates/conductor/tests/collect_stores.rs
revision: 7
---
## Why

A team's snapshots failed ("There was a bug with snapshoting", 2026-10-13). Reproduced on v0.6.0
(v0.4.0 has the same code): the blockers collector runs `git fetch --quiet origin` in every
member's checkout (`crates/conductor/src/collect/blockers.rs:157`, run through `run()` at
`:607-614`) without `GIT_TERMINAL_PROMPT=0` or ssh batch mode; the specifications collector runs its
git through the same `run()` (`specifications.rs:656`). The repositories collector sets
`GIT_TERMINAL_PROMPT=0` (`crates/conductor/src/collect/repositories.rs:1310`) but no batch mode.
One `origin` that cannot be reached without a prompt fails the whole snapshot: an unknown host in
0.5 s (`exited 128: ssh: Could not resolve hostname …`), an unreachable host without batch mode
after 240 s (`git gave no answer within 120 s`, tried twice), and an HTTPS login prompt shown in the
user's terminal, unanswered, for 240 s.

## Acceptance

- One function in `crates/conductor/src/collect/mod.rs` sets the git environment, and every git
  command of the repositories, blockers and specifications collectors runs with it:
  `GIT_TERMINAL_PROMPT=0` and `GIT_SSH_COMMAND` with `-o BatchMode=yes -o ConnectTimeout=10`.
- Tests in `crates/conductor/tests/collect_stores.rs`, with a fake `git` on `PATH` that writes its
  arguments and environment to a file:
  - (a) a snapshot with the repositories, blockers and specifications collectors: every recorded
    git command of each of the three has `GIT_TERMINAL_PROMPT=0` and a `GIT_SSH_COMMAND` holding
    `-o BatchMode=yes -o ConnectTimeout=10`;
  - (b) with `GIT_SSH_COMMAND="ssh -i /tmp/key"` set by the caller, every recorded command has
    `ssh -i /tmp/key -o BatchMode=yes -o ConnectTimeout=10`;
  - (c) a fetch that hangs (the fake `git` sleeps) past the collector bound, shortened through the
    library as `tests/collector_bounds.rs` does: the snapshot is `Completed` and
    `conductor snapshot member-failures` prints that member with `blockers` and a first line saying
    it timed out (`gave no answer within <n> s`, the text `collect::run` gives), the record
    `story:snapshot-member-failures` declares;
  - (d) a member whose fetch fails: it is recorded as a member failure (`blockers`, the fetch's
    first error line) and its blockers are still read from its checkout as last fetched — the
    blocker row its `refs/remotes/origin/main` holds is in the snapshot, as
    `story:snapshot-member-failures` states for the same case.

## Notes

- After `story:snapshot-member-failures` (the failure record and view, spec first; both edit
  `collect/blockers.rs`) and before `story:snapshot-duration` (both edit `collect/mod.rs` and
  `blockers.rs`).
- Shares `crates/conductor/tests/collect_stores.rs` with `story:snapshot-member-failures`, which
  rewrites its two fails-twice cases first; this story adds (a)-(d) after it.
- `specifications.rs` is covered through `blockers::run` and is not edited.

## Scope

`crates/conductor/src/collect/mod.rs`, `crates/conductor/src/collect/blockers.rs`,
`crates/conductor/src/collect/repositories.rs`, `crates/conductor/tests/collect_stores.rs`.
