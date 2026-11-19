---
format: aep.planning-md/3
id: story:controller-name-not-conductor
kind: story
status: draft
title: A repository controller's session name never equals the conductor session's
summary: Messages to conductor reach conductor only
relations:
- decomposes: epic:architecture-next
revision: 2
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
