---
format: aep.planning-md/3
id: story:compact-vs-handover
kind: story
status: draft
title: Controllers compact instead of handing over only if it measurably costs fewer tokens
relations:
- decomposes: epic:controller-health
revision: 1
---
## Why

The operator, 2026-10-14, asked whether controllers should compact from time to time instead of
handing over, and set the condition: "we should only do it if its better, in terms of context and
token spent". Measured that day: a fresh controller pays ~55k of harness on its first turn and ~37k
of start reading by its twentieth, then ~1k per working turn; a hand-over at 200k pays that ~92k
again and loses the session's working memory. Compaction keeps the session but summarizes what it
read, including a profile read with the Read tool.

## Acceptance

- A trial on one controller for one working day each way, measured by the health probe and kept in
  `docs/analysis/compact-vs-handover.md`: (a) hand-over at 200k as today; (b) the controller's
  profile given in its system prompt (as conductor's is) and the controller compacting itself at
  200k with a focus line naming its dispatch, trees and next step. For each: input tokens per merged
  pull request, input tokens per working turn, context right after the reset, turns until the next
  reset, and failed tool calls and rule slips in the 20 turns after a reset.
- Compaction is adopted (profile, `rules.md`, the threshold's action) only if (b) is lower on both
  tokens per merged pull request and context after the reset, with no more slips than (a);
  otherwise the analysis says so and hand-over stays. Either way the story records the numbers.

## Scope

`.agents/repo-controller.md`, `crates/conductor/src/spawn.rs`, `crates/conductor/tests/spawn.rs`,
`docs/analysis/compact-vs-handover.md` (new).
