---
format: aep.planning-md/3
id: story:session-handover-draft
kind: story
status: draft
title: A hand-over is drafted for any session from its transcript and trees, without a model
relations:
- decomposes: epic:usage-continuity
revision: 1
---
## Why

The operator, 2026-10-14, after a usage-limit stop: "when you just restart all sessions under your
control then we have cache churn and big usage again ... is there a way to let them continue from
a handoff, which you can create now by looking at their session and what they did recently". After
a reset the prompt cache is cold, so resuming a session re-sends its whole context (100-300k
tokens each); a fresh session reading a short hand-over costs a fraction of that.

## Acceptance

- `conductor session handover <name|id>` drafts a hand-over for any session without a model: its
  working directory, role and dispatch; the git state of its checkout and of every managed tree it
  touched (branch, head, ahead/behind, uncommitted files); its last task list; its last five
  assistant messages, each cut to 400 characters; failed tool calls since its last message to
  conductor; and the open dispatch from the store. It reads the session's transcript and never
  prints an email, token or secret (the redaction of `story:disk-item-redacted` applies). Tests
  over a fixture transcript and fixture trees hold each section and the redaction.
- It writes the draft to the state directory and prints its path; the draft is under 4,000 words
  whatever the transcript's length (a test with a large fixture holds it).

## Scope

`spec/domains/session.yaml`, `crates/conductor-model`, `crates/conductor/src/cli.rs`,
`crates/conductor/src/session/handover.rs` (new), `crates/conductor/tests/session_handover.rs` (new).
