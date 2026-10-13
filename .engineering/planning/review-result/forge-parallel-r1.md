---
format: aep.planning-md/3
id: review-result:forge-parallel-r1
kind: review-result
status: active
title: Parallel-safety critic, round 1, epic forge-neutral
relations:
- reviews: story:gitlab-source-host
- reviews: story:gitlab-repositories
- reviews: story:gitlab-merge-requests
- reviews: story:gitlab-workspace
- reviews: story:watch-gitlab-ci
- reviews: story:guard-glab-writes
- reviews: story:gh-optional
- reviews: story:profiles-forge-neutral
- reviews: story:gitlab-guide
- reviews: story:doctor-verdict
- reviews: story:doctor-minimum-versions
- reviews: story:version-flag
- reviews: story:gc-archive-profile
revision: 1
---
needs-revision

- story:gitlab-guide — its acceptance shows a config with `host` and says what conductor reads from GitLab, but its only `depends_on` is `story:gh-optional`, so no edge runs to `story:gitlab-source-host` (creates `host`) or to the collector and watch stories (`gitlab-repositories`, `gitlab-merge-requests`, `gitlab-workspace`, `watch-gitlab-ci`) whose reads it documents; add those edges or merge the items — .engineering/planning/story/gitlab-guide.md:28, .engineering/planning/story/gitlab-source-host.md:25
- story:doctor-minimum-versions — it and `story:doctor-verdict` both land on `crates/conductor/src/doctor.rs` and `crates/conductor/tests/doctor.rs` (cited, both scopes); the `depends_on` edge exists but neither body says why, and this body has no Notes section naming the shared files — .engineering/planning/story/doctor-minimum-versions.md:32
- story:doctor-minimum-versions — its scope cites `crates/conductor/build.rs`, which does not exist yet and is not marked new, and omits `crates/conductor/Cargo.toml` (it has no build script or build-dependencies today) and the docs that state the pin behaviour (`README.md:90`, `website/docs/getting-started.md:56`, `website/docs/guides/troubleshoot.md:20`) — .engineering/planning/story/doctor-minimum-versions.md:32
- story:doctor-verdict — the sample output it changes (the `pins:` last line) is printed in `website/docs/getting-started.md:52-61` and described at `README.md:90`, which are in neither this scope nor an edge-ordered claim: `story:gh-optional` and `story:gitlab-guide` also land on those files, and no edge runs between doctor-verdict and gitlab-guide (inferred); the `conductor doctor` description in `crates/conductor/cli.yaml` and generated `website/docs/reference/cli.md:124` are also unlisted — .engineering/planning/story/doctor-verdict.md:31
- story:gitlab-source-host — its scope omits generated `crates/conductor-model/` (`GitLabSource` at `crates/conductor-model/src/config.rs:149`, which `task check` diffs against a fresh `ess generate synthesize`, `Taskfile.yml:314`), generated `website/docs/reference/config.md:154` (docs-check, "the binary does not read that forge"), and `crates/conductor/tests/grouped_instance.rs:677-709`, which asserts `config show` prints a gitlab source without `host`; none of the three is named — .engineering/planning/story/gitlab-source-host.md:36
- story:gitlab-workspace — changing the `WriteWorkspace` comment in `spec/domains/config.yaml` regenerates `crates/conductor-model/` and `website/docs/reference/config.md`, which `story:gitlab-source-host` also lands on, and the body names neither; `website/docs/concepts/instances-and-config.md:111` ("for a `gitlab` source, none") goes stale too — .engineering/planning/story/gitlab-workspace.md:38
- story:gitlab-repositories — the existing `crates/conductor/tests/grouped_instance.rs:677-690` (`a_gitlab_source_asks_nothing…`, asserts empty repository rows) contradicts this acceptance, and so would the `programs()` cases in `crates/conductor/tests/instance_sources.rs` once `glab` joins them; neither is in scope. `website/docs/concepts/the-loop.md:53` and `docs/design/conductor.md:108` ("not read by the binary") are unlisted as well — .engineering/planning/story/gitlab-repositories.md:45
- story:gh-optional — the second acceptance's test is "a test in `crates/conductor/tests/`" with no file named and none in scope; `crates/conductor/tests/grouped_instance.rs:686` already holds "gh is asked nothing" for a gitlab source, so that is probably the file, which `story:gitlab-source-host` also changes and no edge orders against gh-optional (inferred) — .engineering/planning/story/gh-optional.md:36
- story:watch-gitlab-ci — it and `story:gitlab-repositories` both need the `gh`-only statements in `website/docs/concepts/the-loop.md:93` and `docs/design/conductor.md:293` (and the sibling lines 39-40 and 102-103) corrected, those files are in neither scope, and no edge runs between watch-gitlab-ci and the collector stories (inferred) — .engineering/planning/story/watch-gitlab-ci.md:34
- story:guard-glab-writes — the guard prose lives in `README.md:34,37`, `docs/design/conductor.md:221,269` and `website/docs/concepts/decisions-and-messages.md:41-43` ("any `gh` call that is not a read"), none in scope. If these are updated, `README.md` is a third claimant beside `story:gh-optional` and `story:gitlab-guide` with no edge (inferred) — .engineering/planning/story/guard-glab-writes.md:38

What I read: 13 stories, and the epic only through the graph. I ran `aep plan artifact waves --kind story --status draft` and `aep plan artifact graph`, and read the bodies with `cat`. I checked the tree with `git grep` and by reading `Taskfile.yml`, `crates/conductor/src/{doctor.rs,collect/mod.rs,dashboard/waste.rs}`, `crates/conductor-model/src/config.rs`, `crates/conductor/tests/grouped_instance.rs` and `website/docs/**`. Surfaces: 13 cited, 0 inferred, 0 unplaced. Of the 10 acceptance reads I traced to a producer in the set, 8 are recorded by a `depends_on` edge. The two missing are `gitlab-guide` reading `host` from `gitlab-source-host` and the collector and watch stories.

Pairs from the waves list that are ordered by an edge and named in the bodies:
- `gh-optional` with `doctor-verdict` and with `doctor-minimum-versions` on `doctor.rs` (the doctor-verdict to doctor-minimum-versions pair is the exception, flagged in the second finding).
- `gh-optional` with `gitlab-guide` on README and getting-started.
- `gh-optional` with `gitlab-repositories` and `gitlab-merge-requests` on `collect/mod.rs` (the merge-requests edge is transitive).
- `gitlab-merge-requests` with `gitlab-repositories`.
- `gitlab-source-host` with `gitlab-workspace`.

The `watch-unsent-report` collision on `watch.rs` and the other `waves` collisions are with items outside the set, so I did not judge them. `version-flag`, `profiles-forge-neutral` and `gc-archive-profile` collide with nothing in the set and look complete for their own acceptance.

What I could not establish:
- Whether `docs-check` or a test fails on the stale hand-written pages (`the-loop.md`, `instances-and-config.md`), as opposed to the generated ones. I rated those doc omissions as warnings for that reason.
- Out of my lane, for the acceptance critic:
  - `gc-archive-profile` changes the dry run only, but `waste.rs:341` also passes `sources.root` on the apply path.
  - `profiles-forge-neutral` says "5 mentions each", but `git grep -E '\bgh\b'` finds one in each profile.
  - `gh-optional`'s second acceptance may already hold, since `collect::programs` and `grouped_instance.rs:686` already skip `gh` for a gitlab-only instance.

```findings
- file: .engineering/planning/story/gitlab-guide.md
  line: 28
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance shows a config with `host` (created by story:gitlab-source-host) and says what conductor reads from GitLab (created by gitlab-repositories, gitlab-merge-requests, gitlab-workspace, watch-gitlab-ci), but the only depends_on is story:gh-optional; add the missing edges or merge the items"
- file: .engineering/planning/story/doctor-minimum-versions.md
  line: 32
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it shares crates/conductor/src/doctor.rs and crates/conductor/tests/doctor.rs with story:doctor-verdict (cited, both scopes); the depends_on edge exists but no body names the shared files or gives them as the reason"
- file: .engineering/planning/story/doctor-minimum-versions.md
  line: 32
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "scope cites crates/conductor/build.rs, which does not exist yet and is not marked new, and omits crates/conductor/Cargo.toml (no build script or build-dependencies there today) and the docs stating the pin behaviour (README.md:90, website/docs/getting-started.md:56, website/docs/guides/troubleshoot.md:20)"
- file: .engineering/planning/story/doctor-verdict.md
  line: 31
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the sample doctor output it changes is printed in website/docs/getting-started.md:52-61 and README.md:90, files in neither this scope nor edge-ordered against story:gitlab-guide, which also lands on them (inferred); the `conductor doctor` description in cli.yaml and generated website/docs/reference/cli.md:124 are unlisted too"
- file: .engineering/planning/story/gitlab-source-host.md
  line: 36
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "scope omits generated crates/conductor-model (diffed by task check, Taskfile.yml:314), generated website/docs/reference/config.md:154 (docs-check), and crates/conductor/tests/grouped_instance.rs:677-709, which asserts config show prints a gitlab source without host"
- file: .engineering/planning/story/gitlab-workspace.md
  line: 38
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the spec comment change regenerates crates/conductor-model and website/docs/reference/config.md, which story:gitlab-source-host also lands on, and the body names neither; website/docs/concepts/instances-and-config.md:111 goes stale too"
- file: .engineering/planning/story/gitlab-repositories.md
  line: 45
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "crates/conductor/tests/grouped_instance.rs:677-690 (asserts empty gitlab repository rows) and the programs() cases in crates/conductor/tests/instance_sources.rs contradict this acceptance and are not in scope; website/docs/concepts/the-loop.md:53 and docs/design/conductor.md:108 (gitlab not read by the binary) are unlisted"
- file: .engineering/planning/story/gh-optional.md
  line: 36
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the second acceptance's test names no file and none is in scope; it is probably crates/conductor/tests/grouped_instance.rs:686, which story:gitlab-source-host also changes with no edge between them (inferred surface)"
- file: .engineering/planning/story/watch-gitlab-ci.md
  line: 34
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the gh-only CI statements in website/docs/concepts/the-loop.md:93 and docs/design/conductor.md:293 need changing, are in no scope, and are shared with the collector stories with no edge to them (inferred surface)"
- file: .engineering/planning/story/guard-glab-writes.md
  line: 38
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the guard prose in README.md:34,37, docs/design/conductor.md:221,269 and website/docs/concepts/decisions-and-messages.md:41-43 says gh only and is in no scope; README.md would be a third claimant beside story:gh-optional and story:gitlab-guide with no edge (inferred surface)"
```
