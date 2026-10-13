---
format: aep.planning-md/3
id: review-result:forge-design-r1
kind: review-result
status: active
title: Design critic, round 1, epic forge-neutral
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

- story:watch-gitlab-ci — its acceptance prints `main` red and green lines for a GitLab source's projects, but the watch picks those projects from the Active repositories of the newest complete snapshot (`real_commits_7d >= 1`), which only story:gitlab-repositories records for a gitlab source, and no depends_on edge runs from the watch to it — .engineering/planning/story/watch-gitlab-ci.md:25, .engineering/planning/story/gitlab-repositories.md:32, crates/conductor/src/watch.rs:221, crates/conductor/src/repository.rs:63
- story:gitlab-guide — its acceptance shows a config with a gitlab source and `host` and says what conductor reads from GitLab, but it depends only on story:gh-optional, so it can land beside story:gitlab-source-host (which introduces `host`) and the collector, workspace, watch and guard stories; add depends_on edges to story:gitlab-source-host, story:gitlab-merge-requests, story:gitlab-workspace, story:watch-gitlab-ci and story:guard-glab-writes — .engineering/planning/story/gitlab-guide.md:28, .engineering/planning/story/gitlab-source-host.md:26
- story:gh-optional — its scope names `collect/mod.rs` but no acceptance or note says what changes there, because `collect::programs` already omits `gh` without a `github` source, so the depends_on edge from story:gitlab-repositories rests on a change the body does not state, and nothing says which story adds `glab` to `programs()` — .engineering/planning/story/gh-optional.md:47, crates/conductor/src/collect/mod.rs:109

**What I read:** 14 artifacts, each whole body, with `aep plan artifact relations`, `graph` and `validate`.
- **Edges:** I walked 19 edges, 18 inside the set plus `story:gc-archive-profile` informed_by `story:gc-before-pause`, which is outside it. I found no cycle. The `depends_on` edges form a DAG: doctor-minimum-versions to doctor-verdict to gh-optional, and repositories to source-host and gh-optional.
- **Acceptance reads:** I traced 8 reads of produced state to a producer in the set, and 6 have an edge. The two without one are the watch and the guide findings above.
- **Code and spec:** `collect/mod.rs`, `collect/github.rs`, `watch.rs`, `repository.rs`, `doctor.rs`, `config.rs` `workspace_members` and `spec/domains/config.yaml` (`PrerequisiteState`, `GitLabSource`).
- **Validator:** it reports `valid`. Its remarks are "implemented on an assertion" notes on unrelated stories, which I did not restate.

**What I could not establish:**
- **Out of lane, for scope and acceptance:** `story:gh-optional` reads `gh: not needed`, but `conductor.config.PrerequisiteState` has no such variant (`spec/domains/config.yaml:454`). The story's scope omits that file, which `story:gitlab-source-host` and `story:gitlab-workspace` also edit. The second acceptance (a snapshot with no `github` source runs no `gh`) may already hold, since `programs()` and the owner loops are empty without owners.
- **Out of lane, for scope:** the specifications collector lists repositories through `gh repo list` per owner (`collect/specifications.rs:158`). I could not tell whether `story:gitlab-workspace` alone gives a GitLab instance specification rows.
- **Out of lane, for parallel safety:** `gitlab-repositories` and `gitlab-merge-requests` both edit `collect/gitlab.rs` and the fixtures. The edge records the order.
- **Unease, not a finding:** the GitLab pipeline-to-conclusion mapping is written twice, in `story:gitlab-merge-requests` (collector) and `story:watch-gitlab-ci` (watch), with no shared helper named.
- **Unease, not a finding:** `local_checkout` is name-based (`repositories.rs:24`), while `story:gitlab-workspace` matches by origin remote, so the two may disagree on which checkout belongs to a project.
- **Unease, not a finding:** `story:profiles-forge-neutral` will say `glab` is read-only (as `repo-controller.md:82` does for `gh`) with no edge to `story:guard-glab-writes`, which enforces it.
- **Unease, not a finding:** `story:gc-archive-profile` reads "the first checkout the workspace names", which for a gitlab-only instance depends on `story:gitlab-workspace`. Its fixture can avoid that.

```findings
- file: .engineering/planning/story/watch-gitlab-ci.md
  line: 25
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance prints `main` red and green lines for a GitLab source's projects, but the watch picks those projects from the Active repositories of the newest complete snapshot (real_commits_7d >= 1), which only story:gitlab-repositories records for a gitlab source, and no depends_on edge runs from the watch to it"
- file: .engineering/planning/story/gitlab-guide.md
  line: 28
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance shows a config with a gitlab source and `host` and says what conductor reads from GitLab, but it depends only on story:gh-optional, so it can land beside story:gitlab-source-host (which introduces `host`) and the collector, workspace, watch and guard stories; add depends_on edges to story:gitlab-source-host, story:gitlab-merge-requests, story:gitlab-workspace, story:watch-gitlab-ci and story:guard-glab-writes"
- file: .engineering/planning/story/gh-optional.md
  line: 47
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its scope names `collect/mod.rs` but no acceptance or note says what changes there, because `collect::programs` already omits `gh` without a `github` source, so the depends_on edge from story:gitlab-repositories rests on a change the body does not state, and nothing says which story adds `glab` to `programs()`"
```
