---
format: aep.planning-md/3
id: story:setup-command
kind: story
status: implemented
title: conductor setup checks a repository's prerequisites and applies the safe fixes
relations:
- decomposes: epic:onboarding-from-url
- depends_on: story:setup-guide-for-agents
- depends_on: story:doctor-identity
scope:
- confidence: cited
  path: SETUP.md
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/setup.rs
- confidence: cited
  path: crates/conductor/tests/docs_setup.rs
- confidence: cited
  path: crates/conductor/tests/setup.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T13:13:50Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-13T13:14:02Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-10-13T13:19:36Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Why

`SETUP.md` (`story:setup-guide-for-agents`) leaves the repository's own checks to the agent's
reading: whether its planning store is current, whether an instance names it, whether the records
exist and whether it is trusted. `conductor doctor` already checks the machine, the programs, their
pins and the forge logins (`doctor.rs`: `examine`, `merge_pins`, `verdict`).

## Acceptance

- Spec first: `conductor setup` (`[setup]` in `crates/conductor/cli.yaml`, input `--repository
  <path>`, default the working directory, and `--apply`) and its repository rows are declared in
  `spec/` and validated before the code.
- `conductor setup` prints doctor's rows, computed by doctor's own functions, then four repository
  rows: the planning store's version (current, old with the migrate command, or none), whether an
  instance config names the repository, whether its records exist, whether it is trusted; it ends
  with one verdict, doctor's verdict function over both sets of rows (`ready` or `not ready:
  <check>, …`). Tests in `crates/conductor/tests/setup.rs` over fixture repositories (GitHub
  origin; GitLab origin with a self-hosted host; no origin; an old store) with fake CLIs read each
  repository row and the verdict.
- `--apply` writes the instance config naming the repository and runs `conductor init` and
  `conductor trust` when they are missing; it never logs in, installs a program or migrates a
  store, and prints each of those as the command to run; a test holds both halves.
- `SETUP.md` gains, before its last step, the step `conductor setup --apply`, with its `Check:`,
  `Fix:` and `Hands:` lines; `docs_setup.rs` holds it.

## Notes

- After `story:doctor-identity` (doctor's rows, `doctor.rs`, `cli.yaml`, `cli.rs`, the model,
  `reference/cli.md`) and `story:setup-guide-for-agents` (`SETUP.md`, `docs_setup.rs`).
  `story:snapshot-duration` comes after it on the model, `cli.rs` and `reference/cli.md`.

## Scope

`spec/domains/config.yaml`, `crates/conductor/cli.yaml`, `crates/conductor-model`,
`website/docs/reference/cli.md`, `crates/conductor/src/setup.rs` (new),
`crates/conductor/src/doctor.rs`, `crates/conductor/src/cli.rs`, `crates/conductor/src/lib.rs`,
`crates/conductor/tests/setup.rs` (new), `SETUP.md`, `crates/conductor/tests/docs_setup.rs`.
