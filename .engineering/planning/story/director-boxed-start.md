---
format: aep.planning-md/3
id: story:director-boxed-start
kind: story
status: active
title: A boxed instance's director never starts unboxed, and doctor names a missing one
summary: session start --role director refuses OutsideBox outside the box; doctor has a director row
refs:
- provider: github
  reference: beyond10x/conductor#220
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/tests/box_session.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: spec/domains/session.yaml
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:33:49Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-19T18:33:49Z", actor: "human:unknown", revision: 3}
---
## Why

Issue #220. On a host whose instances run in substrate boxes, no instance has a director, and
`CONDUCTOR_INSTANCE=<instance> conductor session start --role director` started one unboxed on the
host (`started …: provider none, …, unboxed: the provider systemd is not available`). Session start
never looks at the placement: it chooses the `confinement` provider
(`crates/conductor/src/session.rs:1743`) and, under `mode: report`, falls back to `none`, noting it
only in its printed line (`session.rs:1850`). Nothing reports a missing director: `conductor doctor`
has no such row.

## Outcome

- `session start --role director` for an instance whose placement opens a substrate box, run outside
  that box, refuses `OutsideBox` (a new `SessionStartRefusal`) before anything is launched; a boxed
  start never falls back to an unboxed one. A host role and the maintainer still start on the host.
- `conductor doctor` prints, for each boxed instance with a director role, `director <instance>:
  <session> runs in its box` or `director <instance>: no director session runs in its box
  (<session>); …`, read from the box's registry of child sessions. A report, not a failure.

## Acceptance (fail on main)

- `tests/box_session.rs` `on_the_host_a_boxed_instance_starts_no_session_outside_its_box`: on the
  host, `session start --role director` exits 1 with `session start: refused: OutsideBox`, the
  harness not called.
- `tests/doctor.rs` `doctor_names_a_boxed_instance_without_a_director_session`.

## Boundaries

- Starting the director in the box from `instance serve` is `story:box-runtime-starts-director`.
- The director's session name is `story:controller-name-not-conductor`.
