---
format: aep.planning-md/3
id: story:focus-line
kind: story
status: draft
title: The board says what the organization is working on, per goal
revision: 1
---
# The board says what the organization is working on, per goal

## What

The operator asked conductor to compress everything currently in progress into 3 to 5 words.
Conductor answered with a table it built by hand from its hand-over: the running controllers
grouped by the goal their dispatch serves, and a one-line summary.

The counts are in the store once dispatches are (`story:decision-commands`, `story:log-import`):
each open dispatch names a goal and a repository. This story shows them without a hand count:

- `conductor dispatch focus` (spec first: a view over open dispatches, one row per goal: goal id,
  goal title, repositories, count, share), and the same table at the top of the dashboard.
- The 3–5-word line stays conductor's to write; the table is what it writes it from.

## Acceptance

- `ess specify validate --path spec` is valid with the new view.
- With dispatches open for 3 repositories on G4 and 1 on G2 in a fixture store, `dispatch focus
  --format jsonl` prints two rows, G4 first with share 0.75.
- The dashboard's `/data.json` carries the same rows.
