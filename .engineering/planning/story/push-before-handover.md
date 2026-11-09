---
format: aep.planning-md/3
id: story:push-before-handover
kind: story
status: active
title: conductor-dev pushes every branch and opens its pull requests before a hand-over
scope:
- confidence: inferred
  path: crates/conductor/roles/maintainer.md
- confidence: inferred
  path: crates/conductor/src/collect/repositories.rs
- confidence: inferred
  path: crates/conductor/src/message.rs
- confidence: inferred
  path: crates/conductor/src/run/handlers.rs
- confidence: inferred
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: crates/conductor/tests/watch.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T16:52:27Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T16:52:27Z", actor: "human:timo", revision: 4}
---
## Why

The operator, 2026-11-07: "this is bad"; "make sure conductor is being modified to have improved over your findings"; "track those changes actually land there and become active too!". At 2026-11-07T16:48Z the product checkout held 52 commits on 7 local branches not on any remote (the oldest from 13:11Z), and the integration branch was 21 commits ahead of `main` with no pull request: a plan board and a config page sat finished while the dashboards ran the previous release. A conductor-dev hand-over named the branches but pushed none.

## Acceptance

- `.agents/conductor-dev.md` says: before a hand-over, push every branch that holds a commit not on a remote and open the pull request of every finished unit; the hand-over names each branch, its last commit and `pushed: yes`.
- The loop's hand-over request to conductor-dev carries the same steps in its text; a test reads them in the delivered text.
- The watch tick writes a line when the product checkout holds a commit not on any remote for longer than 30 min, naming the branch, the count and the oldest commit's age; a test reads the line from a fixture repository.
