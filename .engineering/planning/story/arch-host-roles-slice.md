---
format: aep.planning-md/3
id: story:arch-host-roles-slice
kind: story
status: draft
title: The supervisor and teacher are roles started by conductor session start in a host slice
summary: Migration step 1 of the architecture page
revision: 2
---
## Why

Step 1 of the migration in `website/docs/concepts/architecture.md`, section "Migration, each step reversible", which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- `conductor_host.slice`; the supervisor and teacher as roles in `globals.roles`, started by `conductor session start --role supervisor`; `task supervisor:start` calls it.
- Reversible by: the old task line; a test or the wave page shows the reversal works.
- The page's step 1 row is marked done when this ships.
