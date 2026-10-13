---
format: aep.planning-md/3
id: review-result:v8-design-r2
kind: review-result
status: active
title: Design critic, round 2, v0.8.0 additions
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
revision: 1
---
needs-revision

Five of the six round-1 findings are fixed. The sixth was fixed in part: the same defect now sits in `story:dashboard-repo-view` and `story:dashboard-overview`. These are the three findings that remain.

**Round 1, finding by finding**
- **#1 overview reads rows only snapshot-api served: fixed.** `story:dashboard-overview` now owns `/api/board`, `/api/repositories`, `/api/pull-requests`, `/api/workflow-runs` and `/api/releases` (acceptance, "Read-only endpoints"). Its own tests run against a fixture snapshot, not stand-in JSON.
- **#2 horizontal slice: fixed.** `story:dashboard-snapshot-api` is archived with no scope. Overview, repo-view and health each carry their spec change, endpoints, app view and tests. Each acceptance holds the fixture keys to the served endpoints with a Rust test.
- **#3 `/api/board` against the private `figures()`: fixed.** Overview makes `status.rs::figures()` shared. Both `publish-board` and `/api/board` call it, and a test compares `/api/board`'s deviations with those in the written `STATUS.md`. The code supports this: `figures(&Observed)` is pure over snapshot rows, and `RepositoryFigures` is in scope in `components.yaml`, `observation.yaml` and the model.
- **#4 `snapshot-api` → `onboarding-run` edge with no reason: fixed.** The edge went with the archived story.
  - The remaining edge, overview → `onboarding-run`, has its reason beside it in overview's Notes (the `url` field regenerates the model, `cli.rs` and `reference/cli.md`).
  - The ten-story chain is real, but each link now names the shared file that justifies it, so by the rubric it is not a finding.
- **#5 health reads endpoints nobody serves: fixed.** `story:dashboard-health` declares `/api/health` and the `FailureClass` enum. The fix text is one table in `dashboard.rs`, keyed by class. Scope now holds `observation.yaml`, the model and `doctor.rs`. Edges run directly to `setup-command`, `snapshot-member-failures` and `snapshot-duration`.
- **#6 SPA Notes against the graph: fixed.** `story:dashboard-spa` Notes now say it comes before `sandbox-spec` (through health) and that only `engineer-view` waits for `session-forge-credentials`. This matches the graph.

**New or remaining findings**
- **repo-view reads data no endpoint serves.** This is round-1 #5 again, in a different story. `/data.json` has session rows with name, kind, state, cwd and dispatch; decisions as id, class, choice and reason (no time, no repository); and spend per repository. It has no per-session context, no box, no transcript path and no event list. A "box" exists only in later sandbox stories.
  - The story's `/api/*` list is blockers, specifications and waiting-on.
  - The fix is to declare the endpoint or endpoints for these parts, say what each row holds, and put the files in scope.
- **overview's "timeline of today" has no named source.** It has the same gap, and it is the earlier story, so repo-view's endpoint would run backwards through the graph. Overview should name the endpoint for the rows, or the event endpoint should move into overview.
- **overview's "waiting" filter has no named source.** The only waiting data is `/api/waiting-on`, which repo-view creates after overview. Overview's own Notes give the "waiting on" column to repo-view. The body should say what "waiting" reads from `/api/board`, or the filter should move to repo-view.

**What I read:** 14 artifacts in full with `aep plan artifact show`: the epic, the eight set stories, the archived `dashboard-snapshot-api`, `dashboard-engineer-view`, `dashboard-export` and `onboarding-run`. I also read `review-result:v8-design-r1`, and ran `aep plan artifact relations`, `graph` and `validate`. `validate` printed `valid`, plus its five "closed on an assertion" notes, which are not mine. I checked `crates/conductor/src/dashboard.rs` (`collect`, `session`, `dispatch`, `recent`, `watch`) and `status.rs` (`publish_board`, `observed`, `figures`) against the bodies.

**Graph:** I walked all 128 `depends_on` and `blocks` edges in the store, outside the set as well, with a depth-first search, and found no cycle. The set's chain runs `setup-command` → `conductor-upgrade` → `snapshot-member-failures` → `fetch-without-prompts` → `snapshot-duration` → `onboarding-run` → `overview` → `repo-view` → `health` → `sandbox-spec`. `dashboard-spa` hangs off `conductor-upgrade`. I traced about 20 acceptance reads to a producer in the set. 17 are recorded by an edge. The 3 that no edge records are the ones in the findings above.

**What I could not establish:**
- **Out of my lane, not counted:** four bodies cite `docs/design/dashboard-mockup.html` as the design reference, and the file is not in the tree (`git ls-files` and `git log --all` find nothing).
- **Out of my lane, not counted:** the SPA gate (Node pin in `check.yml` and `task check`, the freshness check stated twice) belongs to the acceptance and parallel-safety critics. Round 1 left it there as well.
- **Not raised, an unease:** overview also carries the "last event is the newest report" fix, which is a `/data.json` change unrelated to the snapshot endpoints. It is small and I could not name a seam.
- **Not raised, an unease:** `story:onboarding-run` still says it is "last in the wave". Health edits `doctor.rs` after it. AGENTS.md says to repeat the onboarding runs after any `doctor` change. That text is in `onboarding-run`, which is outside the set.
- **Not judged:** whether `dashboard.rs` (1241 lines) could be split to loosen the chain, or whether the unbounded set of per-repository waiting-on rules is feasible.

```findings
- file: .engineering/planning/story/dashboard-repo-view.md
  line: 71
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the repository page's sessions part (context, box, spend, transcript path) and today's events part (dispatches, reports and decisions since 00:00) read data that no declared endpoint serves and that /data.json does not hold (its decisions carry no time or repository, its sessions no context, box or transcript path), so the body should declare the endpoint for them, say what its rows hold, and put the files in scope"
- file: .engineering/planning/story/dashboard-overview.md
  line: 76
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the overview's 'timeline of today' names no endpoint, and /data.json holds only each session's newest dispatch event and decisions without a time, so the rows have no producer and repo-view's later events endpoint would run against the edge; the body should name the endpoint that serves the timeline or take over the events endpoint"
- file: .engineering/planning/story/dashboard-overview.md
  line: 74
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the repository table's 'waiting' filter reads waiting-on data that only story:dashboard-repo-view's /api/waiting-on creates, after this story, and the Notes give the 'waiting on' column to repo-view; the body should say what the filter reads from /api/board, or the filter should move to repo-view"
```
