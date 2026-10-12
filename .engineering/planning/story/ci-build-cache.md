---
format: aep.planning-md/3
id: story:ci-build-cache
kind: story
status: implemented
title: CI's task check restores a cargo cache
relations:
- decomposes: epic:disk-and-gate
scope:
- confidence: cited
  path: .github/workflows/check.yml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T19:01:41Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-12T19:01:41Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-12T19:14:37Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

CI's `task check` became the merge gate in wave 06, but `.github/workflows/check.yml` has no cache:
with cancel-in-progress, every push pays a full build against a 45-minute timeout.

## Acceptance

- A second `task check` run on an unchanged `Cargo.lock` restores a cargo cache keyed on
  `Cargo.lock` and the toolchain, from an action pinned by commit, and its log shows the cache hit
  (run linked in the wave record).

## Scope

`.github/workflows/check.yml`.
