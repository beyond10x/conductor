---
format: aep.planning-md/3
id: story:build-run-priority
kind: story
status: draft
title: A red-CI build goes ahead of the host disk queue
relations:
- decomposes: epic:red-ci-ownership
revision: 1
---
## Why

Rules item 125 puts a red-CI fix ahead of any new unit, but the host disk queue is strict arrival
order (`story:build-run-queue`): on 2026-10-14 a fix gate for a red pull request sat at position 6
of 28 while the disk admitted one build at a time (conductor's NEED, 10:3xZ).

## Acceptance

- Spec first: a disk request carries an optional priority reason, `red-ci <pr|main>`, recorded on
  the grant; `conductor build run --red-ci <pr|main>` sets it. The queue orders red-CI requests
  before all others, arrival order within each class; tests in `crates/conductor/tests/build_run.rs`
  hold a red-CI request made last being granted first, and the reason in `host grants`.
- A request may also carry `release` (`conductor build run --release <repo> <version>`): the last
  gate or build before a release or a merge. Order: red-CI, then release, then everything else,
  arrival order within each; a test holds a release request made after unit builds granted before
  them (conductor's report, 2026-10-14 16:14Z: a release's last 6G lane was refused while unit
  builds waited ahead of it).
- The watch prints `red-ci priority used: <repo> <target>` once per grant, so its use can be
  audited; a test holds the line.
- `.agents/repo-controller.md` says a red-CI fix's gate passes `--red-ci <target>` and nothing
  else does; a profile test holds the sentence.

## Scope

`spec/domains/host.yaml`, `crates/conductor-model`, `crates/conductor/src/host.rs`,
`crates/conductor/src/build.rs`, `crates/conductor/src/cli.rs`, `crates/conductor/src/watch.rs`,
`crates/conductor/tests/build_run.rs`, `.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`.
