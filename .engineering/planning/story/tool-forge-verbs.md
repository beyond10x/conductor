---
format: aep.planning-md/3
id: story:tool-forge-verbs
kind: story
status: draft
title: Branch, merge, push, pull requests, CI reruns and releases go through tool, with the forge credentials only in the broker
relations:
- decomposes: epic:agent-tools
- depends_on: story:tool-build-commit-report
scope:
- confidence: inferred
  path: crates/conductor-broker
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/collect
- confidence: cited
  path: crates/conductor/src/gates.rs
- confidence: cited
  path: crates/conductor/tests/fake_provider
- confidence: inferred
  path: crates/tool
- confidence: inferred
  path: spec/domains/tool.yaml
revision: 4
---
# Story: tool-forge-verbs

## Outcome

A session branches, merges, pushes, opens and comments on pull requests, reads their checks, reruns CI and cuts a release through `tool`, each verb within its grant. The forge credentials and the bot route stay in the broker, so a session that holds no credentials still does its forge work.

## Context

docs/design/roles-and-surfaces.md § 6.1 ("the broker holds … the forge credentials, the bot route"), § 6.3 (version control and forge rows), § 7 step 4. Today the guard refuses `gh` writes and non-read `glab` calls in Bash (docs/design/conductor.md § 7), and the 48 h analysis counted `b10x-gates` checks, pushes and `gh pr checks` polls among the round-trip sources (docs/design/agent-tools.md § 3). Both forges are served through the existing collectors, as epic:forge-neutral requires. story:session-forge-credentials gives a boxed session read-only credentials; this story makes even those unnecessary for writes.

## Domain relations

- Session → Repository, many-to-one: a verb acts only on the repositories its grant names. Decided in docs/design/roles-and-surfaces.md § 6.2 ("limits: repositories, paths…").
- Role → privileged verb: `merge` is held only by a lead, `release` only by a releaser. Decided in § 3 and § 6.3 (the "(lead)" and "(releaser)" marks); the grant enforces it, and no `ess/1` document declares roles yet.

## Acceptance

A worker whose grant names `branch`, `push` and `pr` opens a pull request from its branch through `tool`, its session environment holds no forge token, and a `merge` from that worker is refused naming the rule.

1. **Spec first.** `spec/domains/tool.yaml` gains the inputs and errors of `branch`, `merge`, `push`, `pr open|checks|comment`, `ci rerun` and `release`, and the role restriction of `merge` and `release` as grant rules. Validate and regenerate.
2. `tool branch <name>` and `tool merge <branch>` (lead only) act in the caller's tree. `tool push` runs through the Gates bot route the broker holds, and a refused push prints the Gates finding in at most 40 lines.
3. `tool pr open --title t [--body-file p]`, `tool pr checks <n>` (the failing checks only), `tool pr comment <n> "<text>"`, `tool ci rerun <run>` and `tool release <version>` (releaser only) call the forge of the repository's source, GitHub or GitLab. Each prints one line on success.
4. Each of these verbs on a repository outside the grant is refused `NotGranted`. `merge` from a role other than lead, and `release` from a role other than releaser, are refused naming the rule. Every call is recorded.
5. Tests: `crates/conductor-broker/tests/forge_verbs.rs` against the fake forge providers in `crates/conductor/tests/fake_provider`, for both forges. A test asserts that the environment the broker hands a session names no forge token.

## Scope

- `spec/domains/tool.yaml`, `crates/conductor-model` (regenerated)
- `crates/conductor-broker/src/` (vcs and forge modules), `crates/conductor-broker/tests/forge_verbs.rs`
- `crates/tool/src/`
- `crates/conductor/src/collect/github.rs`, `crates/conductor/src/collect/gitlab.rs`, `crates/conductor/src/gates.rs` (reused, not changed in behaviour)

## Out of Scope

- `plan`, `escalate`, `memo` and `wait` (story:tool-plan-memo-wait).
- Moving any role to these verbs with Bash denied (story:agent-run-grants for the lead).
- Release verification (story:release-verify).

## Ambiguities

None.
