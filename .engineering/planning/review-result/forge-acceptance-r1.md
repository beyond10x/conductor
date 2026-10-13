---
format: aep.planning-md/3
id: review-result:forge-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, epic forge-neutral
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

- story:gh-optional — the second acceptance bullet reads the same before and after the work, so its test passes on today's tree: "A snapshot of an instance with no `github` source runs no `gh`". The collectors already list only `origins.owners` (`crates/conductor/src/collect/repositories.rs:384`), a `gitlab` source adds none (`collect/mod.rs:153`), and `programs()` already omits `gh` without an owner (`collect/mod.rs:109`). The bullet must name what changes, for example that the snapshot's start check requires `glab` for a `gitlab` source and no longer `gh`. — .engineering/planning/story/gh-optional.md:36
- story:gh-optional — the first bullet claims `glab` is required only for a `gitlab` source, but its test holds only `gh: not needed` on a gitlab-only config, so the `glab` clause has no check. Add a case where a `gitlab` source with `glab` off `PATH` exits 1 and a `github`-only config reports `glab: not needed`. — .engineering/planning/story/gh-optional.md:32
- story:gitlab-guide — it names a check that does not exist: "`task check`'s documentation build passes with the page in the sidebar". `task docs-check` only runs `conductor-docs generate --check` (`Taskfile.yml:327`), which neither builds Docusaurus nor reads `sidebars.ts`. The build that does is `.github/workflows/pages.yml:34`, which is not part of `task check`. Name that CI job. Nothing checks the README clause (`gh` for a GitHub source, `glab` for a GitLab one) or the guide's content either; add a test or a grep step for each. — .engineering/planning/story/gitlab-guide.md:28
- story:gitlab-merge-requests — one sentence joins three independent outcomes: open merge requests as pull request rows, merged merge requests as merged pull request rows, and the default-branch pipeline as a workflow run. Any one can pass while the others fail, so the sentence gives no single done or not-done answer. Split it, or state a separate assertion for each. — .engineering/planning/story/gitlab-merge-requests.md:29
- story:watch-gitlab-ci — the acceptance promises "the same `main` red and green lines", but the test reads only the red line from a pipeline going from success to failed. The green line has no check. Add a failed-to-success case, or drop green from the acceptance. — .engineering/planning/story/watch-gitlab-ci.md:25
- story:gitlab-workspace — the acceptance adds `exclude` entries, host matching and "reading no network", but the test has two fixture checkouts, one on the group and one "elsewhere". That cannot tell a wrong host from a wrong group, never uses `exclude`, and does not check that no network is read. Add a fixture on another host with the same group path, a fixture matching an `exclude` entry, and an assertion that nothing network-bound ran. — .engineering/planning/story/gitlab-workspace.md:27
- story:guard-glab-writes — "one read and one write per group" names no groups, and the group is any first operand (`github_write`, `rules.rs:1781`). So the case list cannot be counted, and "the same decisions as the matching `gh` cases" points at cases it does not name. The read list `<group> view|list|status|diff|ci view|ci list|ci status` also repeats `ci` inside a `<group>` pattern, so it is ambiguous. Enumerate the groups (for example `mr`, `issue`, `ci`, `release`, `api`, `auth`) and the exact read verbs per group. — .engineering/planning/story/guard-glab-writes.md:27
- story:profiles-forge-neutral — "the forge-neutral step" is undefined in the story. The test only pairs `glab` with each `gh`, and "beside" has no bound (same line or same paragraph). The clauses "a pull request is a merge request on GitLab" and "CI is the repository's workflow or pipeline" have no check. Quote the sentences the profiles must contain, as the existing `profiles.rs` cases do with `text.contains(...)`. — .engineering/planning/story/profiles-forge-neutral.md:26
- story:doctor-verdict — two outcomes share one sentence, a verdict line and `pins: none`. The test "reads the last line of both cases", and the pins line is no longer the last line, so `pins: none` has no check. Add an assertion on the pins line. — .engineering/planning/story/doctor-verdict.md:26
- story:doctor-verdict — "otherwise `not ready:` followed by each missing or too-old program" leaves a case undefined. A pinned program whose version cannot be read also exits 1 (`doctor.rs`, `PrerequisiteState::VersionUnread`), so the verdict line would carry no program. State what it lists. — .engineering/planning/story/doctor-verdict.md:26

**What I read:** 13 of 13 artifacts (the ids you named), with `aep plan artifact show` on each, plus `aep plan artifact kinds` and `lifecycle story`. I checked the tree with `git grep`-style reads of `Taskfile.yml`, `.github/workflows/{check,pages}.yml`, `b10x.toml`, `collect/{mod,repositories,github}.rs`, `doctor.rs`, `guard/rules.rs`, `tests/{cli_tree,profiles,instance_sources,doctor}.rs` and `spec/domains`.

I found no acceptance defect in story:gitlab-source-host, story:gitlab-repositories, story:doctor-minimum-versions, story:version-flag or story:gc-archive-profile.

**What I could not establish:**
- Whether the gitlab-only test in gh-optional's second bullet fails on the current tree. I cannot run cargo, so that finding rests on reading the collectors and `instance_sources.rs:431`.
- How `conductor watch run` picks the projects it reads for a GitLab source. The acceptance says "a project's default-branch pipeline" without saying which projects.
- Out of my lane, for design and parallel-safety:
  - `story:gitlab-source-host` changes `spec/domains/config.yaml`, but `task check` also diffs `crates/conductor-model` against what `ess generate synthesize` produces (`Taskfile.yml:312`), and that crate is in no story's scope.
  - `story:gitlab-source-host` has the spec say the binary reads `glab api --hostname`, one story before any collector does.
  - `README.md` and `website/docs/getting-started.md` are in the scope of both `story:gh-optional` and `story:gitlab-guide`.

```findings
- file: .engineering/planning/story/gh-optional.md
  line: 36
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the second acceptance bullet 'A snapshot of an instance with no `github` source runs no `gh`' reads the same before and after the work: collectors already list only origins.owners (collect/repositories.rs:384) and programs() already omits gh without an owner (collect/mod.rs:109), so its test passes on the unmodified tree; it names no change"
- file: .engineering/planning/story/gh-optional.md
  line: 32
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the first bullet claims glab is required only for a gitlab source, but its test holds only 'gh: not needed' on a gitlab-only config, so the glab-required and glab-not-needed clauses have no check"
- file: .engineering/planning/story/gitlab-guide.md
  line: 28
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names 'task check's documentation build passes with the page in the sidebar', but task check's docs-check (Taskfile.yml:327) neither builds Docusaurus nor reads sidebars.ts (the build is .github/workflows/pages.yml:34, outside task check), and the guide content and the README requirements clause have no test or CI step"
- file: .engineering/planning/story/gitlab-merge-requests.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "one sentence joins three independent outcomes (open merge request rows, merged pull request rows, default-branch pipeline as a workflow run), so one can pass while the others fail"
- file: .engineering/planning/story/watch-gitlab-ci.md
  line: 25
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance promises the same 'main red and green lines' but its test only reads the red line of a success-to-failed pipeline, so the green half has no check"
- file: .engineering/planning/story/gitlab-workspace.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance adds host matching, exclude entries and 'reading no network', but its test has only one fixture on the group and one 'elsewhere', so a wrong host cannot be told from a wrong group and exclude and no-network have no check"
- file: .engineering/planning/story/guard-glab-writes.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'one read and one write per group' names no groups, 'the matching gh cases' are unnamed, and the read list repeats 'ci' inside a <group> pattern, so the case set cannot be counted or checked"
- file: .engineering/planning/story/profiles-forge-neutral.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'the forge-neutral step' is undefined, 'beside' has no bound, and the test only pairs glab with each gh, so the merge-request and workflow-or-pipeline wording is held by no check"
- file: .engineering/planning/story/doctor-verdict.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "two outcomes share one sentence (the verdict line and 'pins: none') but the test reads only the last line, so the pins line has no check"
- file: .engineering/planning/story/doctor-verdict.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'not ready:' is to be followed by 'each missing or too-old program', but a pinned program with an unread version also exits 1 (doctor.rs PrerequisiteState::VersionUnread), so the acceptance leaves that case's output undefined"
```
