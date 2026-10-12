---
format: aep.planning-md/3
id: review-result:ops-hardening-parallel-r1
kind: review-result
status: active
title: Parallel-safety critic, round 1, epic operating-loop-hardening
relations:
- reviews: story:agents-link-user-scope
- reviews: story:session-tasks-follow-prefix
- reviews: story:operator-decision-guarded
- reviews: story:single-conductor-lease
revision: 1
---
needs-revision

Findings, one per line. The first four pairs are ordered by the planned waves, and units inside a wave run one after the other. I therefore treat them as unnamed rather than as concurrent corruption.

story:agents-link-user-scope — shares `Taskfile.yml` and `crates/conductor/tests/taskfile.rs` with story:session-tasks-follow-prefix (same wave 07) and story:restart-current-profile (wave 06), and `AGENTS.md` with story:ci-full-gate (wave 06). The body names none of them and no `depends_on` edge records the order, so `aep plan artifact waves` puts it in wave 1, ahead of ci-full-gate and restart-current-profile in wave 2, the reverse of the planned delivery. The remedies are an ordering edge naming the shared files as its reason, or a split so the surfaces stop overlapping. (cited) — .engineering/planning/story/agents-link-user-scope.md:9-15

story:session-tasks-follow-prefix — shares `Taskfile.yml` and `crates/conductor/tests/taskfile.rs` with story:restart-current-profile (wave 06) and does not say so. The remedies are an ordering edge or a split. (cited, both frontmatters) — .engineering/planning/story/session-tasks-follow-prefix.md:9-15

story:session-tasks-follow-prefix — its Scope names the directory `crates/conductor/src/dashboard/` (it holds `usage.rs` and `waste.rs`), but the "over 300k" line is at `crates/conductor/src/dashboard.rs:1096`. The refusal text is built in `crates/conductor/src/doctor.rs:77` from the subject at `crates/conductor/src/snapshot.rs:599`, and `website/docs/getting-started.md:144` and `website/docs/guides/troubleshoot.md:53` quote it. Correct the surface so its overlap with story:operator-decision-guarded on `dashboard.rs` becomes visible. (cited) — .engineering/planning/story/session-tasks-follow-prefix.md:35-36

story:operator-decision-guarded — its acceptance needs files its scope omits: `crates/conductor/src/decide.rs` (`show` at line 694), `crates/conductor/src/cli.rs` (`RecordOperatorDecisionArgs`), `crates/conductor/tests/decision.rs`, and `crates/conductor/src/dashboard.rs`. It also needs, inferred, a recording-session field on the decision, which means `spec/domains/decision.yaml` and a regenerated `crates/conductor-model`. Without them its overlaps with story:single-conductor-lease (spec, model, `cli.rs`, wave 08) and with story:session-tasks-follow-prefix (`dashboard.rs`, wave 07) are unnamed. Name them in scope and add an ordering edge or split. (cited for the files, inferred for the spec and model) — .engineering/planning/story/operator-decision-guarded.md:31-35, .engineering/planning/story/operator-decision-guarded.md:40-41

story:operator-decision-guarded — shares `crates/conductor/src/guard/rules.rs` and `crates/conductor/tests/guard_rules.rs` with story:guard-store-migrate (wave 06). That story rewrites the `GROUPS` and `READ_LEAVES` derivation (`rules.rs:636`, `rules.rs:662`), and neither body says so. The remedies are an ordering edge naming the two files or a split. (cited, both frontmatters) — .engineering/planning/story/operator-decision-guarded.md:9-14

story:single-conductor-lease — its frontmatter scope lists only `guard.rs`, `docs/design/conductor.md` and `spec/domains`, while its Scope section claims `crates/conductor/src/`, `crates/conductor/tests/` and `crates/conductor-model/` wholesale. A surface that wide collides with every other wave-08 unit, and the store's collision check sees only the narrow list. Name the files: the `session` command in `cli.rs`, `watch.rs`, the new session module, the new spec file and the role settings template. (cited) — .engineering/planning/story/single-conductor-lease.md:9-15, .engineering/planning/story/single-conductor-lease.md:58-60

story:single-conductor-lease — it and story:operator-decision-guarded both change `spec/domains`, the regenerated `crates/conductor-model`, `cli.rs` and the guard module. `guard.rs` is the parent of `guard/rules.rs`, so neither can change without rebuilding the other, and neither body says so. They share wave 08. The remedies are an ordering edge or a split. (the lease side is cited; the operator-decision side is inferred, see its finding above) — .engineering/planning/story/single-conductor-lease.md:29-44

**What I read.** All 9 story bodies and the epic frontmatter. I ran `aep plan artifact waves --kind story --status draft`, `relations` and `graph`, plus `git grep` over `crates/`, `spec/`, `docs/`, `website/docs/`, `Taskfile.yml` and `AGENTS.md`.

**Surfaces.**
- Cited: 9 of 9 by frontmatter scope, but 3 are incomplete: operator-decision-guarded, session-tasks-follow-prefix and single-conductor-lease.
- Inferred: operator-decision-guarded's spec and model surface.
- Unplaceable: 0.

**Acceptance reads traced to a producer in the set.**
- 1: story:public-text-check scans files that ci-full-gate, agents-link-user-scope, init-records-without-origin, operator-decision-guarded and single-conductor-lease write. It is a standing guard, not a dependency, so I raised no finding.
- 0 of those 1 are recorded by a `depends_on` edge.
- The other reads (`conductor config show` `session_name`, the `task check` job) already exist on disk.

**Clear pairs.** The four wave-06 units do not collide with each other. init-records-without-origin collides with no other unit in the set.

**Out of my lane, not counted.**
- public-text-check's `/home/` rule, if read as a substring, matches `docs/analysis/2026-10-09-substrate-sessions.md:57` (`/workspace/home/…`), so the test would fail on day one. That is an acceptance matter.
- operator-decision-guarded's "a controller session is denied the command outright" may already hold, because `("decision","record-operator-decision")` is not in `READ_LEAVES`. That is also an acceptance matter.

```findings
- file: .engineering/planning/story/agents-link-user-scope.md
  line: 9
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares Taskfile.yml and crates/conductor/tests/taskfile.rs with story:session-tasks-follow-prefix (same wave 07) and story:restart-current-profile (wave 06), and AGENTS.md with story:ci-full-gate (wave 06); the body names none, no depends_on edge records the order, and the store's waves put it in wave 1 ahead of ci-full-gate and restart-current-profile (wave 2), the reverse of the planned delivery; remedies are an ordering edge naming the shared files or a split (cited)"
- file: .engineering/planning/story/session-tasks-follow-prefix.md
  line: 9
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares Taskfile.yml and crates/conductor/tests/taskfile.rs with story:restart-current-profile (wave 06) and does not say so; remedies are an ordering edge naming the shared files or a split (cited)"
- file: .engineering/planning/story/session-tasks-follow-prefix.md
  line: 35
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "scope names the directory crates/conductor/src/dashboard/ but the 300k line is crates/conductor/src/dashboard.rs:1096 and the refusal text is built in doctor.rs:77 from snapshot.rs:599 and quoted in website/docs/getting-started.md:144 and website/docs/guides/troubleshoot.md:53; the real surface hides its overlap with story:operator-decision-guarded on dashboard.rs (cited)"
- file: .engineering/planning/story/operator-decision-guarded.md
  line: 31
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance needs crates/conductor/src/decide.rs, cli.rs, tests/decision.rs and dashboard.rs (cited) and, inferred, spec/domains/decision.yaml plus regenerated crates/conductor-model for a recording-session field, none in scope, so its overlaps with story:single-conductor-lease (spec, model, cli.rs, wave 08) and story:session-tasks-follow-prefix (dashboard.rs, wave 07) are unnamed; name them and add an ordering edge or split"
- file: .engineering/planning/story/operator-decision-guarded.md
  line: 9
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares crates/conductor/src/guard/rules.rs and crates/conductor/tests/guard_rules.rs with story:guard-store-migrate (wave 06), which rewrites the GROUPS/READ_LEAVES derivation, and neither body says so; remedies are an ordering edge naming the two files or a split (cited)"
- file: .engineering/planning/story/single-conductor-lease.md
  line: 58
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "frontmatter scope lists only guard.rs, docs/design/conductor.md and spec/domains while the Scope section claims crates/conductor/src/, crates/conductor/tests/ and crates/conductor-model/ wholesale; a surface that wide collides with every wave-08 unit and the store's collision check sees only the narrow list; name the files (cited)"
- file: .engineering/planning/story/single-conductor-lease.md
  line: 29
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "this and story:operator-decision-guarded both change spec/domains, regenerated crates/conductor-model, cli.rs and the guard module (guard.rs is the parent of guard/rules.rs) in wave 08 and neither body says so; remedies are an ordering edge or a split; the operator-decision side is inferred"
```
