---
format: aep.planning-md/3
id: review-result:v8-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, v0.8.0 additions
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

- story:conductor-upgrade — the acceptance says the command "runs the unpacked binary's `--version`" before the rename, but no listed test or declared outcome has an archive whose binary does not run, so that step passes whether or not it exists — .engineering/planning/story/conductor-upgrade.md:66
- story:dashboard-overview — the repository table's "waiting" filter (one of "all, mine, waiting, red CI") names no rule for which repositories are waiting, and the only data for that, `/api/waiting-on`, arrives with `story:dashboard-repo-view`, which comes after this story — .engineering/planning/story/dashboard-overview.md:73
- story:dashboard-overview — "timeline of today" names no events, source or time rule, so a Vitest test passes on any list a fixture holds — .engineering/planning/story/dashboard-overview.md:76
- story:dashboard-overview — open pull and merge requests carrying `url` is "filled by `collect/github.rs` and `collect/gitlab.rs`", but the listed tests read rows over a fixture snapshot and none runs a collector, so the collectors can fill nothing and the story still passes — .engineering/planning/story/dashboard-overview.md:48
- story:dashboard-repo-view — "today's events for the repository: its dispatches, reports and decisions since 00:00" names no endpoint that serves them, and `/data.json` carries only each session's newest dispatch and the last few decisions, so the Rust test that holds the fixture keys to the served endpoints has nothing to hold them to — .engineering/planning/story/dashboard-repo-view.md:72
- story:dashboard-health — the class `old store` is tested with "a store-version refusal", which gives no first-line text, so the classifier's rule for it cannot be checked, unlike the other four classes — .engineering/planning/story/dashboard-health.md:58
- story:dashboard-health — the cache is "refreshed every 10 minutes off the request path, with the time it was taken", but the only test shows two requests giving one `aep` run, so the refresh, the timestamp and the off-request-path claim are unchecked — .engineering/planning/story/dashboard-health.md:43
- story:dashboard-health — `disk` is "free space against the floors, and every disk-waste row", but the only disk test counts the 85 waste rows, so the free-space-against-floors half has no check — .engineering/planning/story/dashboard-health.md:49

**Round 1 findings**

| Finding | Artifact | Status |
|---|---|---|
| r1#1 | story:conductor-upgrade | Fixed. One test per declared outcome now exists, including `--version`, and the doctor note is gone. One residue remains, the `--version` pre-check above. |
| r1#2 | story:snapshot-member-failures | Fixed. `conductor snapshot member-failures --snapshot-id` is declared. |
| r1#3 | story:snapshot-member-failures | Fixed. Both collectors are tested, with the member name, collector name and first error line asserted. |
| r1#4 | story:fetch-without-prompts | Fixed. The duplicate outcome is gone; (c) covers the timeout and (d) the last-fetched read. |
| r1#5 | story:fetch-without-prompts | Fixed. (a) covers all three collectors and (b) the caller's `GIT_SSH_COMMAND`. |
| r1#6 | story:dashboard-spa | Fixed. `task check` runs `npm ci`, Vitest and the `dist/` diff, and `check.yml` installs Node. |
| r1#7 | story:dashboard-spa | Fixed. The panels are listed, there is one Vitest test per panel, and the old-HTML assertions move to `/data.json`. The panel list matches the `<h2>` sections of `page()` at `dashboard.rs:957-1210`. |
| r1#8 | story:dashboard-overview | Mostly fixed. The mockup and the phone-width and theme claims are gone, the "needs you" and red-CI membership rules are stated, and the "last event" test is named. Three residues remain: the waiting filter, the timeline and the collector `url`. |
| r1#9 | story:dashboard-snapshot-api | Moot, the story is retired. Its cases (newer snapshot still collecting or failed, empty store, POST 405, `url`) moved into dashboard-overview's endpoint bullet. Only the collector-fills-`url` part has no test. |
| r1#10 | story:dashboard-repo-view | Fixed for the six waiting-on causes, which now have rules, fixtures that meet and just miss them, and decisions with `repository`. One residue remains, the endpoint for today's events. |
| r1#11 | story:dashboard-health | Partly holds. The assertions and the fix-text source (one table in `dashboard.rs` keyed by `FailureClass`) are fixed, and disk waste is checked at 85 rows. The caching and `disk` residues remain, along with the `old store` text. |

**What I read.** All 8 stories in full with `aep plan artifact show` in `conductor-wave-17`, plus `review-result:v8-acceptance-r1` and `epic:dashboard-for-engineers`. I also checked `release.yml`, `Cargo.toml`, `collect/mod.rs`, `blockers.rs`, `dashboard.rs`, `doctor.rs`, `observation.yaml` and `tests/` by `grep` and `sed`.

**What I could not establish.**
- `docs/design/dashboard-mockup.html` does not exist, and the acceptances no longer rely on it, so I did not count it.
- Several tests may be time-zone or user dependent. The `since 00:00 in the viewer's time zone` Vitest tests depend on the runner's TZ unless it is pinned. The `binary not writable` test passes nothing if CI runs as root. Neither is a finding, since the fix is a test-setup note.
- The `snapshot-duration` requirement that this instance completes in under 300 s can only be checked on that instance, through the recorded doc. I judged it checkable.
- Out of my lane:
  - The scope lists of `story:dashboard-spa` cite bare `dist` and `src` paths, which should sit under `crates/conductor/dashboard/`. That is for the scope or design critic.
  - Ordering between `dashboard-overview` and `dashboard-repo-view` on `/api/waiting-on` is design's concern, though it drives my second finding.

```findings
- file: .engineering/planning/story/conductor-upgrade.md
  line: 66
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says the command runs the unpacked binary's --version before replacing, but no listed test or declared outcome has an archive whose binary does not run, so that step passes whether or not it exists"
- file: .engineering/planning/story/dashboard-overview.md
  line: 73
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the repository table's waiting filter names no rule for which repositories are waiting, and the only data for it, /api/waiting-on, arrives with story:dashboard-repo-view, which comes after this story"
- file: .engineering/planning/story/dashboard-overview.md
  line: 76
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"timeline of today\" names no events, source or time rule, so a Vitest test passes on any list a fixture holds"
- file: .engineering/planning/story/dashboard-overview.md
  line: 48
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "open pull and merge requests carrying url is filled by collect/github.rs and collect/gitlab.rs, but the listed tests read rows over a fixture snapshot and none runs a collector, so the collectors can fill nothing and the story still passes"
- file: .engineering/planning/story/dashboard-repo-view.md
  line: 72
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "today's events (dispatches, reports and decisions since 00:00) name no endpoint that serves them, and /data.json carries only each session's newest dispatch and the last few decisions, so the Rust test holding the fixture keys to the served endpoints has nothing to hold them to"
- file: .engineering/planning/story/dashboard-health.md
  line: 58
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the class old store is tested with \"a store-version refusal\", which gives no first-line text, so the classifier's rule for it cannot be checked, unlike the other four classes"
- file: .engineering/planning/story/dashboard-health.md
  line: 43
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the doctor cache is refreshed every 10 minutes off the request path with the time it was taken, but the only test shows two requests giving one aep run, so the refresh, the timestamp and the off-request-path claim are unchecked"
- file: .engineering/planning/story/dashboard-health.md
  line: 49
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "disk is free space against the floors and every disk-waste row, but the only disk test counts the 85 waste rows, so the free-space-against-floors half has no check"
```
