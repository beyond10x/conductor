---
format: aep.planning-md/3
id: story:guard-glab-writes
kind: story
status: implemented
title: The guard holds glab writes as it holds gh writes
relations:
- decomposes: epic:forge-neutral
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/guard/shell.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:42:57Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-13T10:42:58Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-13T10:42:58Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Why

The guard decides which `gh` calls write to a forge and holds them to the session's role
(`github_write`, `crates/conductor/src/guard/rules.rs:1781`); a `glab` call is not recognised, so
on a GitLab instance a controller could merge, comment or release through `glab` where the same
`gh` call is denied.

## Acceptance

- The guard reads a `glab` call as a read when it is one of: `mr view|list|diff`, `issue
  view|list`, `release view|list|download`, `repo view|list`, `ci view|list|status|trace|get`,
  `api` with GET and no request fields, `auth status`, any call with `--help`, and `--version`;
  every other call is a forge write held exactly as a `gh` write. A test in
  `crates/conductor/tests/guard_rules.rs` decides, for a controller and for conductor, each read
  form listed here as a read, and `mr create|merge|note|approve`, `issue create|close`, `release
  create|delete`, `repo create` and `api -X POST|PUT|DELETE` as writes; and it decides the pairs
  `glab mr view`/`gh pr view`, `glab mr merge`/`gh pr merge`, `glab mr note`/`gh pr comment`,
  `glab issue list`/`gh issue list`, `glab release create`/`gh release create`, `glab ci
  view`/`gh run view`, `glab api projects`/`gh api repos`, `glab api -X POST projects`/`gh api -X
  POST repos` and `glab auth status`/`gh auth status`, and finds each pair decided alike.

## Notes

- Security-relevant: reviewed by `aep:security-reviewer` before merge.
- The guard prose in the README, the design document and the website is `story:gitlab-guide`'s.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/src/guard/shell.rs`,
`crates/conductor/tests/guard_rules.rs`.
