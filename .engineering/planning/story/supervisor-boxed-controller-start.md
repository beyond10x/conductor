---
format: aep.planning-md/3
id: story:supervisor-boxed-controller-start
kind: story
status: active
title: The supervisor starts a boxed instance's missing controller through instance cycle
summary: 'Profile: spawn only at placement none; host CLI version against the instances'' binary'
refs:
- provider: github
  reference: beyond10x/conductor#218
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor/roles/supervisor.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:33:50Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-19T18:33:50Z", actor: "human:unknown", revision: 3}
---
## Why

Issue #218. The host supervisor's routine, step 3 (`crates/conductor/roles/supervisor.md:208`),
starts a missing controller with `CONDUCTOR_INSTANCE=<name> conductor spawn --repository <repo>`.
For an instance in a substrate box that started the controller outside the box (#216); spawn now
refuses it there (`OutsideBox`). The working route is `conductor instance cycle`. Step 1 does not
compare the host CLI's version with the instances' binaries (seen: host 1.14.4, instances 1.15.0).

## Outcome

- Step 3: at `placement: none`, `conductor spawn`; at any other placement, `conductor instance
  cycle`, then a check that the controller is `Running` in `conductor controller controllers`.
- Step 1's host checks: the host's `conductor --version` equal to each instance's
  `binary_version` in `/v1/instances`.

## Acceptance (fail on main)

- `tests/profiles.rs` `the_supervisor_starts_a_boxed_controller_through_a_cycle`; the profile's
  line cap rises from 460 to 463 for these lines (`each_profile_stays_under_its_line_cap`).

## Boundaries

- Profile text only; the diff is reported with the change (AGENTS.md rule 9).
