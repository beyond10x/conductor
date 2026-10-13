---
format: aep.planning-md/3
id: story:doctor-identity
kind: story
status: draft
title: conductor doctor names the machine and each forge identity
relations:
- depends_on: story:doctor-minimum-versions
- decomposes: epic:onboarding-from-url
- depends_on: story:setup-guide-for-agents
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/getting-started.md
- confidence: cited
  path: website/docs/guides/troubleshoot.md
- confidence: cited
  path: website/docs/reference/cli.md
revision: 8
---
## Why

When engineers post `conductor doctor` output to ask for help (a team's trial, 2026-10-13), the
reader cannot tell which forge account a CLI is logged in as, on which host, or which machine and
conductor version produced the output; each answer needs a second round trip. The operator,
2026-10-13: "would be good to see gh + glab identities under doctor? also hostname, system, etc".

## Acceptance

- `conductor doctor` prints, before the program rows, the machine: host name, operating system and
  release, architecture, and `conductor <version>`; a test in `crates/conductor/tests/doctor.rs`
  reads the four lines with the values `uname` gives and the crate version.
- The hosts doctor checks are those of the config file's sources, across every instance:
  `github.com` for a `github` source and the source's `host` for a `gitlab` source. For each such
  host whose CLI is present, doctor prints the account the CLI is logged in as there (from `gh auth
  status --hostname <host>` or `glab auth status --hostname <host>`), login name only, never a
  token or an e-mail address, or `not logged in`; a test with fake `gh` and `glab` on `PATH` and a
  config with one source of each reads one logged-in and one not-logged-in row, and asserts that neither the token string
  nor the e-mail address in the fake's output appears.
- A CLI not logged in to such a host makes the verdict `not ready: <program> (not logged in to
  <host>)`; the same test reads it.

## Notes

- Spec first: the machine rows, the identity rows and the new verdict reason are declared in
  `spec/domains/config.yaml` (doctor's result) before the code, with the regenerated model.
- After `story:setup-guide-for-agents` (`website/docs/getting-started.md`, whose doctor sample this
  story rewrites); `story:setup-command` reuses these rows and comes after it.
- Output is local; the login name is the user's own account.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model`, `crates/conductor/cli.yaml`,
`crates/conductor/src/cli.rs`, `website/docs/reference/cli.md`, `crates/conductor/src/doctor.rs`,
`crates/conductor/tests/doctor.rs`, `website/docs/guides/troubleshoot.md`,
`website/docs/getting-started.md`.
