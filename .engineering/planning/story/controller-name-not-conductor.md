---
format: aep.planning-md/3
id: story:controller-name-not-conductor
kind: story
status: active
title: A repository controller's session name never equals the conductor session's
summary: Messages to conductor reach conductor only
refs:
- provider: github
  reference: beyond10x/conductor#220
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/broker.rs
- confidence: cited
  path: crates/conductor/src/collect/sessions.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/dashboard/overview.rs
- confidence: cited
  path: crates/conductor/src/dashboard/sessions.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:33:50Z", actor: "human:unknown", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-19T18:33:51Z", actor: "human:unknown", revision: 6}
---
## Why

Observed 2026-11-18: the repository controller of the `conductor` repository is named
`<session_prefix>-conductor` (controller rule `<session_prefix>-<repo>`), the same name as the
instance's conductor session (`conductor.session_name`). Messages sent `--to conductor` by other
controllers were delivered to the repository controller, and its own reports came back to it, so
no report reached the director.

## Outcome

- `config validate` and `spawn` refuse a controller session name equal to `conductor.session_name`
  (or derive a distinct one, e.g. `<prefix>-repo-conductor`), and `message send --to conductor`
  never delivers to a repository controller.

## Acceptance

- a test that spawning a controller for a repository named `conductor` gets a name distinct from the
  conductor session, and a message to `conductor` lands in conductor's mailbox only.

## Seen again (2026-11-18)

A `[NEED conductor …]` the controller of the repository named `conductor` sent with
`conductor message send --to conductor --from <prefix>-conductor` came back into that controller's own
session as a new message: sender and addressee are one session name, so the controller cannot tell a
relay of its own message from conductor's answer.

## Done in wave 59 (issue #220)

Under a session prefix the director's derived session name is `<session_prefix>-director`,
whatever name the file writes the role under (`crates/conductor/src/config.rs`
`role_session_name`, `session_of_role`); the old `<session_prefix>-conductor` reads as the
director's for this release only while no checkout of a repository `conductor` lies under the
checkouts root (`config::director_names`), in session start and stop, the sessions collector, the
dashboard and the broker. A file that still writes the old derived name validates. An instance
without a prefix keeps today's names. Tests: `tests/config.rs`, `tests/taskfile.rs`,
`tests/watch.rs`, `tests/spawn.rs` updated to the new name.
