---
format: aep.planning-md/3
id: review-result:forge-scope-r2
kind: review-result
status: active
title: Scope critic, round 2, epic forge-neutral
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
approve

Round 1 findings, each judged against the current bodies:

1. **Releases: does not hold.** `story:gitlab-merge-requests` now claims them in its third acceptance, "records each release published within the window as a release row". Its Why names `conductor repository shipped`. The finding asked for exactly that.
2. **Specification rows: does not hold.** `story:gitlab-repositories` now claims them. Its second acceptance reads "records one specification row per project", with a fixture test. Its scope and Why name `collect/specifications.rs`, including the `gh repo list` enumeration at `:158` and `:269`.
3. **`story:gh-optional` scope overlap: does not hold.** `story:gh-optional`'s scope no longer lists `README.md` or `website/docs/getting-started.md`. Its Notes say the README and website wording is `story:gitlab-guide`'s. Only `story:gitlab-guide` claims the README requirements sentence now.

Fresh pass, same four defects:

- **Gap:** none. I extracted 8 promises from the parent and traced all 8 to an item.
  1. GitLab forge, gitlab.com or self-hosted: `story:gitlab-source-host`.
  2. Snapshot rows (repository, pull request, merged pull request, release, workflow run, specification): `story:gitlab-repositories` and `story:gitlab-merge-requests`. I checked these against the `record.*` calls in `crates/conductor/src/collect`, and every forge-derived kind is claimed. The only other kind, sessions, is not forge data.
  3. Board: follows from the rows.
  4. Workspace: `story:gitlab-workspace`.
  5. Watch lines: `story:watch-gitlab-ci`.
  6. Guard: `story:guard-glab-writes`, with `story:profiles-forge-neutral` aligning the profiles.
  7. Through `glab`: the collector, watch and guard stories.
  8. `gh` needed only for a `github` source: `story:gh-optional`.
- **Reach beyond the parent:** none. The remaining `gh` call sites in `crates/conductor/src` are `watch.rs:259`, the `collect/` files, `guard/rules.rs:1781`, `guard/shell.rs:494` and `doctor.rs:25`. Each is claimed by one of the nine stories. The epic's Stories list names `story:profiles-forge-neutral` and `story:gitlab-guide`.
- **Two items claiming one outcome:** none. Hand-written docs are `story:gitlab-guide`'s alone. The shared spec, model and reference files are ordered by `depends_on` edges, not by overlapping outcomes.
- **Narrowed promise:** none.

**What you read:** 15 artifacts, using `aep plan artifact show` on the epic, the nine stories, the four same-release stories and `review-result:forge-scope-r1`. I also ran `aep plan artifact graph` and `rg` over `crates/conductor/src` for the record calls and `gh` call sites. I read `critic-rubric.md` and `plan-critic-scope.md`.

**What you could not establish:**
- The four same-release stories have no `decomposes` edge to the epic and trace to the adoption report, which I cannot read. I did not count them as reach, as in round 1.
- Out of my lane, for the acceptance critic: `story:gitlab-guide`'s Why mentions "`gh`-only CI and guard prose", and the Notes of `story:guard-glab-writes` and `story:doctor-minimum-versions` assign guard prose and pin wording to it. No acceptance there checks either (`docs_forge.rs` tests only the "not read by the binary" text, the README requirement sentences and the doctor sample line).
- Out of my lane, for the design critic: `.github/workflows/pages.yml` is named in `story:gitlab-guide`'s first acceptance but is not in its scope.

```findings
[]
```
