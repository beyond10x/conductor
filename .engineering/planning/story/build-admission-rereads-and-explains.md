---
format: aep.planning-md/3
id: story:build-admission-rereads-and-explains
kind: story
status: active
title: A queued build run re-reads its thresholds and says why it waits, with figures
summary: 21 runs queued at 73G free; a raised build_slots admitted none of them
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T13:47:00Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T13:47:01Z", actor: "human:timo", revision: 4}
---
## Why

Host supervisor, 2026-11-08 14:47 local: 21 grants Requested, 3 Running, 73G free against a 15G
`thresholds.disk_admit`; controllers waited 20 to 100 min on `build run`. A queued run logged
"waiting for disk" and "waiting for slots: all 3 build_slots are taken" in turn. The supervisor
raised `thresholds.build_slots` from 3 to 5 on every instance at 14:46; the Running count stayed 3.

Read in `crates/conductor/src/build.rs`:
- `Settings` (line ~288) reads `thresholds.build_slots` once, when the run starts; a run already
  queued keeps the old bound for as long as it waits. Each run compares the host-wide count of
  building grants (`ledger.building`) with its own instance's bound.
- The queued line (`tell`, line ~918) says "disk" whenever the slots are not all taken, also when
  the run waits only because an earlier request heads the queue; it names no figure.
- A queued request for `sh -c env | grep …` asked 60.9G (the pooled-peak estimate).

## Acceptance

1. A waiting run re-reads its instance's thresholds at each pass, so a raised `build_slots` or a
   lowered `disk_admit` admits it without a restart. Test: a queued run is granted after the bound
   is raised in its config.
2. The queued line names its reason with the arithmetic: `free <n>G, reserved <n>G, needed <n>G,
   floor <n>G`, or `behind <grant> (<n>G)` when an earlier request holds the head, or
   `slots <taken>/<bound>`. Test for each.
3. Where several instances share the host, the slot bound is the host's: the largest `build_slots`
   of the instances in the host registry, and `docs/config.md` says so. Test with two instances.
4. A command that is not cargo-based and names no `--reserve` asks no pooled peak (the `env | grep`
   case). Test.

## Ambiguities

Acceptance 3 takes the largest bound; the smallest would be the conservative choice.

## Further evidence

Host teacher, 2026-11-08 15:41 local, relayed by the supervisor: after `build_slots` was raised to
5, admission let 26 runs reach Running (`host grants`) while free disk fell 2.5 GB/min (37 GB in
15 min); the disk reservations did not hold the total. Acceptance 5: the sum of the reservations
of Running grants plus the floor never exceeds free disk at admission, and a run whose measured
growth passes its reservation is counted at its measured size. Test.
