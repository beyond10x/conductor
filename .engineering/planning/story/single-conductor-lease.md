---
format: aep.planning-md/3
id: story:single-conductor-lease
kind: story
status: implemented
title: conductor session claim refuses a second conductor and takes over a stale holder
relations:
- decomposes: epic:operating-loop-hardening
- depends_on: story:operator-decision-guarded
- depends_on: story:guard-store-migrate
scope:
- confidence: cited
  path: .claude/conductor-settings.json
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: crates/conductor/tests/session.rs
- confidence: cited
  path: spec/domains/session.yaml
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T18:58:20Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "proposed", to: "active", at: "2026-10-12T18:58:20Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "active", to: "implemented", at: "2026-10-12T19:44:37Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":6}}}
---
## Why

On 2026-10-12 two sessions named `conductor` were live: the managed background session in the
records directory and the first conductor session of 2026-10-06, resumed by hand with the parent
directory of the checkouts as its cwd. It came back under the same name, and nothing refused it.
Two conductors can each dispatch, decide, write the records and grant build slots; a peer that
addresses `conductor` gets an ambiguity error or reaches the wrong one (reported by the second
session itself to conductor-dev, 2026-10-12, at the operator's request). `task conductor:start`
refuses only while a session of that name runs, and only for a start through the task; a
`--resume` or a start by hand passes.

## Design

A conductor lease in the instance's state directory: the session id and pid of the one conductor.
A new noun, so it is modelled in `spec/` first (`ess:specifying`), in a new
`spec/domains/session.yaml`: `conductor.session.Lease` (instance, session id, pid, taken at) and
the commands that take, show and release it; `crates/conductor-model` is regenerated.

- `conductor session claim` (new `crates/conductor/src/session.rs`, wired in
  `crates/conductor/src/cli.rs`) takes the lease. A live holder that is not this session refuses
  the claim: the command exits non-zero and its output names the holder's session id and pid and
  tells the session it is a second conductor and must stop.
- A holder whose pid is gone is stale: the next claim takes the lease over and records that it did.
- The conductor role's settings template `.claude/conductor-settings.json` gains a `SessionStart`
  hook that runs `conductor session claim`, so every conductor session started with the instance's
  settings claims the lease.

The guard's enforcement of the lease is story:lease-guard; the watch's report of two live
conductor sessions, which covers a session resumed without the instance's settings, is
story:watch-two-conductors.

## Acceptance

- A session started with the shipped `.claude/conductor-settings.json` takes the lease: a test runs
  that file's `SessionStart` command for two sessions whose pids are alive and sees the first exit 0
  and the second exit non-zero naming the first's session id and pid, then, with the first pid
  gone, sees a third run take the lease and record the takeover; in
  `crates/conductor/tests/session.rs`.

## Notes

- `docs/design/conductor.md` § 7 and § 8 should describe the lease once it exists; a later docs
  edit.
- How a legitimate `task conductor:restart` interacts with a claim while the old pid is still
  exiting is not settled here; the restart task already stops the old session before it starts the
  new one.
- Ordered after story:operator-decision-guarded: both change `spec/domains/`,
  `crates/conductor-model` and `crates/conductor/src/cli.rs`.

## Scope

`spec/domains/session.yaml` (new), `crates/conductor-model`, `crates/conductor/src/session.rs`
(new), `crates/conductor/src/cli.rs`, `.claude/conductor-settings.json`,
`crates/conductor/tests/session.rs` (new).
