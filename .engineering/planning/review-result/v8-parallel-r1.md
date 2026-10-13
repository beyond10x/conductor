---
format: aep.planning-md/3
id: review-result:v8-parallel-r1
kind: review-result
status: active
title: Parallel critic, round 1, v0.8.0 additions
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

- story:dashboard-overview — its acceptance reads open MRs/PRs with failing checks, shipped today, and a repository table with "waiting on". `dashboard.rs` serves none of these in `/data.json`, because it reads no snapshot store. Those rows are what `story:dashboard-snapshot-api` creates (`/api/pull-requests`, `/api/repositories`, `/api/board`), and the only edge runs the other way (snapshot-api depends_on overview). Overview is therefore built against fixture JSON that its producer later fixes. The reader needs a `depends_on` edge to `story:dashboard-snapshot-api`, which means dropping snapshot-api's own edge to overview to avoid a cycle. Merging the two is the alternative. — .engineering/planning/story/dashboard-overview.md:25, .engineering/planning/story/dashboard-snapshot-api.md:32, crates/conductor/src/dashboard.rs:497-516
- story:snapshot-member-failures — the structured `scope` frontmatter is empty; the paths sit only in the body's `## Scope` section. So `aep plan artifact waves` omits the story (not in its output at all), places `fetch-without-prompts` as if the story were absent, and computes none of its collisions. It collides with `snapshot-duration` on `observation.yaml`, `crates/conductor-model`, `snapshot.rs` and `store/`, and with `dashboard-snapshot-api` on `observation.yaml` and the model. The surface is cited in the body, but no `scope` entries are set. — .engineering/planning/story/snapshot-member-failures.md:1-10, .engineering/planning/story/snapshot-member-failures.md:26-30
- story:snapshot-member-failures — its scope is named too widely and too narrowly. `crates/conductor/tests/` and `crates/conductor/src/store/` are whole directories. It omits `spec/components.yaml` and `crates/conductor/src/cli.rs`, though "a view lists those failures" must be placed in the `snapshot` group of the `cli:` block, which `cli_tree.rs` compares with `cli.rs`. `snapshot-duration` adds its own view to the same group list. — .engineering/planning/story/snapshot-member-failures.md:24, .engineering/planning/story/snapshot-member-failures.md:28-30, spec/components.yaml:130-163, crates/conductor/tests/cli_tree.rs:1-8
- story:snapshot-duration — the `snapshot collector-runs` view and the collector-run record are placed in the `snapshot` group. The scope omits `spec/components.yaml` (the same group list `story:snapshot-member-failures` edits) and `crates/conductor/src/store/`. Neither this body nor the other names that file. The chain edges order the two, but the shared file is unrecorded. — .engineering/planning/story/snapshot-duration.md:41, .engineering/planning/story/snapshot-duration.md:58-62, spec/components.yaml:130-163
- story:conductor-upgrade — its code, tests and spec would name `beyond10x/conductor` in the releases URL. `no_organization.rs` fails on that word in `crates/conductor/src`, `crates/conductor/tests` and `spec`, and its allow-list has no such use. The file is outside this scope, and `story:setup-guide-for-agents` already edits it. The body must either put the repository in config or `CARGO_PKG_REPOSITORY`, or add `tests/no_organization.rs` to scope and name the shared file. — .engineering/planning/story/conductor-upgrade.md:50, .engineering/planning/story/conductor-upgrade.md:71-76, crates/conductor/tests/no_organization.rs:4-6
- story:conductor-upgrade — the surface for "verifies the archive's sum, unpacks it" is unsettled (inferred). `crates/conductor/Cargo.toml` has no `sha2`, `flate2` or `tar` dependency. The body only rules out a TLS crate, so the choice is open: shell out to `sha256sum`/`shasum` and `tar` (as `SETUP.md` allows), or add crates. The crates path puts `crates/conductor/Cargo.toml` and `Cargo.lock` into collision with `story:dashboard-spa`'s `include_dir` change, which neither body names. The body should state which. — .engineering/planning/story/conductor-upgrade.md:52-54, .engineering/planning/story/conductor-upgrade.md:66, crates/conductor/Cargo.toml:15-27
- story:dashboard-snapshot-api — its only recorded reason for ordering is "after `story:dashboard-overview`". Its edge to `story:onboarding-run` has no stated reason. The files that justify it are missing from scope. A `url` field on `RecordPullRequest` and `RecordMergedPullRequest` adds a flag per input field (`cli_tree.rs`) in `cli.rs` (`RecordPullRequestArgs`, line 1308). It also changes the generated `website/docs/reference/cli.md` and the store code. Those are the files the chain head's stories edit, so the body should name the edge and its files. — .engineering/planning/story/dashboard-snapshot-api.md:32-39, crates/conductor/src/cli.rs:1306-1308, spec/domains/observation.yaml:695-720
- story:dashboard-repo-view — "decisions and escalations carry the repository they concern" adds an input field to the decision commands, and so a flag in `cli.rs` (`RaiseDecisionRequestArgs`, line 1713) and `website/docs/reference/cli.md`. Neither is in scope, though both are shared with the chain head. The body names only `dashboard.rs` as shared. — .engineering/planning/story/dashboard-repo-view.md:31, .engineering/planning/story/dashboard-repo-view.md:38-40, crates/conductor/src/cli.rs:1713
- story:dashboard-health — it reads `conductor doctor` rows, `snapshot collector-runs` and member failures, which come from `story:setup-command`, `story:snapshot-duration` and `story:snapshot-member-failures`. A `depends_on` path reaches them only through repo-view, snapshot-api and onboarding-run, and the body names the two snapshot stories without saying so. It also needs endpoints beyond what `story:dashboard-snapshot-api` declares. The scope lists no `spec/domains/observation.yaml` or model, which `snapshot-api` also edits, and no `doctor.rs`. — .engineering/planning/story/dashboard-health.md:25-32, .engineering/planning/story/dashboard-snapshot-api.md:32
- story:dashboard-spa — it replaces the server-rendered `/` page, which `instance_paths.rs` (`<h2>Sessions under …`) and `usage.rs` (`<h2>Token spend since …`, `<tr><td>`) assert on. It may also change `dashboard::serve` and `Sources`, which `gc.rs`, `instance_paths.rs` and `adv_watch.rs` call. None of those test files is in scope, so the body hides edits the implementor will find. — .engineering/planning/story/dashboard-spa.md:34, .engineering/planning/story/dashboard-spa.md:41-43, crates/conductor/tests/instance_paths.rs:432, crates/conductor/tests/usage.rs:650

**What I read.** 14 artifacts, with `aep plan artifact show` on each, `aep plan artifact waves --kind story --status draft` and `aep plan artifact graph`, plus `git grep`/`grep` over `crates/conductor`, `spec`, `Taskfile.yml`, `.github/workflows` and `Cargo.toml` in the wave-17 store.
- **Surfaces:** 13 cited from structured scope. `snapshot-member-failures` is cited from its body only, because its structured `scope` is empty. Inferred: 0. Unplaceable: 0, but that story is invisible to `waves`.
- **Acceptance reads traced to a producer in the set:** 11. Seven are recorded by a direct `depends_on` edge. Three are recorded only by a transitive path (the three in `dashboard-health`). One has no edge, the one reported against `dashboard-overview`.

**Not found, answering your questions:**
- `dashboard-spa` and `dashboard-overview` against the chain on `Taskfile.yml`, `.github/workflows/check.yml`, `crates/conductor/Cargo.toml` and `Cargo.lock`: only `dashboard-spa` claims these, and no other story in the set names them. The one possible exception is the `conductor-upgrade` dependency question above.
- They do not touch any file of the chain head, from `setup-guide-for-agents` through `onboarding-run`.
- Their collisions with the dashboard tail on `dashboard/`, `dashboard.rs` and `tests/dashboard.rs` are ordered by edges and named in each body.
- `snapshot-api` depends_on both `dashboard-overview` and `onboarding-run`, so the tail waits for both and spa/overview are not independent of the whole chain.
- The chain edges are linear and complete, and the body Notes of `doctor-identity`, `setup-command`, `conductor-upgrade` and `fetch-without-prompts` name the shared files.
- `story:spec-first-wording` shares no path with any other story in the set.

**What I could not establish:**
- Whether `store/observations.rs` needs edits for the `url` field. I inferred it from the story's wording and did not open the file.
- Whether the `dashboard-spa` note "after the sandbox wave, in the release after it" (`dashboard-spa.md:39`) is stale. It contradicts placing the story in wave 18. `sandbox-spec` depends_on `dashboard-health`, and the sandbox stories sit outside the set, so I make no finding. I note it as outside the set and as sequencing, which is the operator's call.
- Anything about whether acceptance lines are checkable, or whether the splits are right. Those belong to the other critics.

```findings
- file: .engineering/planning/story/dashboard-overview.md
  line: 25
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance reads open MRs/PRs with failing checks, shipped today and a repository table with waiting-on, which crates/conductor/src/dashboard.rs does not serve in /data.json and which story:dashboard-snapshot-api creates (/api/pull-requests, /api/repositories, /api/board), and the only edge runs snapshot-api -> overview, so the reader is built on fixtures before its producer; overview needs a depends_on edge to story:dashboard-snapshot-api (dropping the reverse edge to avoid a cycle) or the two are merged"
- file: .engineering/planning/story/snapshot-member-failures.md
  line: 1
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the frontmatter carries no scope, only the body's Scope section, so aep plan artifact waves omits the story and computes none of its collisions with story:snapshot-duration (observation.yaml, crates/conductor-model, snapshot.rs, store/) or story:dashboard-snapshot-api (observation.yaml, model); the body cites the surface and the scope entries are not set"
- file: .engineering/planning/story/snapshot-member-failures.md
  line: 28
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "scope names the whole directories crates/conductor/tests/ and crates/conductor/src/store/ and omits spec/components.yaml and crates/conductor/src/cli.rs, though a view must be placed in the snapshot group of the cli: block that cli_tree.rs compares with cli.rs and that story:snapshot-duration also edits"
- file: .engineering/planning/story/snapshot-duration.md
  line: 58
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the collector-runs view and run record are placed in the snapshot group of spec/components.yaml, which story:snapshot-member-failures also edits, yet scope omits spec/components.yaml and crates/conductor/src/store/ and neither body names the shared file"
- file: .engineering/planning/story/conductor-upgrade.md
  line: 50
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the releases URL names beyond10x in src, tests and spec, which crates/conductor/tests/no_organization.rs rejects and does not allow-list; that file is outside scope and is edited by story:setup-guide-for-agents, so the body must either avoid the word or add the file to scope and name the share"
- file: .engineering/planning/story/conductor-upgrade.md
  line: 52
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "inferred: sha256 verification and tar.gz unpacking have no stated mechanism and crates/conductor/Cargo.toml has no sha2/flate2/tar dependency; shelling out keeps Cargo.toml/Cargo.lock out of scope, adding crates collides with story:dashboard-spa (include_dir) on both files, and the body does not say which"
- file: .engineering/planning/story/dashboard-snapshot-api.md
  line: 37
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the depends_on edge to story:onboarding-run has no reason in Notes, and scope omits crates/conductor/src/cli.rs (RecordPullRequestArgs flag for the added url field), website/docs/reference/cli.md and the store code, which are the files that make it collide with the chain head"
- file: .engineering/planning/story/dashboard-repo-view.md
  line: 31
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "adding the repository to decisions adds an input field and so a flag in crates/conductor/src/cli.rs (RaiseDecisionRequestArgs) and website/docs/reference/cli.md, both shared with the chain head and absent from scope; Notes names only dashboard.rs as shared"
- file: .engineering/planning/story/dashboard-health.md
  line: 25
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the health view reads doctor rows, collector runs and member failures that stories in the chain create, reachable only by a transitive depends_on path the body does not state, and it needs endpoints beyond story:dashboard-snapshot-api's, so scope omits spec/domains/observation.yaml, crates/conductor-model and doctor.rs"
- file: .engineering/planning/story/dashboard-spa.md
  line: 41
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "replacing the server-rendered / page breaks crates/conductor/tests/instance_paths.rs and crates/conductor/tests/usage.rs, which assert its <h2> and <td> markup, and may change dashboard::serve/Sources used by gc.rs and adv_watch.rs; none of those test files is in scope"
```
