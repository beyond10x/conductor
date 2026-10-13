---
format: aep.planning-md/3
id: story:dashboard-polish
kind: story
status: draft
title: 'The dashboard app is polished: layout, typography, states, dark mode, small screens'
relations:
- decomposes: epic:dashboard-charts
revision: 1
---
## Why

The operator asked for the web UI to be polished a lot more.

## Acceptance

- A design pass over every page: one spacing and type scale, tables with sticky headers and aligned numbers, empty, loading and error states on every panel, light and dark themes from the system setting, usable at 390 px width; Vitest snapshot tests per page and an axe accessibility check with no serious finding; `dist/` rebuilt.

## Scope

`crates/conductor/dashboard/src/**`, `crates/conductor/dashboard/dist/`.
