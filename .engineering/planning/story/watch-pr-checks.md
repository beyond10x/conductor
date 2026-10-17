---
format: aep.planning-md/3
id: story:watch-pr-checks
kind: story
status: draft
title: The watch reports when an open pull request's checks have concluded
relations:
- decomposes: epic:loop-without-handwork
- decomposes: epic:red-ci-ownership
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: docs/design/conductor.md
revision: 3
---
# Story: watch-pr-checks

## Outcome

When the checks of an open pull request finish, the watch prints one line, and conductor relays the result to the controller at once. No controller polls CI.

## Context

Conductor's NEED N-20261017-10 (2026-10-17). In 40 minutes two controllers ran `gh pr checks --watch` in a background call (mandate PR 43, engineering-protocols PR 22), against rules items 118 and 139. The watch reports only `main` CI, so conductor learns of a PR's result at its 30-minute check-in (DEC-20261017-31) at the earliest.

Decided by conductor-dev (principle 3): watch every open pull request of an Active repository rather than only the ones a controller reported blocked. That needs no parse of report text, and the forge read is one call per repository per pass.

## Acceptance

1. The watch prints `pr checks: <repo>#<n> green|red [<failing check>,…] <head sha7>` once per pull request and head commit, when its checks finish (none pending). A new push, which is a new head, can print again. Tests with a fake forge: pending then green prints once; red names the failing checks; an unchanged state on the next pass prints nothing.
2. The last state printed per pull request and head is kept in the instance state, so a restart does not print it again. Test.
3. Forge reads go through the existing forge adapter, one list call per Active repository per pass, and stay inside the watch's existing rate budget. GitLab merge requests are covered by the same adapter, or the line says unsupported for that forge.
4. The conductor profile says: on a `pr checks` line, relay the result to that repository's controller as `[REPORT …]`-style news at once. Rules items 118 and 139 stand: controllers never watch CI. The profiles test pins the sentence.
5. The dashboard's activity feed shows the line.

## Out of Scope

Merging on green.

## Ambiguities

None.
