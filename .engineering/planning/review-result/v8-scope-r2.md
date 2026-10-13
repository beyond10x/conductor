---
format: aep.planning-md/3
id: review-result:v8-scope-r2
kind: review-result
status: active
title: Scope critic, round 2, v0.8.0 additions
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
approve

Neither round-1 finding still holds, and I found no new ones.

**Round-1 finding 1, `story:dashboard-export`.** It no longer holds.
- The epic's Outcome now ends "and can be shared read-only as a static export" (`.engineering/planning/epic/dashboard-for-engineers.md:25-26`). The Why adds "Engineers who do not run the instance need to see it too" (line 17). That is the first of round 1's two fixes, so the story's static export (`.engineering/planning/story/dashboard-export.md:31`, the `dashboard export --out` acceptance) traces to a sentence in the parent.
- I did not require the operator's name against that sentence. Whether the promise is the right one is not a scope question.

**Round-1 finding 2, `story:conductor-upgrade`.** It no longer holds, in both parts.
- The `doctor` note and its cache are gone. Notes says "Dropped: a `conductor doctor` note about a newer release" (`.engineering/planning/story/conductor-upgrade.md:91-92`), and no acceptance or scope entry mentions it. `doctor.rs` is not in scope.
- The `--version <x.y.z>` pin stays. The Why now cites the reason: "Installing a named older release (a rollback) is the same steps with another version, the 'etc.' of the request" (line 48). That is the other fix round 1 offered.
- The other parts of the request are covered:
  - Check: `upgrade --check` reads the latest release.
  - Download: archive and `SHA256SUMS` are fetched with `curl`.
  - Sha: the sum is verified, and a mismatch refuses and changes no file.
  - Names: they match `.github/workflows/release.yml` (`conductor-$VERSION-$TARGET`, `sha256sum *.tar.gz > SHA256SUMS`, lines 152 and 203).

**Coverage of the epic.** I extracted 12 promises and traced all 12 to an item. Round 1 had 11; the twelfth is the new static-export promise.

| Promise | Item |
|---|---|
| Vue single-page app embedded in the binary | `story:dashboard-spa` |
| Served read-only on 127.0.0.1 | `story:dashboard-spa` |
| What needs them, for the instance | `story:dashboard-overview` |
| For their repository | `story:dashboard-repo-view` |
| What each controller works on and waits on | `story:dashboard-repo-view`, with the controllers busy/idle/silent tile in `story:dashboard-overview` |
| Merge requests and pipelines | `story:dashboard-repo-view`, with the pull-request endpoints in `story:dashboard-overview` |
| What shipped today | `story:dashboard-overview` (releases) and `story:dashboard-repo-view` (today's events) |
| The plan | `story:dashboard-repo-view` (blockers, specifications) |
| Health of their setup | `story:dashboard-health` |
| Operator-only items out of an engineer's view | `story:dashboard-engineer-view` |
| Shared read-only as a static export | `story:dashboard-export` |
| Problems named in the Why: the stale "last event", the 85-row disk table, the unused snapshot data | the "last event" fix in `story:dashboard-overview`, the disk rows in `story:dashboard-health`, the snapshot endpoints in `story:dashboard-overview` and `story:dashboard-repo-view` |

- **Reach beyond the parent:** none. The `url` and `Decision.repository` spec changes, the `task ui` and CI gate, and the doctor cache each serve a promise above.
- **Two items claiming one outcome:** none. Free disk in the overview and the full disk rows in health differ in depth. The export story tests that the engineer view leaks nothing, but the filtering itself belongs to `story:dashboard-engineer-view`.
- **Narrowing:** none. The later stories (`story:dashboard-engineer-view`, `story:dashboard-export`) are named in the epic's Stories line, so the deferral is recorded.
- **Archived story:** `story:dashboard-snapshot-api` is archived with scope None, and every endpoint it held is claimed by exactly one live story.

**What I read.** 9 artifacts: the epic, the six dashboard stories, the archived `story:dashboard-snapshot-api`, `story:conductor-upgrade` and `review-result:v8-scope-r1`. I ran `aep plan artifact show` on each and `aep plan artifact graph`. I also read the epic file with line numbers and grepped `.github/workflows/release.yml` and the upgrade story.

**What I could not establish.**
- Out of my lane, and not setting the verdict: round 1's remarks to the design and acceptance critics. They are the draft `story:session-forge-credentials` and `story:onboarding-run` dependencies against "next release", operator-only items being served to everyone until the engineer view lands, and how the "mine" filter knows who the viewer is. I did not re-check them.
- The promise of a shareable export has no operator quote behind it, only the epic's own Why sentence. I treated that as outside the scope rule.

```findings
[]
```
