---
format: aep.planning-md/3
id: story:decision-options-spellings
kind: story
status: implemented
title: A decision request's lettered options are read in the common spellings
scope:
- confidence: inferred
  path: crates/conductor/src/decide/handle.rs
- confidence: inferred
  path: crates/conductor/src/run/mailbox.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:04Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:04Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-10T07:51:26Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from conductor 1.0.1. A lead's decision request
was refused with "lists no lettered option: one line each, `A: <label> (cost: <cost>)`" though it
wrote `A. <label>. Cost: …`; the refusal could not be delivered either
(story:deliver-reaches-bg-session).

## Acceptance

- A decision request whose options are written `A. <label>. Cost: <cost>` or `A) <label> — cost
  <cost>` is read as lettered options with their costs; a test holds each spelling.
