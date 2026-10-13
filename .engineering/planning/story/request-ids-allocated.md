---
format: aep.planning-md/3
id: story:request-ids-allocated
kind: story
status: draft
title: A controller's request id is allocated, and a reused id is refused
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/message.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: crates/conductor/tests/message_ids.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: spec/scenarios/receive-message-reused-id.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

On 2026-10-13 a controller's successor session sent a cross-repository request under the id its
predecessor had already used for a different need; conductor logged the second one by hand under a
suffixed id. An answer can then be matched to the wrong request. Decision ids are allocated by the
record commands; request ids are typed by each controller.

## Acceptance

- Spec first: a `NextRequestId` query (repository, kind `need|decision-request|resource`) and an
  error `RequestIdReused` on `ReceiveMessage` are declared in `spec/domains/dispatch.yaml`; a
  scenario `spec/scenarios/receive-message-reused-id.yaml` receives two messages with one id and
  different first lines and the second is refused with `RequestIdReused`; `ess verify conform run
  --target interpreted` passes it.
- `conductor message next-id --repo <r> --kind need` prints an id free in the store and in
  `<records>/dispatches/`, and two calls in a row print two different ids; a test in
  `crates/conductor/tests/message_ids.rs` holds it.
- `message receive-message` with an id already recorded under a different first line exits 1 and
  names `RequestIdReused` and the next free id on standard error; the same id with the same first
  line is a duplicate and records nothing (`crates/conductor/tests/message_ids.rs`).
- The guard allows `conductor message next-id` to a controller; a case in
  `crates/conductor/tests/guard_rules.rs` holds it. `.agents/repo-controller.md` tells a controller
  to take each request id from `message next-id`; `crates/conductor/tests/profiles.rs` holds the
  sentence.

## Notes

- After `story:records-from-commands`: messages reach the store through the command only once
  that story lands.
- Shares `crates/conductor/src/guard/rules.rs` and `crates/conductor/tests/guard_rules.rs` with
  `story:guard-write-targets` and `story:rules-show`; one at a time on those files.

## Scope

`spec/domains/dispatch.yaml`, `spec/scenarios/receive-message-reused-id.yaml` (new),
`crates/conductor-model`, `crates/conductor/src/message.rs`, `crates/conductor/src/cli.rs`,
`crates/conductor/cli.yaml`, `crates/conductor/src/guard/rules.rs`,
`crates/conductor/tests/message_ids.rs` (new), `crates/conductor/tests/guard_rules.rs`,
`.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`,
`website/docs/reference/cli.md`.
