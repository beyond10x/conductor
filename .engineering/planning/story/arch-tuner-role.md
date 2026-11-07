---
format: aep.planning-md/3
id: story:arch-tuner-role
kind: story
status: draft
title: A tuner role adjusts settings within a grant the operator fills
summary: Migration step 8 of the architecture page
revision: 2
---
## Why

Step 8 of the migration in `website/docs/concepts/architecture.md`, section "Migration, each step reversible", which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- `tuner` with an empty grant; the operator fills `globals.tuning`.
- Reversible by: an empty grant; a test or the wave page shows the reversal works.
- The page's step 8 row is marked done when this ships.
