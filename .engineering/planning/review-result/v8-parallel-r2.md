---
format: aep.planning-md/3
id: review-result:v8-parallel-r2
kind: review-result
status: active
title: Parallel critic, round 2, v0.8.0 additions
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

- story:snapshot-member-failures — its acceptance (a member a collector cannot read no longer fails the snapshot) reverses `a_fetch_that_fails_twice_fails_the_snapshot_naming_the_repository_and_the_command` and `an_ess_that_fails_twice_fails_the_snapshot_naming_the_repository_and_the_command`, which assert `SnapshotState::Failed` and an empty `case.blockers()`. Both are in `crates/conductor/tests/collect_stores.rs` (cited, tests at :501-533 and :1212-1226). That file is in no scope entry or note of this story, but `story:fetch-without-prompts` edits it, and neither body mentions the other on that file. The remedies are to add `collect_stores.rs` to this story's scope and say the two cases are rewritten here, or to move the rewrite into `story:fetch-without-prompts` and say so in both. — .engineering/planning/story/snapshot-member-failures.md:50-52, crates/conductor/tests/collect_stores.rs:501-533

**Round 1, finding by finding** (all ten were fixed in the plan revision):
1. `dashboard-overview`, reader beside its producer: no longer holds. Overview now creates `/api/board`, `/api/repositories`, `/api/pull-requests`, `/api/workflow-runs` and `/api/releases` itself (`dashboard-overview.md:44-50`). `story:dashboard-snapshot-api` is archived. Overview depends_on `dashboard-spa` and `onboarding-run`.
2. `snapshot-member-failures`, empty structured scope: no longer holds. The scope has 11 entries, and `waves` places the story in wave 17 with its collisions computed.
3. `snapshot-member-failures`, scope too wide and too narrow: no longer holds. The whole directories are replaced by `member_failures.rs` (new) and the store files, and `spec/components.yaml` and `cli.rs` are added. The remaining `collect_stores.rs` gap is the new finding above.
4. `snapshot-duration`, `components.yaml` and `store/` unrecorded: no longer holds. Both are in scope, and Notes at `snapshot-duration.md:63-67` name every shared file against `snapshot-member-failures`.
5. `conductor-upgrade`, `no_organization.rs`: no longer holds. The URL comes from `env!("CARGO_PKG_REPOSITORY")`, set in the root `Cargo.toml`, with `repository.workspace = true` in `crates/conductor/Cargo.toml`. The word never appears in the scanned files, and the test passes unchanged (`conductor-upgrade.md:52-58`).
6. `conductor-upgrade`, unsettled mechanism: no longer holds. It shells out to `curl`, `sha256sum`/`shasum` and `tar`, adds no crate, and `Cargo.lock` is unchanged. Its Notes name the `crates/conductor/Cargo.toml` order against `dashboard-spa`.
7. `dashboard-snapshot-api`: no longer applies. The story is archived and its endpoints moved to overview and repo-view.
8. `dashboard-repo-view`, `cli.rs` and `reference/cli.md`: no longer holds. Both are in scope, with `decide.rs`, `decision.yaml` and `tests/decision.rs`, and Notes order the shared files through overview after `onboarding-run`.
9. `dashboard-health`, transitive reads: no longer holds. Health has direct depends_on edges to `setup-command`, `snapshot-member-failures` and `snapshot-duration`. `observation.yaml`, the model and `doctor.rs` are in scope. It serves its own `/api/health`.
10. `dashboard-spa`, test files: no longer holds. `instance_paths.rs` and `usage.rs` are in scope, and the acceptance moves their `<h2>` and `<td>` assertions to `/data.json`.

**What I read.** 13 stories (`setup-guide-for-agents`, `doctor-identity`, `setup-command`, `conductor-upgrade`, `snapshot-member-failures`, `fetch-without-prompts`, `snapshot-duration`, `onboarding-run`, `dashboard-spa`, `dashboard-overview`, `dashboard-repo-view`, `dashboard-health`, `spec-first-wording`), plus `review-result:v8-parallel-r1`. Commands: `aep plan artifact show` on the review and `story:dashboard-snapshot-api` and `story:doctor-minimum-versions`, the story bodies read from the files, `aep plan artifact waves --kind story --status draft` (the release's items sit in waves 1, 2, 13 and 15-23; 1510 collisions overall), and `git grep`/`grep` over `crates/conductor`, `spec`, `Cargo.toml` and `README.md`.
- Surfaces: 13 cited, 0 inferred, 0 unplaceable. None of the 13 is among the 11 `unassessed` stories in the `waves` output.
- Acceptance reads traced to a producer in the set: 11. Ten are recorded by a direct `depends_on` edge. One is recorded only transitively: `conductor-upgrade`'s SETUP.md and README edits and `docs_setup.rs`'s CLI-tree check, through `setup-command`.
- Unordered pairs in the chain: only `dashboard-spa` against `snapshot-member-failures`, `fetch-without-prompts`, `snapshot-duration` and `onboarding-run`, plus `spec-first-wording` against all. `waves` lists no shared path for any of them. `spec-first-wording` shares no path with the other 12 stories.

**What I could not establish or leave outside my lane:**
- `dashboard-overview` also edits `snapshot.rs` (`record_pull_request` at :659-693, the `pull-requests` view at :1047), which its scope omits. Every other story in the set that edits it is ordered before overview by the chain, so no two can run at once and it is not a finding.
- `dashboard-spa`'s structured scope names `dist` and `src` as bare root paths and omits `.gitignore`, which its body's Scope section lists. No story in the set shares these, so it is cosmetic here.
- `onboarding-run.md` calls itself "last in the wave", but `dashboard-overview` and `dashboard-repo-view` come after it and edit the same `cli.rs`, model and `reference/cli.md`. The edges order them, so this is only wording.
- `docs/design/dashboard-mockup.html` is cited as the design reference by `dashboard-spa`, `dashboard-overview`, `dashboard-repo-view` and `dashboard-health`, and `docs/design/` holds only `conductor.md`. I did not search other branches for it, so I could not tell which story creates it. This is acceptance-critic territory.
- `fetch-without-prompts` case (d) reads a failed member's blockers from its last fetch. `snapshot-member-failures` records that member as a failure with no rows. Whether the two behaviours agree is the design critic's question, not mine.

```findings
- file: .engineering/planning/story/snapshot-member-failures.md
  line: 50
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance that a member a collector cannot read no longer fails the snapshot reverses the cases a_fetch_that_fails_twice_fails_the_snapshot_naming_the_repository_and_the_command and an_ess_that_fails_twice_fails_the_snapshot_naming_the_repository_and_the_command in crates/conductor/tests/collect_stores.rs (cited, :501-533 and :1212-1226), a file that is in no scope entry or note of this story but is edited by story:fetch-without-prompts, and neither body names the share; add collect_stores.rs to this story's scope and say the two cases are rewritten here, or move the rewrite to story:fetch-without-prompts and say so in both"
```
