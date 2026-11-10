---
format: aep.planning-md/3
id: story:guard-admits-own-mr-auto-merge
kind: story
status: implemented
title: Under forge.route direct the guard admits setting merge-on-green on the own repository's merge request
scope:
- confidence: inferred
  path: crates/conductor/src/guard/rules.rs
- confidence: inferred
  path: crates/conductor/tests/guard_rules.rs
- confidence: inferred
  path: website/docs/security/the-guard.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:06Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:06Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-10T07:51:27Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from conductor 1.0.1, a trial instance, forge.route
direct. The guard refuses a lead's `glab api -X PUT
projects/<id>/merge_requests/<iid>/merge -f merge_when_pipeline_succeeds=true`; it admits `glab
mr create|update|note|merge`, and `glab api` only for GET or a pipeline retry. On the reporter's self-hosted GitLab `glab mr merge --auto-merge` answers 405, so a lead cannot set
merge-on-green on an existing merge request. Workaround in use: the push option
`-o merge_request.merge_when_pipeline_succeeds` for new merge requests, and the director session
makes the API calls. Detail: the reporter's defect list item 12.

The same report names `POST projects/<id>/pipeline` refused; story:guard-admits-own-pipelines
(wave 49) admits `glab api -X POST` to the own repository's pipeline and job retry endpoints, so
that part lands with wave 49.

## Acceptance

- Under forge.route direct the guard admits `glab api -X PUT
  projects/<own>/merge_requests/<iid>/merge` with `merge_when_pipeline_succeeds=true`, where
  `<own>` is the repository the checkout's `origin` names (by path or by numeric id), and denies
  it for another project; tests hold both.
