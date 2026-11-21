---
format: aep.planning-md/3
id: story:dashboard-unit-merge-setting
kind: story
status: active
title: The dashboard's settings page offers controllers.unit_merge
summary: A choice of controller or human for who merges a unit of work's request
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
scope:
- confidence: cited
  path: crates/conductor/dashboard/dist
- confidence: cited
  path: crates/conductor/dashboard/src/api/config.ts
- confidence: cited
  path: crates/conductor/dashboard/tests/config.spec.ts
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T04:41:04Z", actor: "human:unknown", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-20T04:41:04Z", actor: "human:unknown", revision: 4}
---
## Why

Wave 61 added `controllers.unit_merge` (`controller`, `human`). The dashboard's settings page
lists it already (its fields come from `config::document`, through
`fields()` in crates/conductor/src/dashboard/configure.rs:92-101; `document` emits it at
crates/conductor/src/config.rs:6457-6460), but as free text, with no select and no check:
`CHOICES` (crates/conductor/dashboard/src/api/config.ts:169-174) has no entry for it, so the page
renders no select (Config.vue:208-212) and checks no "one of" (config.ts:217-219).

## Outcome

`CHOICES` holds `'controllers.unit_merge': ['controller', 'human']`: the page shows a select with
the current value, and refuses any other value before it writes.

## Acceptance

- A Vitest case that fails on main: `problemOf('controllers.unit_merge', 'both')` is
  `one of controller, human`, and `CHOICES` holds exactly those two for the key.
- The committed `dist/` equals a fresh `task ui` build, and `task ui:ux-gate` holds its baseline
  (the ux fixtures hold no `unit_merge`, so it should not move).

## Boundaries

No other settings key changes; Config.vue and the API's settings write are unchanged; the key is
in spec/domains/config.yaml:404 already, so no spec change.

## Scope

crates/conductor/dashboard/src/api/config.ts, crates/conductor/dashboard/tests/config.spec.ts,
crates/conductor/dashboard/dist/.
