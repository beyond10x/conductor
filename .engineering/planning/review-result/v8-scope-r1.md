---
format: aep.planning-md/3
id: review-result:v8-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, v0.8.0 additions
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

Two warnings, no blockers. Every promise I could trace in the epic is claimed by exactly one item.

- `story:dashboard-export` — the epic's Outcome promises a dashboard "served read-only on 127.0.0.1" (`.engineering/planning/epic/dashboard-for-engineers.md:23-24`). It says nothing about sharing, and the operator's two quoted sentences (lines 15-17) ask for neither a static export nor a shared copy. The only trace is the epic's Stories line (line 28). The story writes the dashboard out for a static host such as GitLab Pages (`.engineering/planning/story/dashboard-export.md:31`), which goes beyond "127.0.0.1" and has no operator sentence behind it. The fix is one of two: the epic's Outcome states shareable export and cites who asked, or the story is dropped.
- `story:conductor-upgrade` — the operator asked for "checks newer version and download from gh + check sha, etc...". The story also adds a `doctor` note about a newer release, backed by a cache `upgrade --check` writes (`.engineering/planning/story/conductor-upgrade.md:61-62`). It adds a `--version <x.y.z>` pin as well (line 46). Neither follows from check, download or verify, and the "etc..." is the only cover for them. The story should either cite why they are needed or drop them.

**Judgement of `story:conductor-upgrade` against the operator's request.** The request is covered and nothing is narrowed.
- **Check for a newer version:** `upgrade --check` reads `releases/latest`.
- **Download from GitHub:** done with `curl` rather than the `gh` CLI. The repository is public and the unauthenticated API returned 200, so this works.
- **Check the sha:** the archive is verified against `SHA256SUMS`, and a mismatch refuses and changes nothing.
- **Names match the release workflow:** the story's `conductor-<version>-<target>.tar.gz` and `SHA256SUMS` match `story:prebuilt-binaries` (implemented, `.github/workflows/release.yml`).

**Coverage of the epic.** I extracted 11 promises and traced all 11.

| Promise | Item |
|---|---|
| Vue single-page app embedded in the binary | `story:dashboard-spa` |
| Served read-only on 127.0.0.1 | `story:dashboard-spa` |
| What needs them | `story:dashboard-overview` |
| What each controller works on and waits on | `story:dashboard-repo-view` and `story:dashboard-overview` |
| Merge requests and pipelines | `story:dashboard-repo-view` and `story:dashboard-snapshot-api` |
| What shipped today | `story:dashboard-overview` and `story:dashboard-repo-view` |
| The plan | `story:dashboard-repo-view` |
| Health of their setup | `story:dashboard-health` |
| Operator-only items out of an engineer's view | `story:dashboard-engineer-view` |
| For the instance and for their repository | `story:dashboard-overview` and `story:dashboard-repo-view` |
| Snapshot data (repositories, releases, blockers, specifications, board) | `story:dashboard-snapshot-api` and `story:dashboard-repo-view` |

No two items claim the same outcome. The free-disk tile in the overview and the disk view in health differ in depth.

**What I read.** I read 10 artifacts (the epic, the seven dashboard stories, `story:conductor-upgrade` and `story:prebuilt-binaries`). I used `aep plan artifact show` on each and on the status heads of `story:session-forge-credentials`, `story:onboarding-run`, `story:setup-command`, `story:snapshot-duration` and `story:snapshot-member-failures`. I also ran `aep plan artifact graph` and read `release.yml`.

**What I could not establish.** All of these are out of my lane and did not set the verdict.
- **Next-release requirement:** the epic does not record the operator's "next release must include the updated dashboard". `story:dashboard-spa` Notes say "After the sandbox wave (`story:session-forge-credentials`), in the release after it". `story:session-forge-credentials` is still draft, and `story:dashboard-snapshot-api` depends on draft `story:onboarding-run`. That reads as a conflict with "next release", for the design critic.
- **Engineer view deferred:** with `story:dashboard-engineer-view` set for later, the next release serves operator-only items to everyone. The story exists, but whether that is acceptable is for the design critic.
- **"Mine" filter:** the overview acceptance names a "mine" filter but no item says how the viewer's identity is known. That is for the acceptance critic.
- **Raw ids and timestamps:** the epic's Why lists them as a problem (line 12), but no acceptance promises readable ones. The Outcome does not promise it either, so I did not count it as a gap.

```findings
- file: .engineering/planning/story/dashboard-export.md
  line: 31
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story writes a shareable static export for a static host such as GitLab Pages, but the epic's Outcome promises a dashboard 'served read-only on 127.0.0.1' (.engineering/planning/epic/dashboard-for-engineers.md:23-24) and the operator's quoted asks (lines 15-17) name no sharing or export; only the Stories line (line 28) lists it, so the epic's Outcome must promise it or the story must go"
- file: .engineering/planning/story/conductor-upgrade.md
  line: 61
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the operator asked to check for a newer version, download from GitHub and verify the sha, but the story also adds a doctor note on a newer release (with a cache written by upgrade --check, lines 61-62) and a --version <x.y.z> pin (line 46), neither traceable to the request beyond 'etc...'"
```
