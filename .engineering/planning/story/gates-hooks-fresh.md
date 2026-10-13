---
format: aep.planning-md/3
id: story:gates-hooks-fresh
kind: story
status: draft
title: A checkout whose Gates hooks lag is flagged and can be refreshed
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/collect/repositories.rs
- confidence: cited
  path: crates/conductor/src/repository.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/collect_stores.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/observation.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 3
---
## Why

On 2026-10-13, 42 of 47 checkouts under an instance's checkouts root ran an older Gates hook
(`.git/b10x-gates-hooks/b10x-gates` 0.1.14 or 0.1.15) while `b10x-gates` on `PATH` was 0.1.17; a
controller found it only when its pushes relied on the old hook's behaviour (conductor's report).
Nothing records a checkout's installed hook version, and refreshing them is a manual step per
checkout.

## Acceptance

- Spec first: the repository record gains the installed Gates hook version
  (`spec/domains/observation.yaml`), and the board's deviations gain `stale gates hooks` when it
  differs from the `b10x-gates --version` on `PATH`; a test with a fixture checkout whose hook
  prints an older version reads the field and the deviation.
- Spec first: `conductor repository refresh-hooks [--apply]` lists each checkout of the instance
  whose hook lags or is absent (`.git/b10x-gates-hooks` missing: 12 checkouts on 2026-10-13) and,
  with `--apply`, runs `b10x-gates install --repo <checkout> --repository <owner>/<name> --policy
  <the instance's policy>` in it and prints the version before (or `absent`) and after; a test with
  a fake `b10x-gates` on `PATH` holds an absent, a lagging and a current checkout.
- The watch prints `gates hooks stale: <n> checkouts` once per new snapshot that finds any; a test
  holds the line.

## Scope

`spec/domains/observation.yaml`, `spec/components.yaml`, `crates/conductor-model`,
`crates/conductor/cli.yaml`, `crates/conductor/src/cli.rs`, `crates/conductor/src/repository.rs`,
`crates/conductor/src/collect/repositories.rs`, `crates/conductor/src/status.rs`,
`crates/conductor/src/watch.rs`, `crates/conductor/tests/collect_stores.rs`,
`crates/conductor/tests/watch.rs`, `website/docs/reference/cli.md`.
