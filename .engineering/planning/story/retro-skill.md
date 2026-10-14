---
format: aep.planning-md/3
id: story:retro-skill
kind: story
status: draft
title: A conductor:retro skill and docs page run the retro
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:retro-spec
revision: 2
---
## Why

`docs/design/user-retro.md` (approved 2026-10-13): a conductor:retro skill runs the retro with a person in the loop.

## Acceptance

- the skill runs `conductor retro`, asks "What went wrong?" and "What went well?", shows the report, and asks the engineer to type `send`;
- its profile forbids `--yes` (a profile test holds the sentence);
- a docs page under website/docs/guides/ shows the flow and lists what is never sent.

## Scope

`.claude/skills/conductor-retro/` (or the product's skill location), `website/docs/guides/retro.md`, tests.
