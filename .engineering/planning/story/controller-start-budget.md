---
format: aep.planning-md/3
id: story:controller-start-budget
kind: story
status: draft
title: A controller starts on at most 45k tokens
relations:
- decomposes: epic:controller-health
revision: 1
---
## Why

Conductor, 2026-10-14 (G9 K16): a controller started at 14:09Z read 83k input tokens on its first
turn and 122k by its tenth, against a hand-over threshold of 150k, leaving about 30k of working
room. The charter was 20k of it (conductor compacts charters); the rest is fixed: the harness
prompt with its tool and skill listings, the agent file, and what the profile tells a controller to
read at start, which is the whole `rules.md` (71KB, about 18k tokens) although a controller follows
only its own section and the all-sessions one, plus the repository's `AGENTS.md` (15-61KB) and
`README.md`.

## Acceptance

- `docs/analysis/controller-start-budget.md` records, from a headless start with the controller
  settings in a fixture repository, the input tokens of each part (harness and tools, skills
  listing, deferred-tool and connector instructions, agent file, profile, rules read, charter,
  repository files), before and after.
- `.agents/repo-controller.md` reads the rules through `conductor rules show --section "repository
  controller"` and `--section "all sessions"` (when present), never the whole file; reads
  `README.md` only when the dispatch needs it; and the profile itself stays under 8KB; profile tests
  hold the reading list and the size.
- The controller settings drop what a controller never uses (plugins, connector servers), named in
  the analysis; a controller's first turn is at most 45k input tokens with a 15KB charter, measured
  as above and by the health probe a day later.

## Scope

`.agents/repo-controller.md`, `.claude/controller-settings.json`, `crates/conductor/tests/profiles.rs`,
`docs/analysis/controller-start-budget.md` (new).
