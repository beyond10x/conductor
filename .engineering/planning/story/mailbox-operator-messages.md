---
format: aep.planning-md/3
id: story:mailbox-operator-messages
kind: story
status: draft
title: The operator leaves conductor an instruction from outside its session
relations:
- decomposes: epic:host-bridge
revision: 2
---
# Story: mailbox-operator-messages

## Outcome

A process outside any session, such as the operator's desktop UI, can leave conductor a free-form instruction from the operator. Conductor reads it at the start of its next turn and answers it, and the sender can read the answer and the status without typing into conductor's session.

## Context

Requested by the operator through the loom-canvas session (2026-10-17). The specification models `Transport: Mailbox` (spec/domains/dispatch.yaml), but no command reads or writes it (docs/design/conductor.md § 3).

Shapes decided by conductor-dev, so the UI can bind to them:
- `conductor message send --to conductor --from <label> (--text <text> | --text-file <path>) [--format text|json]` prints the new message id `MSG-<date>-<n>`.
- `conductor message inbox --to conductor [--status unread|read|answered|all] [--format text|json]`.
- `conductor message ack --message-id <id> [--reply <text> | --reply-file <path>]` sets `read`, or `answered` when it carries a reply.
- `conductor message show --message-id <id> [--format text|json]` returns status, the time of each change, and the reply.

The mailbox is the operator's channel. The guard denies `message send --to conductor` to every session, so a session never speaks as the operator; sessions keep SendMessage. As with `record-operator-decision`, any process outside a session that runs as the operator's user can write the mailbox. The threat model in design § 3 says so.

## Acceptance

1. Spec: entity `Message` (id, to, from, body, status, sent_at, read_at, answered_at, reply) and the four commands above in `spec/domains/dispatch.yaml`, validated with ess before any code.
2. `send` and `show` round-trip, and `inbox --status unread` lists only unread messages. Tests.
3. `ack` moves unread to read, or to answered with a reply. An ack of an answered message refuses `AlreadyAnswered`. Tests.
4. The guard denies `conductor message send --to conductor` to conductor, controllers and workers, and allows `inbox`, `ack` and `show` to conductor. Test in `guard_rules`.
5. The conductor profile reads `message inbox --status unread` at the start of every turn. It treats each message as the operator's words, acts on it like a chat message, and acks it with its answer. Design § 3 says that the latency is at most conductor's wake-up interval.
6. Text and JSON output are documented in `website/docs` with one example the UI can bind to.

## Out of Scope

Waking a running session on send. Messages to controllers. Delivery of `[DECISION]` lines (story:decision-relay-recorded-answers).

## Ambiguities

None.
