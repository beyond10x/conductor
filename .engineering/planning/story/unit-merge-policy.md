---
format: aep.planning-md/3
id: story:unit-merge-policy
kind: story
status: active
title: An instance setting chooses who merges a unit of work's request
summary: 'controllers.unit_merge: controller or human; the unit charter carries it'
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
- depends_on: story:loop-alerts-when-it-cannot-act
scope:
- confidence: cited
  path: conductor.example.yaml
- confidence: inferred
  path: crates/conductor-model
- confidence: inferred
  path: crates/conductor/http.yaml
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/spawn
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: inferred
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: crates/conductor/tests/work_start.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/concepts/the-loop.md
- confidence: inferred
  path: website/docs/reference
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T19:24:58Z", actor: "human:unknown", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-19T19:24:58Z", actor: "human:unknown", revision: 7}
---
## Why

A unit of work's controller starts on the default charter (`DEFAULT_CHARTER`,
`crates/conductor/src/spawn.rs:739`, `spawn/charter.md`), which says `merge: human` and "never merge
it": its green request waits for a person, also on an instance the operator runs autonomously
(#207). The charter is written once, when missing or empty (`default_charter`, `spawn.rs:784`,
called at `:182`), so it stays in the records after it was written.

## Outcome

An instance setting chooses who merges a unit of work's request: `controller` or `human`. A
repository on conductor's default charter follows it at its next unit start.

## Acceptance

- `controllers.unit_merge` (values `controller`, `human`; absent: `human`, today's behaviour) is a
  field of `conductor.config.Controllers` in `spec/domains/config.yaml` and of `CONTROLLER_KEYS`
  (`crates/conductor/src/config.rs:905`); `conductor config validate` names a bad value by its YAML
  path and exits 1; `conductor config show --format json` prints it.
- With `human`, the charter a unit start writes is `DEFAULT_CHARTER` byte for byte. With
  `controller`, it is a second compiled form whose Boundaries say: once the full gate passed and
  the request's checks are green, merge it (no auto-merge), one request at a time, then report
  `done`; its last line is `merge: controller`.
- A charter equal byte for byte to either default form is rewritten to the form `unit_merge` names
  at the next unit start; any other charter is never rewritten.
- `is_default_charter` (`spawn.rs:745`, called by `Tick::spawn`, `crates/conductor/src/run.rs:800`)
  answers true for both forms, so such a repository still gets a controller only for a dispatch.
- Tests in `crates/conductor/tests/work_start.rs` that fail on main: a unit start on an instance
  with `unit_merge: controller` writes a charter whose last line is `merge: controller`; an existing
  human-form default charter is rewritten to the controller form; an edited charter is left
  byte for byte.

## Boundaries

The operator's own charters are never rewritten; the `merge: human` review cap
(`run/dispatch.rs:382`, `:406`) is unchanged; a goal's dispatches are not affected.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model` (regenerated), `crates/conductor/http.yaml`
(re-pinned `source_digest`), `crates/conductor/src/config.rs`, `crates/conductor/src/spawn.rs`,
`crates/conductor/src/spawn/`, `crates/conductor/tests/work_start.rs`, `docs/config.md`,
`conductor.example.yaml`, `website/docs/concepts/the-loop.md`, `website/docs/reference/`
(regenerated).

## Notes

Depends on `story:loop-alerts-when-it-cannot-act`: both change `spec/` and so regenerate
`crates/conductor-model` and re-pin `crates/conductor/http.yaml`.
