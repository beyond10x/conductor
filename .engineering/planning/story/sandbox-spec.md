---
format: aep.planning-md/3
id: story:sandbox-spec
kind: story
status: draft
title: The sandbox's commands, refusals and records are specified first
relations:
- decomposes: epic:session-confinement
- depends_on: story:gitlab-guide
- depends_on: story:version-flag
- depends_on: story:doctor-minimum-versions
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/tests/cli_tree.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: spec/domains/observation.yaml
- confidence: cited
  path: spec/domains/session.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/reference/config.md
revision: 15
---
## Why

Conductor is specification first (AGENTS.md rule 1): every noun, command and refusal of the
sandbox gets a typed home in `spec/` before a story is implemented against it. The config already
declares `confinement` and each role's `limits`, `writable` and `network`
(`spec/domains/config.yaml:152-241`), and the observation the `envelope_*` fields
(`spec/domains/observation.yaml:367-391`); what the sandbox stories add has no declaration yet.
The operator, 2026-10-13: "we need to prioritise the sandbox" and "conductor is specs first".

## Acceptance

- `spec/` declares, with `ess specify validate --path spec` passing on `ess` 0.59 (the spec's
  current format, `ess/22`):
  - `session start` (`[session, start]` in `crates/conductor/cli.yaml`, input `role`: conductor or
    conductor-dev) and one error per refusal: provider unavailable, measurement short, unknown
    role, session already running, launch failed; and `session stop` (`[session, stop]`, input
    `role`) with its refusals: unknown role, no session running;
  - `SpawnRefusal` gains provider unavailable and measurement short (`spec/domains/dispatch.yaml`);
  - a started session's identity (session name, provider, the unit name where there is one) and
    the measurement record a provider writes, including a partial box under `report` (limits
    applied, no write fence: `envelope_writable` absent with a `fence_missing` reason) beside the
    existing unboxed form;
  - a provider availability row (`provider`, `available` or the reason) as part of `doctor`'s
    result;
  - the resolved writable places of a session (each place word with its absolute paths) on the
    measurement;
  - `roles[].forge_token_env` (a list of environment variable names) on a role, carried on the
    session request a provider receives.
- `ess verify conform synthesize` refuses nothing new, the regenerated `crates/conductor-model`,
  `website/docs/reference/config.md` and `website/docs/reference/cli.md` match (`task check`), and
  the CLI tree test (`crates/conductor/tests/cli_tree.rs`) holds the new `session start` and
  `session stop` leaves, each with a handler that answers "not implemented" (exit 2) until
  `story:session-envelope-seam`; a test reads that answer.

## Notes

- Every `UNMAPPED:` marker the drafting leaves is listed in the review report and settled before
  the seam is implemented.
- After wave 17 (`story:gitlab-guide` lands last there; the model, `reference/` pages, `cli.rs`
  and `cli_tree.rs` are shared with its stories).

## Scope

`spec/domains/session.yaml`, `spec/domains/dispatch.yaml`,
`spec/domains/config.yaml`, `spec/domains/observation.yaml`, `spec/ess-inputs.yaml`,
`crates/conductor/cli.yaml`, `crates/conductor-model`, `crates/conductor/src/cli.rs`,
`crates/conductor/tests/cli_tree.rs`, `website/docs/reference/config.md`,
`website/docs/reference/cli.md`.
