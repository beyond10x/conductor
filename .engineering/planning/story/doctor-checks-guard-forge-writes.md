---
format: aep.planning-md/3
id: story:doctor-checks-guard-forge-writes
kind: story
status: active
title: conductor doctor checks the guard allows each role's forge writes
summary: doctor said ready while the guard denied every forge write
scope:
- confidence: inferred
  path: crates/conductor/cli.yaml
- confidence: inferred
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: inferred
  path: crates/conductor/src/guard.rs
- confidence: inferred
  path: crates/conductor/src/guard/rules.rs
- confidence: inferred
  path: crates/conductor/src/role.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:11:40Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T13:11:40Z", actor: "human:timo", revision: 4}
---
## Why

Reported 2026-11-07 from a running instance: `conductor doctor` ended `ready` while the guard
denied every forge write of the instance's controllers. Why the guard denied them is reported, not
verified. Verified in the code: `crates/conductor/src/doctor.rs` checks the programs conductor
starts, their versions and pins, and holds no check of the guard or of the settings that wire it
(no `guard` or `hook` in the file), so a guard that denies a role's routine writes cannot turn the
verdict.

## Acceptance

- `conductor doctor` asks the guard, for each session role, about one forge write that role is
  allowed by design (a pull request create for a controller in its own checkout), without
  recording anything, and prints one line per role: `allowed`, or `denied` with the guard's rule.
- A role whose sample forge write the guard denies makes the verdict `not ready`, naming the role.
- A test in `crates/conductor/tests/doctor.rs` runs it against a config whose guard denies the
  controller's forge writes and reads `not ready` on the last line.

## Route and policy (added 2026-11-07)

The operator, 2026-11-07, relayed by the host supervisor ("make sure the issues are fixed and
that they are persistent ... part of conductor"). `conductor doctor` said ready on two GitLab
instances whose controllers could not write to the forge.

- `conductor doctor` is red when `forge.route: gates` has no `gates_policy`, or when the bot route
  has no route for a source's host, and its line names the fix (`forge.route: direct`, a
  `gates_policy`, or the bot's route for that host). A test reads each case.
