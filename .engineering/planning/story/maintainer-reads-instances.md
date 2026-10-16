---
format: aep.planning-md/3
id: story:maintainer-reads-instances
kind: story
status: draft
title: The maintainer reads every registered instance, and a check fails on committed text that names an instance
relations:
- decomposes: epic:role-names
- depends_on: story:role-names-in-files
- depends_on: story:instance-names-unique
scope:
- confidence: cited
  path: .agents/maintainer.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/host.rs
- confidence: cited
  path: crates/conductor/tests/host_scan_names.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: website/docs/reference
revision: 3
---
# Story: maintainer-reads-instances

## Outcome

The maintainer reads every registered instance's records and transcripts as product evidence, and
a check fails when text it is about to commit names an instance.

## Context

Design: `docs/design/roles-and-surfaces.md` § 2 ("The maintainer is not part of any instance. It
reads every instance's records and transcripts as product evidence. What it writes into the product
never names an instance (rules today, enforced by the scan)") and § 8 question 5, approved by the
operator on 2026-10-16; `docs/design/conductor.md` § 12 (what conductor-dev reads). Today the
public-text check in `crates/conductor/tests/no_organization.rs` finds record ids and home paths,
but no test can know an instance's name: AGENTS.md rule 5 forbids a test from reading the
operator's real config. So the check that knows the names runs from the host registry of story
`instance-names-unique` (`<host>/instances.json`), outside `cargo test`.

## Domain relations

- Maintainer → Instance, one-to-many, read only, no ownership; each outlives the other — inferable
  from `docs/design/roles-and-surfaces.md` § 2 (operator-approved); no `ess/1` document declares it.
- Host registry → Instance, one-to-many — from story `instance-names-unique` (draft), on which this
  story depends; not yet declared in an `ess/1` document.

## Acceptance

1. Spec first: `spec/components.yaml` (or `crates/conductor/cli.yaml` for a local command) gains
   `conductor host scan-names [<path>…]`, which reads the host registry and fails, exit 1, on any
   registered instance name or session prefix as a word of its own in the given files, naming
   file, line and the rule, never the name; with no registry it prints that it skipped.
   `ess specify validate --path spec` and `task regen` pass.
2. Test: `crates/conductor/tests/host_scan_names.rs` with a fixture registry under a temporary
   `HOME` holding instances `example-a` and `example-b`: a file naming `example-a` fails, a file
   naming neither passes, the output never prints the name.
3. `task check` runs `conductor host scan-names` over the tracked public text (the paths
   `no_organization.rs` reads) when a registry exists, and prints that it skipped otherwise.
4. `.agents/maintainer.md` lists what it reads for each registered instance, by the
   `conductor config show --format json` fields (`records`, the transcripts directory), and runs
   `conductor host scan-names` on its staged text before each commit. Test:
   `crates/conductor/tests/profiles.rs` holds both sentences.

## Scope

`spec/components.yaml` or `crates/conductor/cli.yaml`, `crates/conductor-model` (generated),
`crates/conductor/src/host.rs`, `crates/conductor/src/cli.rs`,
`crates/conductor/tests/host_scan_names.rs`, `crates/conductor/tests/profiles.rs`, `Taskfile.yml`
(`check`), `.agents/maintainer.md`, `website/docs/reference/**` (generated).

## Out of Scope

- Where the maintainer is configured (decision-blocker `maintainer-host-config`).
- The host registry itself (story `instance-names-unique`).
- A view across instances for the operator (epic `host-bridge`).
