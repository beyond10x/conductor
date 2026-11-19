---
format: aep.planning-md/3
id: story:unit-merge-policy
kind: story
status: draft
title: An instance setting chooses who merges a unit of work's request
summary: 'controllers.unit_merge: controller or human; the unit charter carries it'
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
revision: 1
---
## Why

A unit of work's controller starts on the default charter (`DEFAULT_CHARTER`,
`crates/conductor/src/spawn.rs:739`, `spawn/charter.md`), which says `merge: human`: its green
request waits for a person, also on an instance the operator runs autonomously (#207).

## Outcome

An instance setting chooses who merges a unit of work's request: `controller` or `human`.

## Acceptance

- A config key (`controllers.unit_merge`, values `controller`, `human`; default `human`, today's
  behaviour) is in `spec/`, `config validate` names a bad value by its YAML path, and
  `config show` prints it.
- With `controller`, the charter written for a unit of work carries `merge: controller` and the
  merge steps of the controller profile; with `human` it is byte for byte today's.
- `is_default_charter` still recognizes both rendered forms, so the loop keeps treating such a
  repository as a unit-of-work repository (`run/dispatch.rs:405`).
- A test fails on main: an instance with `unit_merge: controller` gets a unit charter without
  `merge: human`.

## Boundaries

The operator's own charters are not rewritten; the review cap of `merge: human` repositories is
unchanged.
