---
format: aep.planning-md/3
id: story:arch-one-controller-per-instance
kind: story
status: draft
title: One controller per instance, with repository work done by implementer workers
summary: Migration step 5 of the architecture page
revision: 3
---
## Why

Step 5 of the migration in `docs/design/architecture-next.md` (the page `website/docs/concepts/architecture.md` it was drafted from is deleted), which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- One controller per instance; today's repository controllers become `implementer` workers it starts.
- Reversible by: `controllers.per_repository: true` keeps today's form; a test or the wave page shows the reversal works.
- The page's step 5 row is marked done when this ships.
