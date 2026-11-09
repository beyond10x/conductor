---
format: aep.planning-md/3
id: story:arch-tuner-role
kind: story
status: archived
title: A tuner role adjusts settings within a grant the operator fills
summary: Migration step 8 of the architecture page
revision: 4
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T10:19:50Z", actor: "human:timo", revision: 4}
---
## Why

Step 8 of the migration in `docs/design/architecture-next.md` (the page `website/docs/concepts/architecture.md` it was drafted from is deleted), which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- `tuner` with an empty grant; the operator fills `globals.tuning`.
- Reversible by: an empty grant; a test or the wave page shows the reversal works.
- The page's step 8 row is marked done when this ships.

## Archived

Duplicate of story:tuner, which is implemented; archived on 2026-11-09 by the owner's planning-maintenance decision.
