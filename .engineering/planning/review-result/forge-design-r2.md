---
format: aep.planning-md/3
id: review-result:forge-design-r2
kind: review-result
status: active
title: Design critic, round 2, epic forge-neutral
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
approve

All three round-1 findings are closed. I found no new defect of shape, and I report nothing as a finding.

**Round-1 findings**
- **`story:watch-gitlab-ci` and `story:gitlab-repositories` (blocker): no longer holds.** The watch now has `depends_on` edges to `story:gitlab-source-host` and `story:gitlab-merge-requests`, and the latter depends on `story:gitlab-repositories`. The watch's note names that transitive path. The status mapping it reuses is a single function in `collect/gitlab.rs`, written in `story:gitlab-merge-requests` (line 36), and that story's note says the watch reuses it.
- **`story:gitlab-guide` edges (warning): no longer holds.** It now depends on all nine stories it documents: gh-optional, source-host, repositories, merge-requests, workspace, watch, guard, doctor-verdict and doctor-minimum-versions. That covers the third acceptance, which reads the verdict line from `story:doctor-verdict`.
- **`story:gh-optional` and `collect/mod.rs` (warning): no longer holds.** The second acceptance (`gh-optional.md:42`) now says `collect::programs` asks for `glab` for a `gitlab` source. The `PrerequisiteState` change (`not needed`) is stated in the notes and listed in the scope. The edge from `story:gitlab-repositories` therefore rests on a stated change.

**Cycles.** There are none. The `depends_on` edges form a DAG:
- `story:doctor-minimum-versions` to `story:doctor-verdict` to `story:gh-optional` to `story:gitlab-source-host`.
- `story:gitlab-repositories` to source-host and gh-optional, then `story:gitlab-merge-requests`, then `story:watch-gitlab-ci`, then `story:gitlab-guide`.
- `story:gitlab-workspace` to source-host and repositories.
- `story:profiles-forge-neutral` to `story:guard-glab-writes`.

**Serialising chain.** The longest chain is six deep: source-host, gh-optional, repositories, merge-requests, watch, guide. Nearly every edge on it exists because two stories edit the same file, and the body beside the edge says so. `collect/gitlab.rs`, `collect/mod.rs`, `spec/domains/config.yaml` and `doctor.rs` are the shared files. The alternative would be splitting those surfaces, which the drafter did not choose, so I report no finding. Four stories sit outside the chain: `story:guard-glab-writes`, `story:version-flag` and `story:gc-archive-profile` are free of any other story in the set, and `story:profiles-forge-neutral` waits only on the guard. The doctor pair runs beside the collector stories.

**Splits.** The collector is cut by recorded rows. Repositories and specifications are in one story, and pull requests, releases and pipelines in the other. Each can be demonstrated alone with a fake `glab`. The guard and profile stories are split by enforcement and wording, with the edge recorded. I found no split abstraction or horizontal slice.

**Rubric part 3 (what I read).** 13 stories and the epic, each whole body, plus `aep plan artifact relations`, `graph`, `validate` and `waves`. `validate` reports `valid`; its only remarks are assertion-closure notes on unrelated stories. I walked 28 `depends_on` or `informed_by` edges. That includes the one outside the set, `story:gc-archive-profile` informed_by `story:gc-before-pause`, and it also covers the edges to the epic. I traced 13 acceptance reads of state another story in the set produces, and an edge records all 13. They are the watch and repository rows, the guide against its nine stories and the doctor verdict line, and the doctor pair on `doctor.rs`.

**Rubric part 4 (what I could not establish)**
- **Unease, not a finding.** `story:gitlab-repositories` records `local_checkout` and reads a checkout's `spec/` directory without saying how a project maps to a checkout. `story:gitlab-workspace` matches by origin remote on the source's host and group. The existing GitHub `local_checkout` is name-based, so the two stories could disagree on which checkout a GitLab project owns.
- **Out of lane, for scope.** `story:version-flag`, `story:gc-archive-profile`, `story:doctor-verdict` and `story:doctor-minimum-versions` are not in the epic's Stories list, and the doctor pair has no `decomposes` edge. Whether the set covers its source belongs to the scope critic.
- **Out of lane, for acceptance.** Whether `gh: not needed` and the three doctor cases are checkable together with the verdict line is the acceptance critic's question.

```findings
[]
```
