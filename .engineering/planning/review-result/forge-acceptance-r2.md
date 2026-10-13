---
format: aep.planning-md/3
id: review-result:forge-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, epic forge-neutral
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

- story:gitlab-guide — the first bullet's named check does not hold its claims: the Pages build (`.github/workflows/pages.yml:34`, which also runs on `pull_request`) passes whether or not `guides/gitlab` is listed in the explicit `docsSidebar` (`website/sidebars.ts:7`). It also reads none of the page's content (the `host: gitlab.example.com` config, `glab auth login --hostname`, "the list of what conductor reads from GitLab", whose entries are not named). `docs_forge.rs` is already created for the other bullets, so name its assertions for the sidebar entry and the three content items — `.engineering/planning/story/gitlab-guide.md:49`
- story:gitlab-guide — the acceptance says "No hand-written page says the binary does not read GitLab", but the test finds only the literal "not read by the binary". `website/docs/concepts/instances-and-config.md:130` says "the binary does not read it", which that literal misses, so the test passes while the acceptance fails. State the phrasings the test rejects, or make the acceptance the literal — `.engineering/planning/story/gitlab-guide.md:53`
- story:gitlab-guide — "both requirement sentences in the README" quotes no sentence. The README's requirement today is a table row (`README.md:56`, "| `gh`, logged in | | pull requests, CI runs, releases, repository lists of a GitHub source |"), not a sentence. Quote the text or the row the test must find for `gh` and for `glab` — `.engineering/planning/story/gitlab-guide.md:53`
- story:guard-glab-writes — the acceptance lists twenty-odd read forms and says every other call is a held write, but its test decides nine pairs. No pair covers `mr list|diff`, `issue view`, `release view|list|download`, `repo view|list`, `ci list|status|trace|get`, `--help` or `--version`. The only write forms covered are `mr merge`, `mr note`, `release create` and `api -X POST`. Name that each listed read form is decided in the test, or cut the list to the forms the pairs cover — `.engineering/planning/story/guard-glab-writes.md:27`
- story:gitlab-repositories — the first bullet names open issues, newest release tag and date, commits after it and `local_checkout`, but its test "reads two rows, one archived". Nothing says the test asserts those fields, so a row with `open_issues: 0` and `local_checkout: false` for every project passes. Name the fixture values the test holds for each field — `.engineering/planning/story/gitlab-repositories.md:43`
- story:gitlab-repositories — the second bullet's test "reads the row of a fixture project whose checkout holds a `spec/` directory", but the specification row has `presence`, `path`, `format`, `validation` and more (`crates/conductor-model/src/observation.rs:2399`). The bullet names none the test asserts, so a `Missing` row also "reads". Name the fields and values (for example `presence` `Present`, `path` `spec`) — `.engineering/planning/story/gitlab-repositories.md:48`
- story:doctor-minimum-versions — the acceptance claims `aep`, `ess` and `worktree` are compared against built-in minimums, but the test uses only a fake `aep` printing `aep 0.63.1`. A `build.rs` that embeds only the `aep` pin passes it. Name an `ess` and a `worktree` case (`b10x.toml` pins `ess = "0.59"` and `worktree = "0.18"`) — `.engineering/planning/story/doctor-minimum-versions.md:29`
- story:doctor-verdict — the acceptance enumerates three failure forms, `(missing)`, `(too old)` and `(version unread)`, but the test covers a run with one missing and one unread program. `(too old)` has no case, though `PrerequisiteState::OlderThanPin` already exists (`crates/conductor/src/doctor.rs:273`) and a pin file can reach it — `.engineering/planning/story/doctor-verdict.md:26`

Round 1 status of the ten findings:
- Forge-acceptance-r1#1, gh-optional second bullet: fixed. Line 42 now names `glab` in `collect::programs` and a refusal before the snapshot starts, which the current `programs()` (`collect/mod.rs:106`) does not do.
- Forge-acceptance-r1#2, gh-optional glab clause: fixed. Lines 38-41 hold three cases, including `glab: not needed` and a gitlab source without `glab` exiting 1.
- Forge-acceptance-r1#3, gitlab-guide: partly fixed.
  - Fixed: the Pages job is now named (line 50) and a `docs_forge.rs` test is added.
  - Still holds: the guide's content and sidebar listing have no check. The README clause is tested, but the sentences are unnamed (first and third findings above).
- Forge-acceptance-r1#4, gitlab-merge-requests: fixed. Four bullets, each with its own test (lines 29-38).
- Forge-acceptance-r1#5, watch-gitlab-ci: fixed. The green transition is now in the acceptance and the test (lines 26-30).
- Forge-acceptance-r1#6, gitlab-workspace: fixed. Four fixtures and a `glab` and `gh` that fail when called (lines 31-36).
- Forge-acceptance-r1#7, guard-glab-writes: partly fixed.
  - Fixed: the groups and pairs are enumerated.
  - Still holds, in a new form: the listed reads outnumber the paired cases (fourth finding above).
- Forge-acceptance-r1#8, profiles-forge-neutral: fixed. The sentence is quoted and "same line" bounds "beside" (line 27).
- Forge-acceptance-r1#9, doctor-verdict pins line: fixed. Line 30 is its own bullet, and "the same test reads it".
- Forge-acceptance-r1#10, doctor-verdict unread program: fixed. `(version unread)` is now listed (line 28).

The fourth, fifth, sixth, seventh and eighth findings above are new in round 2. Round 1 cleared those bodies, and nothing in round 1's text applied to them.

No acceptance defect in story:gitlab-source-host, story:gitlab-merge-requests, story:gitlab-workspace, story:watch-gitlab-ci, story:gh-optional, story:profiles-forge-neutral, story:version-flag or story:gc-archive-profile. Each acceptance names a before and after state, one outcome and a test fixture.

**What I read:** 13 of 13 ids. I read the 13 story bodies with Read on `.engineering/planning/story/*.md`, because the `aep` shell call was refused for a store outside the session worktree. I also read `review-result:forge-acceptance-r1` with `aep plan artifact show` and the rubric files. I checked the tree for `pages.yml`, `sidebars.ts`, `Taskfile.yml`, `b10x.toml`, `guard/rules.rs`, `doctor.rs`, `collect/mod.rs`, the model rows and the profiles. I did not run `aep plan artifact kinds` or `lifecycle story`.

**What I could not establish:**
- Whether `crates/conductor/tests/gc.rs`'s fixture `worktree` can record `--repo` for both calls. I read only `waste.rs:153` and the fixture's function list.
- Whether `config show` already prints sources.
- Out of my lane, for design and parallel-safety (all four carried from round 1):
  - `story:gitlab-source-host` regenerates `crates/conductor-model`, which `Taskfile.yml:312` diffs.
  - `story:gitlab-source-host` has the spec name `glab api --hostname` a story before any collector reads it.
  - `README.md` and `website/docs/getting-started.md` appear in the scope of both `story:gh-optional` and `story:gitlab-guide`; the guide's Notes say the guide is the only story on the hand-written pages.
  - `story:doctor-minimum-versions` needs a repo-root `b10x.toml` at build time, which `build.rs` may not find when built from a packaged crate.

```findings
- file: .engineering/planning/story/gitlab-guide.md
  line: 49
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the first bullet's named check, the Pages build, passes whether or not guides/gitlab is listed in the explicit docsSidebar (website/sidebars.ts:7) and reads none of the page's content (the host config, glab auth login --hostname, the unnamed list of what conductor reads), so the sidebar entry and the content have no check; name docs_forge.rs assertions for them"
- file: .engineering/planning/story/gitlab-guide.md
  line: 53
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says no hand-written page says the binary does not read GitLab but the test finds only the literal 'not read by the binary', which misses website/docs/concepts/instances-and-config.md:130 ('the binary does not read it'), so the test passes while the acceptance fails"
- file: .engineering/planning/story/gitlab-guide.md
  line: 53
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'both requirement sentences in the README' quotes no sentence and the README's requirement today is a table row (README.md:56), so the text the test must find for gh and for glab is undefined"
- file: .engineering/planning/story/guard-glab-writes.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance lists twenty-odd read forms and says every other call is a held write, but its nine pairs cover neither mr list|diff, issue view, release view|list|download, repo view|list, ci list|status|trace|get, --help or --version, nor writes other than mr merge, mr note, release create and api -X POST, so most of the list has no case"
- file: .engineering/planning/story/gitlab-repositories.md
  line: 43
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the first bullet names open issues, release tag and date, commits after it and local_checkout, but the test only 'reads two rows, one archived', so nothing says it asserts those fields and a row with zeroed values passes"
- file: .engineering/planning/story/gitlab-repositories.md
  line: 48
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the second bullet's test 'reads the row' of a fixture project but names no field it asserts (presence, path, validation of crates/conductor-model RecordSpecification), so a Missing row also passes"
- file: .engineering/planning/story/doctor-minimum-versions.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance claims aep, ess and worktree are compared against built-in minimums but the test uses only a fake aep printing aep 0.63.1, so a build.rs embedding only the aep pin passes; name an ess and a worktree case"
- file: .engineering/planning/story/doctor-verdict.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance enumerates (missing), (too old) and (version unread) but the test covers a run with one missing and one unread program, so the (too old) form has no case though PrerequisiteState::OlderThanPin exists (doctor.rs:273)"
```
