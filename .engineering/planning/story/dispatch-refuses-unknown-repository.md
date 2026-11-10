---
format: aep.planning-md/3
id: story:dispatch-refuses-unknown-repository
kind: story
status: implemented
title: dispatch send-dispatch refuses a repository no snapshot or workspace names
scope:
- confidence: inferred
  path: crates/conductor/src/dispatch.rs
- confidence: inferred
  path: crates/conductor/tests/dispatch.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:01Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:02Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-10T07:51:25Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from the first conductor 1.0.1 run of a trial instance. `conductor dispatch send-dispatch --repository checks` was accepted though no snapshot
names a repository `checks`, and nothing warned; the dispatch then started nothing (see
story:checkout-name-differs-from-project).

## Acceptance

- `dispatch send-dispatch` refuses a repository that neither the newest complete snapshot nor the
  workspace names, with a message naming the repositories it knows; a test holds the refusal and
  its exit code.
