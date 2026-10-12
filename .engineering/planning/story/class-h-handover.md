---
format: aep.planning-md/3
id: story:class-h-handover
kind: story
status: implemented
title: A class-H call is handed to the operator as a ready command, never attempted
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_hook.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T12:33:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-12T12:33:14Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-12T12:33:21Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

On 2026-10-12 the operator authorized conductor to update an organization secret. Conductor tried
`gh secret set`; the guard denied it with a message that points GitHub writes to the bot route;
conductor tried the bot route, which refused; it then explained and retried once more. The operator
got the command to run only after three further messages of their own, about three minutes later
(conductor's transcript and the instance's dispatch records, 2026-10-12).

Two causes: the guard's denial for a credential write sends conductor to the bot route, which is
for repository writes, not for an organization's secrets (a credential is class H whatever route
carries it); and the profile does not say that a class-H item is handed over as a ready command at
once, even when the operator says "you do it".

## Acceptance

- The guard's denial of `gh secret`, `gh auth` and `gh ssh-key` writes says the call is class H, the
  operator's hands, and names no bot route; a test in `tests/guard_hook.rs` holds the text.
- `.agents/conductor.md` § class H: an operator's authorization does not make a class-H call
  conductor's; its first answer puts the exact command on the operator's clipboard and in the reply
  as a `! ` line, and conductor attempts no route of its own.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_hook.rs`, `.agents/conductor.md`.
