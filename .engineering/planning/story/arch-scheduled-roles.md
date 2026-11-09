---
format: aep.planning-md/3
id: story:arch-scheduled-roles
kind: story
status: draft
title: Roles carry a schedule and scope; the loop starts scheduled roles
summary: Migration step 4 of the architecture page
revision: 3
---
## Why

Step 4 of the migration in `docs/design/architecture-next.md` (the page `website/docs/concepts/architecture.md` it was drafted from is deleted), which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- `roles[].schedule` and `scope`; the loop starts scheduled roles.
- Reversible by: remove the entry; a test or the wave page shows the reversal works.
- The page's step 4 row is marked done when this ships.
