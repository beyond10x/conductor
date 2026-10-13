---
format: aep.planning-md/3
id: story:doctor-identity
kind: story
status: draft
title: conductor doctor names the machine and each forge identity
relations:
- depends_on: story:doctor-minimum-versions
scope:
- confidence: cited
  path: crates/conductor-model
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
  path: website/docs/reference/config.md
revision: 2
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
- For each needed forge CLI that is present, `conductor doctor` prints the account it is logged in
  as on each host it knows (`gh auth status`, `glab auth status`, read without network writes),
  login name only, never a token or an e-mail address, or `not logged in`; a test with fake `gh`
  and `glab` on `PATH` reads one logged-in and one not-logged-in host, and asserts no token string
  from the fake's output appears.
- A forge CLI not logged in to the host an instance's source names makes the verdict `not ready:
  <program> (not logged in to <host>)`; a test holds it.

## Notes

- Spec first: the machine rows, the identity rows and the new verdict reason are declared in
  `spec/domains/config.yaml` (doctor's result) before the code, with the regenerated model and
  `website/docs/reference/cli.md`.
- Output is local; the login name is the user's own account. No token, e-mail or other personal
  data is printed.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model`, `website/docs/reference/config.md`,
`crates/conductor/src/doctor.rs`, `crates/conductor/tests/doctor.rs`,
`website/docs/guides/troubleshoot.md`, `website/docs/getting-started.md`.
