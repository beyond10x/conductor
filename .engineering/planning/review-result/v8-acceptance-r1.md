---
format: aep.planning-md/3
id: review-result:v8-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, v0.8.0 additions
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

- story:conductor-upgrade — the tests bullet covers check, one successful upgrade and a wrong sum, but not the declared outcomes "already current", "no archive for this target", "binary not writable" or `--version <x.y.z>`, and the doctor note has no test or cache location, so those outcomes can be marked done unchecked — `.engineering/planning/story/conductor-upgrade.md:57`
- story:snapshot-member-failures — "A view lists those failures for a snapshot. Test." names no view, command or endpoint, and the spec bullet declares no view, so there is nothing to run and look at — `.engineering/planning/story/snapshot-member-failures.md:24`
- story:snapshot-member-failures — the only test is a failing fetch in the blockers collector, so the specifications collector path and the recorded member name, collector and first error line are never asserted — `.engineering/planning/story/snapshot-member-failures.md:20`
- story:fetch-without-prompts — the second bullet repeats the outcome and test of `story:snapshot-member-failures` (three members, one failed fetch, a completed snapshot with a recorded failure), so both stories would be closed on one result; what is new is "times out" and "reads its checkout as last fetched", and no test asserts either — `.engineering/planning/story/fetch-without-prompts.md:37`
- story:fetch-without-prompts — "Every git command a collector runs" is checked only by a fake `git` recording the blockers fetch, so the other collectors' commands, and the user-set `GIT_SSH_COMMAND` that must be kept with the two options added, have no check — `.engineering/planning/story/fetch-without-prompts.md:33`
- story:dashboard-spa — the acceptance does not make `task check` run the Vue tests (Node and Vitest in `check.yml`, a `task ui` freshness diff), yet `story:dashboard-overview`, `story:dashboard-repo-view` and `story:dashboard-health` accept on Vitest tests; today `task check` has only fmt, clippy and `cargo test` and the workflow installs no Node, so those tests never gate anything — `.engineering/planning/story/dashboard-spa.md:34`
- story:dashboard-spa — "the existing panels render from `/data.json` in the app with the same content the old HTML had" cannot be checked by the Rust tests the sentence lists, which only read the index, an asset, the header and the `dist/` diff; it names no JS-side test and no list of panels or content, and the bullet joins build, embed, header, server limits and rendering into one sentence — `.engineering/planning/story/dashboard-spa.md:34`
- story:dashboard-overview — "the overview of the mockup" points at a canvas outside the repository, and "usable at phone width", dark and light themes and "at a glance" are not observable by Vitest component tests from fixture JSON, so the check is a vote; it names no fixture shape, no rule for the "needs you" or "red CI" membership, and no assertion for the "last event" correction — `.engineering/planning/story/dashboard-overview.md:25`
- story:dashboard-snapshot-api — the title promises the "newest complete snapshot", but the acceptance gives no case with a newer incomplete snapshot or an empty store, and it does not say that the tests assert the new `url` field on merge and pull request rows or the read-only refusal; "the snapshot age" is the only named assertion beyond "read each endpoint" — `.engineering/planning/story/dashboard-snapshot-api.md:32`
- story:dashboard-repo-view — the only test named is "for each waiting-on case"; wave progress, main CI, release, merge requests with links, plan, sessions with transcript link, today's events and the new repository field on decisions and escalations in `decide.rs` have none, and the six waiting-on causes have no rule saying what data makes one apply — `.engineering/planning/story/dashboard-repo-view.md:31`
- story:dashboard-health — "Tests over fixture data" does not say which assertions, "each with its fix text" names no source for that text, and "run by the server off the request path, cached" and "disk waste in full" have no observable check — `.engineering/planning/story/dashboard-health.md:25`

**What I read.** All 8 stories and `epic:dashboard-for-engineers`, with `aep plan artifact show` run in `conductor-wave-17`. I also read `Taskfile.yml` (`check`), `.github/workflows/check.yml`, `crates/conductor/tests/dashboard.rs`, and grepped for `GIT_TERMINAL_PROMPT`, `failure_reason` and `include_dir`.

**What I could not establish.**
- Whether implementors can run Node locally; the acceptances do not say, so I only judged what CI would check.
- The mockup canvas "Conductor dashboard redesign" was not available to me.
- Out of my lane, for design or parallel-safety:
  - `story:dashboard-overview` needs repository and pull request data that `story:dashboard-snapshot-api` only adds after it.
  - `story:dashboard-health` cites `story:snapshot-duration` without a `depends_on` relation.
  - The "Why" of `story:fetch-without-prompts` cites `repositories.rs:1203-1206`, but the `GIT_TERMINAL_PROMPT` line is now at `crates/conductor/src/collect/repositories.rs:1310`.

```findings
- file: .engineering/planning/story/conductor-upgrade.md
  line: 57
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the tests bullet covers check, one successful upgrade and a wrong sum, but not the declared outcomes already current, no archive for this target, binary not writable or --version <x.y.z>, and the doctor note has no test or cache location, so those outcomes can be marked done unchecked"
- file: .engineering/planning/story/snapshot-member-failures.md
  line: 24
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"A view lists those failures for a snapshot. Test.\" names no view, command or endpoint, and the spec bullet declares none, so there is nothing to run and look at"
- file: .engineering/planning/story/snapshot-member-failures.md
  line: 20
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the only test is a failing fetch in the blockers collector, so the specifications collector path and the recorded member name, collector and first error line are never asserted"
- file: .engineering/planning/story/fetch-without-prompts.md
  line: 37
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the second bullet repeats the outcome and test of story:snapshot-member-failures (three members, one failed fetch, a completed snapshot with a recorded failure), so both stories would be closed on one result; what is new is times-out and reads-its-checkout-as-last-fetched, and no test asserts either"
- file: .engineering/planning/story/fetch-without-prompts.md
  line: 33
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"Every git command a collector runs\" is checked only by a fake git recording the blockers fetch, so other collectors' commands and the user-set GIT_SSH_COMMAND that must be kept with the two options added have no check"
- file: .engineering/planning/story/dashboard-spa.md
  line: 34
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance does not make task check run the Vue tests (Node and Vitest in check.yml, a task ui freshness diff), yet dashboard-overview, dashboard-repo-view and dashboard-health accept on Vitest tests; today task check has only fmt, clippy and cargo test and the workflow installs no Node, so those tests never gate anything"
- file: .engineering/planning/story/dashboard-spa.md
  line: 34
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"the existing panels render from /data.json in the app with the same content the old HTML had\" cannot be checked by the Rust tests the sentence lists and names no JS-side test or list of panels, and the bullet joins build, embed, header, server limits and rendering into one sentence"
- file: .engineering/planning/story/dashboard-overview.md
  line: 25
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"the overview of the mockup\" points at a canvas outside the repository, and phone width, dark and light themes and at-a-glance are not observable by Vitest component tests from fixture JSON; it names no fixture shape, no rule for needs-you or red-CI membership and no assertion for the last-event correction"
- file: .engineering/planning/story/dashboard-snapshot-api.md
  line: 32
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the title promises the newest complete snapshot but the acceptance gives no case with a newer incomplete snapshot or an empty store, and does not say the tests assert the new url field on merge and pull request rows or the read-only refusal"
- file: .engineering/planning/story/dashboard-repo-view.md
  line: 31
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the only test named is for each waiting-on case; wave progress, main CI, release, merge requests with links, plan, sessions with transcript link, today's events and the new repository field on decisions and escalations in decide.rs have none, and the six waiting-on causes have no rule saying what data makes one apply"
- file: .engineering/planning/story/dashboard-health.md
  line: 25
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"Tests over fixture data\" says no assertions, each-with-its-fix-text names no source for that text, and run-off-the-request-path-and-cached and disk-waste-in-full have no observable check"
```
