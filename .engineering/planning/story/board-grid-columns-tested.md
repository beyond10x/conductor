---
format: aep.planning-md/3
id: story:board-grid-columns-tested
kind: story
status: draft
title: A test fails when the plan board grid ignores --board-columns
tags:
- from-finding
relations:
- informed_by: review-result:w49-board-hide-done-adversary-1
revision: 1
---
## Why

Wave 49 adversary pass on story:board-hide-done (review-result:w49-board-hide-done-adversary-1) found that no test checks the board grid uses `--board-columns`: a copy of `crates/conductor/dashboard/src/views/Plan.vue` with the grid fixed at `repeat(5, …)` (line 382) leaves the suite green, because Vitest loads no component styles in jsdom. The gap predates the wave.

## Acceptance

- A test fails when the `.board` grid in `Plan.vue` stops reading `--board-columns`, for example a Playwright check of the column count with Done hidden.
