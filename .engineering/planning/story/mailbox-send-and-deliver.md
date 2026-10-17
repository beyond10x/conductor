---
format: aep.planning-md/3
id: story:mailbox-send-and-deliver
kind: story
status: draft
title: conductor message send records a controller's message through the mailbox; session deliver types text into a running session
relations:
- decomposes: epic:conductor-run
revision: 1
---
## Why

Controllers report with the harness's `SendMessage` to a conductor session, and the session
relays by the same tool. With no conductor session (`docs/design/conductor-run.md` § 3) a report
needs a command that records it, and the daemon needs a way to put text in front of a session. The
spec names a `Mailbox` transport (`spec/domains/dispatch.yaml:40-42`, `body_path` :340) that nothing
writes or reads; the tmux launcher can type into a session (`crates/conductor/src/confine/systemd.rs:456-480`,
`type_text`), used only for the first prompt today.

## Outcome

`conductor message send --to conductor --from <session> --text <file|->` records a controller's
message through the mailbox; `conductor session deliver --name <session> --text <file|->` types text
into a running session and records it. The first-line grammar gains the `[HANDOVER <repo>] …` form
the design names.

## Acceptance

1. Spec first: `spec/domains/dispatch.yaml` declares the mailbox send (a `ReceiveMessage` with
   transport `Mailbox` and a `body_path` under `<state>/mailbox/<recipient>/`) and
   `spec/domains/session.yaml` declares `DeliverMessage` and its `MessageDelivered` event with
   session name, first line, body path and time; a scenario sends and delivers one message;
   `ess specify validate --path spec` exits 0; `task regen` leaves `crates/conductor-model` as
   committed.
2. `conductor message send --to conductor --from <session> --text <file|->`: parses the first line
   with `message::read`; an unparsed line prints `FORMAT`, exits 1 and writes nothing; a parsed one
   writes `<state>/mailbox/conductor/<ulid>.md`, records the message as Received with transport
   `Mailbox` and that path, and appends the `message-received` line. Tests in
   `crates/conductor/tests/mailbox.rs`: one per form (REPORT, NEED, DECISION-REQUEST, RESOURCE,
   HANDOVER), one rejection.
3. `[HANDOVER <repo>] context <n>k` and `[HANDOVER <repo>] context shutdown` parse as a form in
   `message.rs` `FORMS`; `[REPORT <repo> <id>] blocked handover <path>` stays a report. Test.
4. `conductor session deliver --name <session> --text <file|->`: under the systemd provider,
   `type_text` into that session's tmux; records `delivered` (session, first line, body path) in the
   store and the dispatch log; exit 1 naming the session when its tmux socket is absent, no retry.
   Test with a fake `tmux` on `PATH` that records its arguments (`crates/conductor/tests/deliver.rs`).
5. Guard: a controller may run `conductor message send` (declared in `conductor_run`'s allowed list,
   `crates/conductor/src/guard/rules.rs`) and may not run `conductor session deliver`; tests in the
   guard suite.
6. `website/docs/reference/**` regenerated (`task docs-generate`); `task docs-check` exits 0.

## Scope

`spec/domains/dispatch.yaml`, `spec/domains/session.yaml`, `spec/scenarios/`, `crates/conductor-model`
(regenerated), `crates/conductor/src/message.rs`, `crates/conductor/src/session.rs`,
`crates/conductor/src/confine/systemd.rs` (`type_text` made callable from `session deliver`),
`crates/conductor/src/guard/rules.rs`, `crates/conductor/cli.yaml` or `spec/components.yaml` (the
command tree), `crates/conductor/tests/mailbox.rs`, `crates/conductor/tests/deliver.rs`,
`website/docs/reference/`.
