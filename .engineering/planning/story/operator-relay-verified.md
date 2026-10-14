---
format: aep.planning-md/3
id: story:operator-relay-verified
kind: story
status: draft
title: An operator message from any of the operator's sessions reaches conductor, trusted by its verified sender
relations:
- decomposes: epic:loop-without-handwork
- depends_on: story:mailbox
revision: 1
---
## Why

On 2026-10-14 an operator message sent from one of the operator's own Claude sessions running in
the normal (prompting) permission mode reached neither conductor nor the gateway twice: a message
between sessions of different permission modes is held for approval at the receiver, nobody
approves it in a background session, and it expires. The operator chose, 2026-10-14, how such a
relay is trusted: "Verified sender".

## Acceptance

- `conductor mail post --from-operator <text|file>` writes the message to conductor's mailbox
  (`story:mailbox`) with the sending process's evidence: the chain of parent processes up to a
  `claude` process, whether that process is interactive (no `--bg`, a terminal) or runs in the
  remote-control service's cgroup, and its session id; it is accepted as the operator's only when
  that chain ends in an interactive Claude session of the user or the remote-control gateway, and
  is otherwise recorded as a request from that session; tests with fixture process tables hold
  each case.
- The guard refuses `conductor mail post --from-operator` to conductor, conductor-dev and every
  controller; a guard case holds it.
- `.agents/conductor.md` says a verified relay counts as the operator's words, quoted and recorded
  as theirs, and an unverified one is a request conductor decides as class C or O itself; a profile
  test holds it.

## Scope

`spec/domains/dispatch.yaml`, `crates/conductor-model`, `crates/conductor/src/mail.rs`,
`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/`, `.agents/conductor.md`.
