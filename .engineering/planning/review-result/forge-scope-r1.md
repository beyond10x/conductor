---
format: aep.planning-md/3
id: review-result:forge-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, epic forge-neutral
relations:
- reviews: epic:forge-neutral
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

- epic:forge-neutral — "the same snapshot rows … as a GitHub instance" is promised, and the GitHub `github` collector's `RecordRelease` rows (releases in the 14-day window, which `conductor repository shipped` reads) are claimed by no item; `story:gitlab-merge-requests` records only open, merged and workflow-run rows, and `story:gitlab-repositories` records a release tag inside the repository row, which is not a release row. `story:gitlab-merge-requests` would most naturally take it. — .engineering/planning/epic/forge-neutral.md:21
- epic:forge-neutral — "the same snapshot rows … as a GitHub instance" is also promised for the specifications collector's `RecordSpecification` rows, and no item claims them. `enumerated()` lists repositories only from `gh repo list` per GitHub owner and from `local` sources, so a `gitlab` source yields none even once `story:gitlab-workspace` writes members. That story's Why says the specifications collector "see[s] none", but its acceptance covers only the workspace file. `story:gitlab-repositories` or `story:gitlab-workspace` would most naturally take it. — .engineering/planning/epic/forge-neutral.md:21
- story:gh-optional — its scope names `README.md` and `website/docs/getting-started.md`, but no acceptance claims a change there. The README requirements sentence (`gh` for a GitHub source, `glab` for a GitLab one) is claimed by `story:gitlab-guide`, so the same outcome sits in two scopes with one acceptance. The body should either drop those two files or claim the `gh`-optional wording and leave `glab` to the guide. — .engineering/planning/story/gh-optional.md:48

**What you read:** 14 artifacts: the epic, its nine stories and the four release stories. I ran `aep plan artifact show` on each and `aep plan artifact graph`. I also read `critic-rubric.md` and `plan-critic-scope.md`. I checked the code the promises touch: `collect/github.rs`, `collect/specifications.rs`, `collect/mod.rs`, `shipped.rs`, `watch.rs`, `guard/rules.rs`, and the remaining `gh` uses in `crates` found with `rg`.

**Promise count:** I extracted 8 promises from the parent and traced 6 to an item.
1. GitLab forge, gitlab.com or self-hosted: `story:gitlab-source-host`.
2. Same snapshot rows: partial. Repositories, pull requests, merged pull requests and workflow runs are claimed; releases and specifications are not (findings 1 and 2).
3. Board: follows from the snapshot rows, so it carries the same gaps.
4. Workspace: `story:gitlab-workspace`.
5. Watch lines: `story:watch-gitlab-ci`.
6. Guard: `story:guard-glab-writes`.
7. Through `glab`: the collector, watch and guard stories.
8. `gh` needed only by a `github` source: `story:gh-optional`.

**What you could not establish:**
- The four stories with no epic (`story:doctor-verdict`, `story:doctor-minimum-versions`, `story:version-flag`, `story:gc-archive-profile`) trace to the adoption report, which I cannot read here. They do not decompose the epic, so I did not count them as reach beyond it. `story:doctor-verdict` and `story:doctor-minimum-versions` stay in `story:gh-optional`'s dependency chain and do not claim anything the epic promises.
- `story:profiles-forge-neutral` and `story:gitlab-guide` are named in the epic's Stories list, but its Outcome does not mention profiles or documentation. I treated that as traceable and did not make it a finding.
- Out of my lane:
  - Whether `story:gh-optional`'s second acceptance is already true, since `collect::programs()` pushes `gh` only when owners exist (acceptance critic).
  - Whether `story:guard-glab-writes` carries over the class-H credential rules that `gh` has (design critic).
  - Whether `story:gitlab-source-host` should land before `story:gh-optional` (parallel-safety critic).

```findings
- file: .engineering/planning/epic/forge-neutral.md
  line: 21
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"the same snapshot rows ... as a GitHub instance\" is promised and no item claims the RecordRelease rows the github collector records (releases in the window, read by `conductor repository shipped`); story:gitlab-merge-requests would most naturally take it"
- file: .engineering/planning/epic/forge-neutral.md
  line: 21
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"the same snapshot rows ... as a GitHub instance\" is promised and no item claims the specifications collector's RecordSpecification rows for a gitlab source, whose enumeration reads only gh owners and local sources (crates/conductor/src/collect/specifications.rs:269); story:gitlab-repositories or story:gitlab-workspace would most naturally take it"
- file: .engineering/planning/story/gh-optional.md
  line: 48
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the scope names README.md and website/docs/getting-started.md but no acceptance claims a change there, while story:gitlab-guide's acceptance claims the README requirements sentence; the two scopes overlap on one outcome that only one acceptance states"
```
