---
format: aep.planning-md/3
id: review-result:v8-design-r1
kind: review-result
status: active
title: Design critic, round 1, v0.8.0 additions
relations:
- reviews: epic:dashboard-for-engineers
- reviews: story:conductor-upgrade
- reviews: story:snapshot-member-failures
- reviews: story:fetch-without-prompts
- reviews: story:snapshot-duration
- reviews: story:dashboard-spa
- reviews: story:dashboard-overview
- reviews: story:dashboard-repo-view
- reviews: story:dashboard-health
- reviews: story:dashboard-snapshot-api
revision: 1
---
needs-revision

- `story:dashboard-overview` — its acceptance reads repositories, merge and pull requests with failing checks, and shipped-today releases, which only the `/api/*` endpoints of `story:dashboard-snapshot-api` provide. The edge runs the other way (`snapshot-api` depends_on `overview`) and the overview is tested only on fixture JSON, so its repository table never meets real snapshot rows. The overview should own the endpoints it renders, or depend_on `story:dashboard-snapshot-api` and flip the "shares `dashboard.rs`" ordering — `.engineering/planning/story/dashboard-overview.md:25`, `.engineering/planning/story/dashboard-snapshot-api.md:32`
- `story:dashboard-snapshot-api` — the set is cut by layer. One story declares all seven endpoints, including `/api/blockers`, `/api/specifications` and `/api/board`, which only later views read, and every view story tests against fixture JSON. No story shows a view reading a served endpoint, so the integration error sits in the seam between them. Each endpoint should move into the story that renders it, as the overview takes repositories, pull-requests, workflow-runs and releases, and `repo-view` takes blockers and specifications — `.engineering/planning/story/dashboard-snapshot-api.md:32`
- `story:dashboard-snapshot-api` — `/api/board` is declared as "the snapshot views' rows", but the `Board` record holds only `deviation_rows` as a count and a file path. The deviations (red main, failing PR, PR open over 14 days, unreleased commits, no checkout, specification missing or refused) exist only inside the private `figures()` of `status.rs`. The overview's "red CI on main", "open MRs/PRs with failing" and state chips, and `repo-view`'s waiting-on cases, would be re-derived in TypeScript. The body should say the endpoint serves what `status.rs::figures` computes, extracted and shared, and that the SPA does not recompute it — `crates/conductor/src/status.rs:332`, `spec/domains/observation.yaml:477`, `.engineering/planning/story/dashboard-overview.md:25`
- `story:dashboard-snapshot-api` — its edge to `story:onboarding-run` has no reason written beside it, in this body or in `onboarding-run`'s. Together with `setup-command` → `conductor-upgrade` → `snapshot-member-failures` → `fetch-without-prompts` → `snapshot-duration` → `onboarding-run`, it queues `repo-view`, `health` and `sandbox-spec` behind the wave's last story. The shared surface is `crates/conductor-model`, which is regenerated. The trade-off is to keep the edge and record that file as its reason, or drop it and accept a regeneration collision on the model with `snapshot-duration`, `member-failures` and `onboarding-run` — `aep plan artifact graph`, `.engineering/planning/story/dashboard-snapshot-api.md:10`
- `story:dashboard-health` — its acceptance reads collector runs, member failures "each with its fix text", doctor rows and the newest snapshot's state, but no story in the set declares an endpoint that serves them. `snapshot-api`'s list has none of them and serves only the newest complete snapshot, so a failed snapshot is invisible. The fix text is created by neither `snapshot-duration` nor `snapshot-member-failures`, whose records hold the member, collector and first error line. Health's scope has no `spec/domains/observation.yaml` or `crates/conductor-model`. Health should declare the health endpoint and where the fix text comes from, with those files in scope — `.engineering/planning/story/dashboard-health.md:25`, `.engineering/planning/story/snapshot-member-failures.md:20`
- `story:dashboard-spa` — its Notes put it "After the sandbox wave (`story:session-forge-credentials`)", but the graph puts it before that story. `session-forge-credentials` reaches `sandbox-spec`, which depends_on `dashboard-health`, which reaches `dashboard-spa`. Recording the order the Notes state would close a cycle, and the set is placed in v0.8.0, before the sandbox work. The Notes should state the real order — `.engineering/planning/story/dashboard-spa.md:39`, `aep plan artifact graph`

**What you read:** 9 artifacts in full (`aep plan artifact show` on the five dashboard stories, the epic, `conductor-upgrade`, `snapshot-member-failures` and `fetch-without-prompts`). I also read `snapshot-duration`, `setup-command`, `onboarding-run`, `sandbox-spec`, `session-forge-credentials`, `dashboard-engineer-view` and `dashboard-export` for their edges. I ran `aep plan artifact relations`, `graph` and `validate`. Validate printed `valid`; its five "closed on an assertion" notes are not mine. I checked `dashboard.rs`, `status.rs`, `observation.yaml`, `Taskfile.yml` `check` and `check.yml` against the bodies.

**Graph:** I walked all 111 `depends_on` edges in the store, outside the set as well, and found no cycle. `decomposes`, `informed_by` and `supersedes` are not "needs first" and I did not walk them. I traced about 11 acceptance reads to a producer in the set. Nine are recorded by an edge, directly or transitively; for example `health` reaches `snapshot-duration` and `member-failures` through `onboarding-run` and `fetch-without-prompts`. Two are not: the overview reading snapshot rows (finding 1) and `health` reading endpoints and fix text nobody produces (finding 5).

**What I could not establish:**
- The SPA build and embedding approach is sound as drawn. A committed `dist/` checked fresh mirrors the generated-model diff at `Taskfile.yml:315-316` and keeps `cargo install --git` working without Node.
- Two points on it are out of my lane. `Taskfile.yml` `check` and `check.yml` install no Node, `b10x.toml` pins none, and the acceptance names no Node pin. The freshness check is also stated twice, once in `task check` and once as a test in `tests/dashboard.rs`. Both belong to the acceptance and parallel-safety critics.
- Edges inside the chain (`member-failures` after `conductor-upgrade`, `snapshot-duration` after `setup-command`) are explained only in the earlier story's Notes. They are file-sharing orderings, so I did not raise them.
- I could not judge whether `dashboard.rs` (1241 lines) splits cleanly for the dashboard chain. Splitting it, for example into `dashboard/api.rs`, is the other option to the shared-file edges.

```findings
- file: .engineering/planning/story/dashboard-overview.md
  line: 25
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads repositories, pull requests with failing checks and releases that only the /api endpoints of story:dashboard-snapshot-api serve, the edge runs the other way (snapshot-api depends_on overview) and the tests use fixture JSON, so the overview is never shown against real rows; the overview should own those endpoints or depend_on story:dashboard-snapshot-api"
- file: .engineering/planning/story/dashboard-snapshot-api.md
  line: 32
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the set is cut by layer: one story declares all seven endpoints, including blockers, specifications and board that only later views read, and every view story tests against fixture JSON, so no story shows a view reading a served endpoint; each endpoint should move into the story that renders it"
- file: .engineering/planning/story/dashboard-snapshot-api.md
  line: 32
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "/api/board is declared as the snapshot views' rows but the Board record holds only a deviation count and a path, and the deviations are computed only inside status.rs figures(), so the overview chips, red-CI and failing counts and repo-view's waiting-on cases would be re-derived in TypeScript; the body should say the endpoint serves what status.rs computes, extracted and shared"
- file: .engineering/planning/story/dashboard-snapshot-api.md
  line: 10
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its depends_on edge to story:onboarding-run has no reason written beside it and queues repo-view, health and sandbox-spec behind the wave's last story through a ten-item chain; the shared surface is crates/conductor-model, so either keep the edge and record that file as its reason, or drop it and accept a regeneration collision on the model"
- file: .engineering/planning/story/dashboard-health.md
  line: 25
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads collector runs, member failures with fix text, doctor rows and the newest snapshot's state, but no story declares an endpoint serving them (snapshot-api serves only the newest complete snapshot), no story creates the fix text, and its scope has neither spec/domains/observation.yaml nor crates/conductor-model"
- file: .engineering/planning/story/dashboard-spa.md
  line: 39
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Notes put it after story:session-forge-credentials, but the graph puts it before (session-forge-credentials reaches sandbox-spec, which depends_on dashboard-health, which reaches dashboard-spa), so recording the stated order would close a cycle and contradicts the set being in v0.8.0 ahead of the sandbox work"
```
