---
format: aep.planning-md/3
id: story:pr-check-names
kind: story
status: draft
title: A pull request's failing checks are recorded and reported by name
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
## Why

The operator's top-three report of 2026-10-14 said a pull request "stays red on Security"; it was
also red on eight test shards and two documentation checks. The snapshot records only a count of
failing checks per pull request, so a cycle cannot name them.

## Acceptance

- Spec first: the pull-request record carries the names of its failing checks beside
  `failing_checks` (`spec/domains/observation.yaml`); the GitHub and GitLab collectors fill it; a
  test in `crates/conductor/tests/collect_github.rs` over a fixture with three failing checks holds
  all three names.
- `conductor status` and the board name every failing check of a pull request they list;
  `.agents/conductor.md` says a report on a pull request names every red check; tests hold both.

## Scope

`spec/domains/observation.yaml`, `crates/conductor-model`, `crates/conductor/src/collect/`,
`crates/conductor/src/status.rs`, `crates/conductor/tests/collect_github.rs`, `.agents/conductor.md`,
`crates/conductor/tests/profiles.rs`.
