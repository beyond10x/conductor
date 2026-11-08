---
format: aep.planning-md/3
id: story:guard-admits-own-pipelines
kind: story
status: implemented
title: The controller guard admits pipeline retry and run of its own repository under forge.route direct
summary: Controllers wait on the supervisor to retry or start their own pipelines
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T19:23:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T19:23:25Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-08T11:25:46Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":2}}}
---
## Why

Under `forge.route: direct` the controller guard (`crates/conductor/src/guard/rules.rs`) admits `glab mr create|update|note|merge` of the session's own repository but denies `glab ci retry` and `glab ci run`, and a pipeline start through `glab api`. On 2026-11-07 three controller NEEDs on two GitLab instances asked the host supervisor to retry or start a pipeline by hand ("controllers cannot start pipelines"); each controller waited until it did.

## Acceptance

- Under `forge.route: direct`, the guard admits `glab ci retry`, `glab ci run` and `glab ci view` of the session's own repository (named with `-R <path>`, or the checkout's own origin when `-R` is absent), and `glab api` POST to `projects/<own project, path-encoded>/pipeline` or `.../pipelines/<id>/retry` and `.../jobs/<id>/retry`.
- The same commands naming another repository stay denied, with the rule's name in the denial.
- Guard tests: each admitted form for the own repository, each denied for another repository, and `-R` absent inside the checkout.
- The guard's latency test (`AGENTS.md` rule 6) still holds.
