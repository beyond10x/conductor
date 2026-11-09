---
format: aep.planning-md/3
id: story:arch-global-settings
kind: story
status: archived
title: Global settings with precedence over instance settings
summary: Migration step 3 of the architecture page
revision: 4
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T10:19:51Z", actor: "human:timo", revision: 4}
---
## Why

Step 3 of the migration in `docs/design/architecture-next.md` (the page `website/docs/concepts/architecture.md` it was drafted from is deleted), which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- `globals` with precedence; `config show --effective`.
- Reversible by: keys stay valid at the instance level; a test or the wave page shows the reversal works.
- The page's step 3 row is marked done when this ships.

## Archived

Duplicate of story:config-globals, which is implemented; archived on 2026-11-09 by the owner's planning-maintenance decision.
