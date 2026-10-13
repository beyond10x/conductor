---
format: aep.planning-md/3
id: story:onboarding-run
kind: story
status: draft
title: Two fresh sessions given only the URL end ready
relations:
- decomposes: epic:onboarding-from-url
- depends_on: story:setup-command
- depends_on: story:snapshot-duration
scope:
- confidence: cited
  path: AGENTS.md
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
  path: crates/conductor/src/setup.rs
- confidence: cited
  path: crates/conductor/tests/docs_setup.rs
- confidence: cited
  path: crates/conductor/tests/setup.rs
- confidence: cited
  path: docs/analysis/onboarding-run.md
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 16
---
## Why

Onboarding is only proven by doing it: a fresh session, in a repository conductor has never seen,
given nothing but the URL (`epic:onboarding-from-url`).

## Acceptance

- Two recorded runs in `docs/analysis/onboarding-run.md`, each a new Claude Code session started
  in a scratch repository on this host with the prompt "<conductor repository URL> — I want to use
  conductor in this repo": one repository with a GitHub `origin`, one with a GitLab `origin` on a
  self-hosted host (`gitlab.example.com` in the record), each with a planning store one format old.
  Each run records every step the session took, every question it asked the person, the time to
  `ready`, and `conductor setup` output before and after, whose planning-store row reads old before
  and current after and whose verdict reads `ready` after.
- Each gap a run hits is fixed in `SETUP.md`, `conductor setup` or `conductor doctor` in the same
  wave, and the run is repeated until both end `ready` with questions only for logins and the store
  migration.
- AGENTS.md § Releases gains the step: repeat both runs before a release that changes anything
  onboarding runs: `SETUP.md`, the README pointer, `conductor setup`, `conductor doctor`, the
  install path (release archives) or the prerequisites (the epic's promise that onboarding stays
  working).

## Notes

- The runs install conductor from the wave's branch (`cargo install --git … --rev <head>`, in a
  build slot) until v0.8.0's archives exist; the record says which.

- After `story:snapshot-duration` (the dashboard stories come after it on the same files), so the gap fixes land on `setup.rs`,
  `doctor.rs`, `cli.yaml`, `cli.rs`, the model and `reference/cli.md` after every other story of
  the wave.

## Scope

`docs/analysis/onboarding-run.md` (new), `SETUP.md`, `AGENTS.md`, `crates/conductor/src/setup.rs`,
`crates/conductor/src/doctor.rs`, `crates/conductor/cli.yaml`, `crates/conductor/src/cli.rs`,
`spec/domains/config.yaml`, `crates/conductor-model`, `website/docs/reference/cli.md`,
`crates/conductor/tests/setup.rs`, `crates/conductor/tests/docs_setup.rs`.
