---
format: aep.planning-md/3
id: story:board-hide-done
kind: story
status: active
title: The plan board hides done stories by default, behind a toggle
summary: A "Hide done" toggle on the Plan board, on by default, remembered per browser
scope:
- confidence: inferred
  path: crates/conductor/dashboard/dist/assets/index-C8abUvRL.css
- confidence: cited
  path: crates/conductor/dashboard/dist/assets/index-rugqrwX_.js
- confidence: inferred
  path: crates/conductor/dashboard/dist/index.html
- confidence: cited
  path: crates/conductor/dashboard/src/views/Plan.vue
- confidence: inferred
  path: crates/conductor/dashboard/src/views/__tests__/__snapshots__/pages.spec.ts.snap
- confidence: cited
  path: crates/conductor/dashboard/src/views/__tests__/plan-hide-done.spec.ts
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T18:17:55Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T18:17:56Z", actor: "human:timo", revision: 4}
---
## Why

The operator, 2026-11-07, relayed by the host supervisor: "dev work: board -> toggle button "hide done" active by default". The Done column fills the board with finished work and pushes the open columns out of view.

## Acceptance

- The Plan board's bar has a "Hide done" toggle, pressed on a first visit; while pressed, the Done column is not shown and the board lays out the other four columns.
- Releasing the toggle shows the Done column again; the choice is kept in the browser's local storage and read back on the next visit, and a page without storage still renders with the toggle pressed.
- A Vitest test mounts the board with a Done card: the card is absent by default and present after the toggle is released.

## Scope

`crates/conductor/dashboard/src/views/Plan.vue`, its test under `crates/conductor/dashboard/src/views/__tests__/`, and the committed `dist/`.
