---
format: aep.planning-md/3
id: story:arch-host-roles-slice
kind: story
status: archived
title: The supervisor and teacher are roles started by conductor session start in a host slice
summary: Migration step 1 of the architecture page
revision: 4
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T10:19:51Z", actor: "human:timo", revision: 4}
---
## Why

Step 1 of the migration in `docs/design/architecture-next.md` (the page `website/docs/concepts/architecture.md` it was drafted from is deleted), which the operator asked for on 2026-11-07. Drafted from the merged page; the operator decides which steps become active.

## Acceptance

- `conductor_host.slice`; the supervisor and teacher as roles in `globals.roles`, started by `conductor session start --role supervisor`; `task supervisor:start` calls it.
- Reversible by: the old task line; a test or the wave page shows the reversal works.
- The page's step 1 row is marked done when this ships.

## Archived

Duplicate of story:host-roles-by-daemon, which is implemented; archived on 2026-11-09 by the owner's planning-maintenance decision.
