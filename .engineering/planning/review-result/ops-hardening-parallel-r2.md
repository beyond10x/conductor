---
format: aep.planning-md/3
id: review-result:ops-hardening-parallel-r2
kind: review-result
status: active
title: Parallel-safety critic, round 2, epic operating-loop-hardening
relations:
- reviews: story:single-conductor-lease
- reviews: story:agents-link-user-scope
- reviews: story:instance-scoped-tasks
revision: 1
---
needs-revision

Round 1's findings are resolved. The remaining findings are unnamed collisions in the planned order. They are ordered by wave and not concurrent, so I treat them as unnamed and not as corruption.

Findings:

story:single-conductor-lease — it adds a new `session` command group, and `story:guard-store-migrate` (wave 06) adds a test that every `conductor` subcommand is a read leaf or denied. The hand-kept `GROUPS` and `READ_LEAVES` lists in `crates/conductor/src/guard/rules.rs:636` and `:662` therefore have to change when `session claim` lands, along with `crates/conductor/tests/guard_rules.rs`. Neither file is in its scope, and no edge to `story:guard-store-migrate` records it. Add both files and an ordering edge, or split the surface. (cited that the lists are hand-kept, inferred that the edit is needed) — .engineering/planning/story/single-conductor-lease.md:43-44

story:agents-link-user-scope — it and `story:instance-scoped-tasks` (both wave 07) both change `Taskfile.yml` and `crates/conductor/tests/taskfile.rs`. Each depends on `story:restart-current-profile`, but no edge or note orders them against each other. Add an ordering edge naming the two files, or split. (cited, both frontmatters) — .engineering/planning/story/agents-link-user-scope.md:34

story:instance-scoped-tasks — it regenerates `crates/conductor-model` (wave 07), as do `story:operator-decision-guarded` and `story:single-conductor-lease` (wave 08). The generated files they share include `plan.json`, `PLAN.md` and `ports/conductor_cli.rs`. Its body does not mention them, and `aep plan artifact waves` puts operator-decision-guarded first, the reverse of the plan. Add an ordering edge or split. (cited, frontmatters) — .engineering/planning/story/instance-scoped-tasks.md:14-17

Not findings:
- Wave 06 has no pair collisions.
- `story:operator-decision-guarded` and `story:single-conductor-lease` are ordered by edge and notes.
- `story:lease-guard` is ordered after its two dependencies.
- `story:watch-two-conductors`, `story:init-records-without-origin`, `story:doctor-refusal-wording` and `story:session-tasks-follow-prefix` collide with nothing in the set.

**What I read.** All 13 story bodies, plus `aep plan artifact waves`, the graph, round 1's record, `rules.rs` and the `crates/conductor-model` file list.

**Surfaces.** 13 cited, 0 inferred, 0 unplaced.

**Acceptance reads.** I traced 2 to a producer in the set, and one `depends_on` edge records them (`story:lease-guard` to `story:single-conductor-lease`). The other is the `guard-store-migrate` walk of the CLI tree, covered by the first finding.

Out of my lane: none.

```findings
- file: .engineering/planning/story/single-conductor-lease.md
  line: 43
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "adds a new `session` command group while story:guard-store-migrate's test walks every conductor subcommand, so GROUPS/READ_LEAVES in crates/conductor/src/guard/rules.rs:636,662 and crates/conductor/tests/guard_rules.rs must change; neither file is in scope and no depends_on edge to story:guard-store-migrate records it; add both files and an edge, or split (cited that the lists are hand-kept; the needed edit is inferred)"
- file: .engineering/planning/story/agents-link-user-scope.md
  line: 34
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares Taskfile.yml and crates/conductor/tests/taskfile.rs with story:instance-scoped-tasks (both wave 07); neither body orders the two; add an ordering edge naming the files, or split (cited, both frontmatters)"
- file: .engineering/planning/story/instance-scoped-tasks.md
  line: 14
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "regenerates crates/conductor-model (plan.json, PLAN.md, ports/conductor_cli.rs) shared with story:operator-decision-guarded and story:single-conductor-lease (wave 08); its body names neither and the store waves order operator-decision-guarded first, the reverse of the plan; add an ordering edge or split (cited, frontmatters)"
```
