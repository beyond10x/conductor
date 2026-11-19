---
format: aep.planning-md/3
id: story:disk-low-fits-disk
kind: story
status: active
title: One disk floor for reclaim and the supervisor; doctor names a disk_low the disk cannot meet
summary: reclaim floor disk_admit+5G is the check; doctor warns on disk_low above half the filesystem
refs:
- provider: github
  reference: beyond10x/conductor#213
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: crates/conductor/roles/supervisor.md
- confidence: cited
  path: crates/conductor/roles/teacher.md
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/host.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:31:59Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-19T18:31:59Z", actor: "human:unknown", revision: 5}
---
## Why

Issue #213. On a host whose `/` is 145G, `globals.thresholds.disk_low` at its default 100G
(`crates/conductor/src/config.rs:442`, `DISK_LOW_GIB`) can never be met with 89G used, so the
watch reports `disk low` on every pass. The supervisor's profile checked free disk against
`thresholds.disk_low` and, under it, ran `conductor host reclaim`
(`crates/conductor/roles/supervisor.md:70`, `:169`, `:185`), whose floor is another one:
`thresholds.disk_admit` plus a 5G margin (`crates/conductor/src/gc/reclaim.rs:168`,
`floor_bytes`, read at `:236`), so it answered "reclaimed 0 bytes ... the floor is 26843545600"
every time. Two floors, one of them unreachable.

## Outcome

- One floor: `disk_admit` plus 5G, the floor `conductor host reclaim` prints and the watch's
  reclaim and `build run` already use. The supervisor's and the teacher's disk checks read it
  (`roles/supervisor.md`, `roles/teacher.md`); `disk_low` stays the watch's report line, and the
  spec says it is no floor (`conductor.config.Thresholds`, `conductor.host.ReclaimDisk`).
- `conductor doctor` prints, for each instance on no remote daemon whose `disk_low` lies above
  half the filesystem at its `disk_path`, one line `disk <instance>: thresholds.disk_low <n>G lies
  above half of the <m>G filesystem at <path>; ...` asking for a lower `disk_low` and
  `disk_clear`; reported, not a failure (`doctor.rs`, `disk_low_lines`).

## Acceptance (fail on main)

- `crates/conductor/tests/doctor.rs` `doctor_names_a_disk_low_above_half_the_filesystem`: a
  `disk_low` of 1000000G gets the line, one of 1G none, and doctor exits 0 either way.

## Boundaries

Not `config validate`: the check reads the host's filesystem, and validate is a check of the file
(a validate note would fire on every small CI or test disk under the 100G default). The
defaults do not change, and `host reclaim`'s floor does not move to `disk_low` (that would make a
reclaim run on a sound host aim at 100G free).
