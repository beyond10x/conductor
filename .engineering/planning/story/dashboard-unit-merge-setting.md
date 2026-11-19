---
format: aep.planning-md/3
id: story:dashboard-unit-merge-setting
kind: story
status: draft
title: The dashboard's settings page offers controllers.unit_merge
summary: A choice of controller or human for who merges a unit of work's request
refs:
- provider: github
  reference: beyond10x/conductor#207
relations:
- decomposes: epic:routine-duties-in-loop
revision: 1
---
## Why

Wave 61 added `controllers.unit_merge` (`controller`, `human`). The dashboard's settings page
(`crates/conductor/dashboard/src/api/config.ts`) lists a choice for each other `controllers` key but
none for this one, so an operator who runs an instance from the dashboard cannot see or change who
merges a unit of work's request.

## Outcome

The settings page shows `controllers.unit_merge` with its two values and writes it as it writes the
other `controllers` keys.

## Acceptance

- The settings page lists `controllers.unit_merge` with the choices `controller` and `human`, the
  current value selected; a write sends the key through the existing settings write.
- A Vitest case that fails on main: the settings model holds the key with exactly those two values.
- The committed `dist/` equals a fresh `task ui` build, and `task ui:ux-gate` holds its baseline.

## Boundaries

No other settings key changes; the API's settings write is unchanged.

## Scope

`crates/conductor/dashboard/src/api/config.ts`, its tests, `crates/conductor/dashboard/dist/`.
