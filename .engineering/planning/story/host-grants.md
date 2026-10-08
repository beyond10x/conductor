---
format: aep.planning-md/3
id: story:host-grants
kind: story
status: draft
title: Instances on one host take grants from one ledger
relations:
- decomposes: epic:multi-instance-host
- depends_on: story:instance-session-names
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/host.rs
- confidence: cited
  path: spec/domains/host.yaml
revision: 7
---
## Why

Each conductor grants build slots and starts controllers against the host's disk and CPU alone, so
two instances can each grant the same space (`specification:multi-instance-host`, finding 6).

## Acceptance

- Spec first: `spec/domains/host.yaml` from `spec/drafts/host.yaml` (its `SessionName` imported
  from the config domain, story `instance-session-names`), its UNMAPPED markers settled: a grant is
  released by `conductor host release` or when its session leaves the session list; refusal
  outcomes for the host's disk floor and most working controllers. The host's limits are a `host:`
  section of the config file, parsed in `crates/conductor/src/host.rs`.
- `conductor host grant`, `conductor host release` and the view `conductor host grants` keep the
  ledger in `~/.b10x/conductor/host/state`.
- Tests: (1) two instances' grants against one floor, the second refused with the numbers;
  (2) two processes take grants at the same moment and the ledger admits no more than the floor
  allows; (3) a grant whose session has left the fixture session list is released by the next
  `conductor host grants` read.
- The conductor profile takes a host grant before starting a controller or granting a build over
  `thresholds.build_size`.

## Scope

`spec/domains/host.yaml` (new), `spec/ess-inputs.yaml`, `spec/system.yaml`,
`crates/conductor/cli.yaml`, `crates/conductor/src/host.rs` (new), `crates/conductor/src/cli.rs`,
`crates/conductor/tests/host.rs` (new), `.agents/conductor.md`. After `instance-session-names`
(shared `.agents/conductor.md`; the `SessionName` type).
