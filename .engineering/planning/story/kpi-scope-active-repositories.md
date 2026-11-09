---
format: aep.planning-md/3
id: story:kpi-scope-active-repositories
kind: story
status: active
title: Repository-scoped KPIs count only Active repositories
summary: K5 counted 28 repositories where 4 are Active
scope:
- confidence: cited
  path: crates/conductor/src/metric.rs
- confidence: cited
  path: crates/conductor/src/repository.rs
- confidence: inferred
  path: crates/conductor/tests/metric.rs
- confidence: cited
  path: docs/design/self-improvement.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T15:37:36Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T15:37:36Z", actor: "human:timo", revision: 3}
---
## Why

Host supervisor, 2026-11-07: on a second instance, `metric metric-readings` for 2026-11-06 held K5 unreleased rows for 28 repositories (instance total 157), including repositories with no Active mark and outside any controller's work (deployment configs, profile and chart repositories); only 4 were Active. `crates/conductor/src/metric.rs` filters no repository by `repository activity`, and `docs/design/self-improvement.md:43` names no scope for K5. A KPI over repositories nobody works on cannot be lowered by the loop.

## Acceptance

- K5 and every repository-scoped KPI count only repositories Active in `repository activity` on the reading's day; the instance row sums those scopes only. A test with one Active and one Inactive repository, each with an unreleased merge, reads one scope.
- `docs/design/self-improvement.md` names the scope rule for repository-scoped KPIs.
- `repositories[].no_releases` stays the way to leave an Active repository out of K5.
