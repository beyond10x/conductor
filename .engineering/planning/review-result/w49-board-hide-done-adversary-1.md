---
format: aep.planning-md/3
id: review-result:w49-board-hide-done-adversary-1
kind: review-result
status: active
title: Wave 49 U2 adversary, pass 1
relations:
- reviews: story:board-hide-done
revision: 1
---
unit: story:board-hide-done, worktree w49-u2 at HEAD 1d8d4206c plus one uncommitted test file
verdict: CONFIRMED (one note-level gap in the tests; nothing red against the acceptance)
cases: executed 218→222, red 0
origin: introduced 0, pre-existing 1, undecided 0
wrote-outside-worktree: 4 paths, all in the unit's scratch directory
needs-coordinator: none

I could not make the feature fail. All three acceptance lines hold. The only finding is a gap in
the tests: nothing checks that the board's grid actually uses the column count.

Cases added in crates/conductor/dashboard/src/views/__tests__/plan-hide-done-adversary.spec.ts,
all green on first run: the no-storage fake really throws; storage that refuses writes still
releases the toggle; pressing the toggle again hides Done and saves 'true'; an empty saved value
counts as a first visit.

Suite: `npx vitest run`: Test Files 31 passed (31), Tests 222 passed (222), exit 0; without the
new file 218 passed.

Finding: crates/conductor/dashboard/src/views/Plan.vue:382, confirmed, pre-existing. A copy of
Plan.vue with the grid fixed at `repeat(5, …)` leaves the unit's spec 4/4 green; the
four-column test only checks the inline `--board-columns: 4` value, and Vitest loads no component
styles in jsdom. Plan.vue is the same at the base commit.

Tried and could not break: first-visit default, the saved key conductor.plan.hideDone, no storage,
throwing storage, saved values other than 'false', tests/plan.spec.ts, the committed dist/ build,
errors on load or click.

```findings
- file: crates/conductor/dashboard/src/views/Plan.vue
  line: 382
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: no test checks that the board grid uses --board-columns, so fixing it at five columns leaves the whole suite green and acceptance line 1's four-column layout unchecked
```
