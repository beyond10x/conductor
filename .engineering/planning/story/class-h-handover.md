---
format: aep.planning-md/3
id: story:class-h-handover
kind: story
status: draft
title: A class-H call is handed to the operator as a ready command, never attempted
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_hook.rs
revision: 2
---
## Why

On 2026-10-12 the operator authorized conductor to refresh the org secret `B10X_GATES_POLICY`
(H-20261011-02) at 11:41:10Z. Conductor tried `gh secret set`; the guard denied it with a message
that points GitHub writes to the bot route; conductor tried the bot route, which stopped with
"protected file unavailable"; it then explained and retried once more. The operator wrote "REFRESH
THE SECRET NOW!" at 11:42:28Z and got the command on the clipboard only at 11:44:01Z, after a third
message of his own. Transcript `8ce08ff1-485f-46a4-a253-ec3b64ac9e33.jsonl`, lines from 11:41:10Z
to 11:44:28Z; records `dispatches/2026-10.jsonl`, `H-20261011-02` "granted-guard-denied".

Two causes: the guard's denial for a credential write sends conductor to the bot route, which is
for repository writes, not for an organization's secrets (the bot refused because the call lacked
`--policy`, rules item 29, but a credential is class H whatever route carries it); and the profile does not say that a class-H item is handed over as a ready command at once, even
when the operator says "you do it".

## Acceptance

- The guard's denial of `gh secret`, `gh auth` and `gh ssh-key` writes says the call is class H, the
  operator's hands, and names no bot route; a test in `tests/guard_hook.rs` holds the text.
- `.agents/conductor.md` § class H: an operator's authorization does not make a class-H call
  conductor's; its first answer puts the exact command on the operator's clipboard and in the reply
  as a `! ` line, and conductor attempts no route of its own.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_hook.rs`, `.agents/conductor.md`.
