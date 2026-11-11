---
format: aep.planning-md/3
id: story:role-need-line-is-sendable
kind: story
status: draft
title: Every message line a role text spells out is accepted by message send, and a NEED for conductor reaches one named reader
tags:
- from-finding
revision: 1
---
## Why

Found by the adversary passes over unit U1 of wave 51 (`review-result:w51-u1-adversary-1`,
finding F2; `review-result:w51-u1-adversary-2`, finding F4). Both were there before the wave.

- The controller role teaches one NEED line literally (`crates/conductor/roles/controller.md`
  line 85, the worktree workspace that is not activated). That line has one tag field;
  `conductor message send` requires two (`crates/conductor/src/message.rs` about 228) and
  refuses it. A controller that follows its role text cannot send the one NEED the text spells
  out. `tests/profiles.rs` about 421 pins the role text as a constant, so no test sends it.
- A NEED whose first word is `conductor` is meant for the supervisor. In an instance whose
  checkouts root holds a checkout named `conductor`, the mailbox relays it to that repository's
  controller, or keeps it for that controller's next start
  (`crates/conductor/src/run/mailbox.rs` about 404). The supervisor still finds the file in the
  mailbox. No instance is shown to have such a checkout.

## Acceptance

- Every message line a role text spells out literally is accepted by `conductor message send`
  as written; a test sends each one.
- The docs say which of the two a NEED for `conductor` reaches when a checkout has that name,
  and a test holds it.
