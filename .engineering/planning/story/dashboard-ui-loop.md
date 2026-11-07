---
format: aep.planning-md/3
id: story:dashboard-ui-loop
kind: story
status: active
title: 'The dashboard runs the UI iteration loop: a baseline, layout and accessibility gates on every merge, rounds of at most five changes'
summary: the dashboard has no baseline, no layout gate and no screen-only check
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:03:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:03:17Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-11-07, started the UI iteration loop on this repository and chose the dashboard
app (`crates/conductor/dashboard`) as its target. The dashboard has no baseline, no layout gate and
no screen-only check; the app's tests are Vitest over jsdom with axe-core
(`crates/conductor/dashboard/package.json`).

This is the standing story of the loop: each round's change set is one task under it, so a round
needs no new approval.

## Acceptance

Round 1 (set-up):

- An adapter for the dashboard: the scenarios are hermetic store fixtures that drive each page
  (overview, repository, decisions, KPIs, health) state by state; gating viewports 1440x900 and
  1280x800, measured-only 1024x768, 768x1024, 390x844; both themes the app ships; the
  always-visible elements are the header with the instance selector and the page navigation; the
  completion state of the Decisions page is an answered decision.
- `ux/baseline.json` with each scenario's values, `ux/rounds/R-01/` with `report.md`,
  `round.json` and `shots/`, `ux/trend.md` with one row, and `ux/findings/` with a record per
  confirmed finding.
- Every merge runs, without keys or network: a layout contract (no horizontal page scroll, no
  element past its box, the always-visible elements fully in view, at the gating viewports in both
  themes over every scenario state), axe-core with 0 serious or critical violations
  (`wcag2a, wcag2aa, wcag21a, wcag21aa, wcag22aa`), and the ratchet over `ux/baseline.json`. They
  are part of `task check` and of the CI `task check` job.
- A test fails on a closed S1 or S2 finding with no regression asset.

Each later round: at most 5 changes, each finding's regression asset red before its fix, the
ratchet never worse, `ux/trend.md` one row longer.

## Notes

- The browser runner (Playwright or an equivalent) must run in the CI `task check` job; the
  dashboard's committed `dist/` check stays as it is.
- Running code stays Rust or the app's TypeScript (AGENTS.md rule 3).
