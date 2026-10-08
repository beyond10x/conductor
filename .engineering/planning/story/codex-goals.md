---
format: aep.planning-md/3
id: story:codex-goals
kind: story
status: draft
title: The snapshot reads the operator's Codex thread goals
revision: 1
---
# The snapshot reads the operator's Codex thread goals

## What

The operator, relayed by conductor: "there were goals which should probably be read by
conductor". Source: `~/.codex/goals_1.sqlite`, table `thread_goals` (`thread_id`, `goal_id`,
`objective`, `status` in active|paused|blocked|usage_limited|budget_limited|complete,
`token_budget`, `tokens_used`, `time_used_seconds`, `created_at_ms`, `updated_at_ms`; read from
`.schema`).

A collector reads it read-only on each snapshot. Objectives may name the operator's employer or its
products, so:
- the store and every export carry only the thread goal's id, status, tokens used, time used,
  updated time, and the `G` id it maps to (or none);
- the objective text stays in `state/codex-goals.json`, which Git ignores, for conductor to read;
- the mapping to a `G` id is conductor's decision, recorded with neutral wording.

Spec first: a `CodexGoal` observation in `spec/domains/observation.yaml` with those fields and no
text field.

Depends on `story:snapshot-driver`.

## Acceptance

- A snapshot over a fixture database records one observation per row with no objective text in the
  store or in any export; `rg` for a fixture objective over the exports prints nothing.
- The board counts thread goals by status and lists those mapped to each `G` id.
