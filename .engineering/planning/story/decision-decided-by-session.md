---
format: aep.planning-md/3
id: story:decision-decided-by-session
kind: story
status: draft
title: A conductor decision records the session that took it, with no prompt hash
summary: --decider-name without --prompt-sha256 is refused, so decisions lose their decider
revision: 2
---
## Why

Reported 2026-11-07 from a running instance: `conductor decision record-conductor-decision
--decider-name <session>` is refused, `--decider-name is given without --prompt-sha256; give
both`. A session deciding with the operator's authority has no prompt hash, so five decisions
were recorded with no decider at all.

Verified in the code: `crates/conductor/src/decide.rs:333` refuses a decider name without a
prompt hash; the pair is the `Decider` adapter's evidence (`story:decide-model-adapter`), a
bounded model call, not a session.

## Acceptance

- A decision taken by a session is recorded with who took it: `--decider-name` alone is accepted
  and recorded as a session decider, or a separate `--decided-by <session>` field is recorded and
  shown by the decision views.
- `--prompt-sha256` without `--decider-name` is still refused.
- A test records one decision with a session decider and reads it back from the decision view.
