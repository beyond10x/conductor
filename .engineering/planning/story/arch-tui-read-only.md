---
format: aep.planning-md/3
id: story:arch-tui-read-only
kind: story
status: draft
title: conductor tui shows instances and sessions, read-only
summary: Migration step 2 of the architecture page
revision: 2
---
## Why

Step 2 of the migration in `website/docs/concepts/architecture.md`, section "Migration, each step reversible", which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- `conductor tui` read-only: instances, sessions, attach, watch, log.
- Reversible by: nothing to undo; a test or the wave page shows the reversal works.
- The page's step 2 row is marked done when this ships.
