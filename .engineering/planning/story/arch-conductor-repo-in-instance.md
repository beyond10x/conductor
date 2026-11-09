---
format: aep.planning-md/3
id: story:arch-conductor-repo-in-instance
kind: story
status: draft
title: The conductor repository joins an instance; a releaser role replaces conductor-dev
summary: Migration step 6 of the architecture page
revision: 3
---
## Why

Step 6 of the migration in `docs/design/architecture-next.md` (the page `website/docs/concepts/architecture.md` it was drafted from is deleted), which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- The conductor repository in an instance; `releaser` role; `conductor self-install`; `conductor-dev` retired.
- Reversible by: the `conductor-dev` role stays accepted for one release; a test or the wave page shows the reversal works.
- The page's step 6 row is marked done when this ships.
