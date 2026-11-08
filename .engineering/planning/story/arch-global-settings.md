---
format: aep.planning-md/3
id: story:arch-global-settings
kind: story
status: draft
title: Global settings with precedence over instance settings
summary: Migration step 3 of the architecture page
revision: 2
---
## Why

Step 3 of the migration in `website/docs/concepts/architecture.md`, section "Migration, each step reversible", which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- `globals` with precedence; `config show --effective`.
- Reversible by: keys stay valid at the instance level; a test or the wave page shows the reversal works.
- The page's step 3 row is marked done when this ships.
