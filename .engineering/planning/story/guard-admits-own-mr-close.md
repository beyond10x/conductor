---
format: aep.planning-md/3
id: story:guard-admits-own-mr-close
kind: story
status: draft
title: A controller closes its own superseded merge request, or reports that it could not
summary: Under forge.route direct the guard admits glab mr close and gh pr close on the session's own repository, and the controller role reports a refused close as refused
tags:
- from-finding
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: inferred
  path: crates/conductor/roles/controller.md
- confidence: inferred
  path: crates/conductor/tests/guard_rules.rs
revision: 1
---
## Why

Agentbox friction log F47 (2026-11-15/16). Controllers reported a merge request closed when
the guard had refused their `glab mr close`. In e2e-tests !612 and ai-agent-platform !559 (the
09:18 report said "closed"), GitLab still showed the request open, and the operator's session
closed them.

Read at v1.13.0: `forge.route: direct` admits `mr create|update|note|merge` and
`pr create|edit|comment|merge|ready` on the session's own repository
(`crates/conductor/src/guard/rules.rs:786-794`). Close is not among them, so a controller
cannot retire a merge request it superseded, and the report and the forge disagree.

## Outcome

- The direct route admits `glab mr close` and `gh pr close` on the session's own repository
  (named with `-R`, or the checkout's), as it admits merge.
- The controller role says that a refused forge write is reported as refused, never as done.

## Acceptance

- Under `forge.route: direct`, the guard admits `glab mr close 12 -R <own repo>` and denies it
  for another repository. Both are held in `guard_rules` tests.
