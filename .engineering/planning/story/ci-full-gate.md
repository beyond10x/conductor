---
format: aep.planning-md/3
id: story:ci-full-gate
kind: story
status: implemented
title: CI runs task check on every pull request and on main
relations:
- decomposes: epic:operating-loop-hardening
scope:
- confidence: cited
  path: .github/workflows/check.yml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T17:03:45Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-12T17:03:48Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-12T17:27:23Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

`task check` (spec validation, conformance, the model compared with a fresh generation, fmt,
clippy, `cargo test`, docs-check) runs only on this host. CI runs the site build, the docs crate's
tests and the shared Gates (`.github/workflows/pages.yml`, `shared-gates.yml`). On 2026-10-12 a
merge waited hours for disk on the host, and one docs round merged without `task check`.

## Design

A workflow `.github/workflows/check.yml` with one job named `task check`, triggered on every pull
request and on every push to `main`. It installs the tools the gate calls at the pins in
`b10x.toml` (`ess`, `aep`) and leaves `B10X_GATES_POLICY` unset (the shared Gates job scans
already).

## Acceptance

- The `task check` job of `.github/workflows/check.yml` runs on every pull request and every push
  to `main`, and goes red on a pull request carrying a failing test (observed once on a throwaway
  branch, its run linked in the wave record).

## Notes

- Once the job is green on `main`, `AGENTS.md` should say a pull request merges when the `task
  check` job is green, and that a local `task check` is needed only when CI cannot run it. That is
  a follow-up edit, not part of this story's acceptance.

## Scope

`.github/workflows/check.yml`.
