---
format: aep.planning-md/3
id: story:disk-protocol-spec
kind: story
status: implemented
title: The host ledger, disk reservations and reclaim are specified first
relations:
- decomposes: epic:disk-protocol
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T17:12:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T17:12:18Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T18:12:43Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`epic:disk-protocol`: disk becomes a protocol the binary runs. Its nouns — the host ledger, a disk
reservation and its lifecycle, a reclaim run, a shortfall — have no typed home; `spec/drafts/host.yaml`
holds a draft ledger with open UNMAPPED markers.

## What

`spec/domains/host.yaml`, from the draft, listed in `spec/system.yaml` and `spec/ess-inputs.yaml`:
- `HostGrant` gains kind `Disk` with `bytes` (requested), `peak_bytes` (measured), the command it
  covers, its process group, and the lifecycle `Requested → Granted → Running → Released`, with
  `Refused` terminal; a grant whose process group is gone is released by the next command that
  reads the ledger (settles the UNMAPPED expiry marker: no time expiry).
- Commands: `ReserveDisk` (outcomes: granted, granted after reclaim, refused with shortfall),
  `StartRun`, `RecordPeak`, `ReleaseGrant`; `Reclaim` (outcomes: reclaimed `<bytes>` with its steps,
  floor not reached with what is left by path and size).
- Views: `Grants` (live and recent, with peak), `ReclaimRuns`.
- The CLI binding in `crates/conductor/cli.yaml`: `conductor build run`, `conductor host grants`,
  `conductor gc reclaim`.

## Acceptance

- `ess specify validate --path spec` exits 0 with no UNMAPPED marker left in the host domain.
- `ess verify conform synthesize --path spec` yields scenarios for every new outcome.
- `task regen` regenerates `crates/conductor-model` with the new types (coordinator runs it).

## Scope

`spec/domains/host.yaml` (new, from `spec/drafts/host.yaml`), `spec/system.yaml`,
`spec/ess-inputs.yaml`, `spec/components.yaml`, `crates/conductor/cli.yaml`.
