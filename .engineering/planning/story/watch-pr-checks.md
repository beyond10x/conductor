---
format: aep.planning-md/3
id: story:watch-pr-checks
kind: story
status: draft
title: The watch reports when an open pull request's checks have concluded
relations:
- decomposes: epic:loop-without-handwork
- decomposes: epic:red-ci-ownership
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: docs/design/conductor.md
revision: 4
---
## Why

The watch reads only `main` CI (design § 8). On 2026-10-13 controllers waited for their pull
requests' checks themselves: the harness blocked a foreground `sleep N; gh pr checks` seven times,
conductor sent seven "do not poll" messages and relayed results by hand, one controller was silent
31 min waiting on a pull request's CI, and one red pull request went unseen for 3 h 26 min because
its controller had reported blocked and stopped.

Again on 2026-10-17 (conductor's NEED N-20261017-10): in 40 minutes mandate (PR 43) and
engineering-protocols (PR 22) watched their pull requests' CI in a background call, against rules
items 118 and 139. Conductor learns a pull request's result only at its 30-minute check-in.

## Acceptance

- For each open pull request (a GitLab merge request is one) of an Active repository of the newest
  complete snapshot, `conductor watch run` prints `PR checks done: <repo>#<n> <green|red> (head
  <sha>)` once, when all its checks have concluded for that head, and again only for a new head; a
  test in `crates/conductor/tests/watch.rs` with fake `gh` and `glab` answers reads one line for a
  concluded red pull request, none for a pending one, and one again after a new head concludes.
- Pull-request checks are read every `cadence.ci`, like `main` CI; design § 8's probe table has
  the row.
- `.agents/repo-controller.md` says: after `progress PR opened`, work the next unit or report
  `blocked ci <pr>` and end the turn; conductor relays the watch line. `.agents/conductor.md` says
  conductor relays a `PR checks done` line to the controller of that repository. A test in
  `crates/conductor/tests/profiles.rs` holds both sentences.

## Notes

- First of the three watch stories (`story:watch-pr-checks`, `story:watch-goal-health`,
  `story:watch-waits`); all edit `crates/conductor/src/watch.rs` and
  `crates/conductor/tests/watch.rs`, one at a time.
- No new record or command: a watch line is output only.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`, `docs/design/conductor.md`,
`.agents/repo-controller.md`, `.agents/conductor.md`, `crates/conductor/tests/profiles.rs`.
