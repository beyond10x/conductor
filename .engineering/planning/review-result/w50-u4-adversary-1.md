---
format: aep.planning-md/3
id: review-result:w50-u4-adversary-1
kind: review-result
status: active
title: Wave 50 U4 checkout-name-differs-from-project, adversary pass 1
relations:
- reviews: story:checkout-name-differs-from-project
revision: 1
---
```
unit: U4 story:checkout-name-differs-from-project, commit d30500383 on base d5cb4f8fc, plus my uncommitted test file
verdict: NEEDS-CHANGE
cases: executed 1743→1749 (1743 from the implementor's gate; I ran six binaries only: 178→184), red 3
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 7 paths, all under the assigned scratch `adv/`
needs-coordinator: yes: F1 and F3 are a decision (one source wins or fail; an old `exclude` honoured or reported), not a bug with one fix
```

**1. diff --stat**: `git status --short` shows only `?? crates/conductor/tests/checkout_name_adversary.rs`. No implementation file was touched.

**2. Cases added** in `~/.local/state/worktree/trees/b10x/conductor/w50-u4/crates/conductor/tests/checkout_name_adversary.rs`. The first run was this binary alone, before any suite run: `6 tests run: 3 passed, 3 failed`. I then removed one unused helper and ran rustfmt, so the panic lines in that run's output (381, 396, 419) are now 368, 383, 407.

- **F1, red:** `two_sources_whose_groups_nest_record_their_shared_checkout_once` (assert at :368)
  `exit Some(1) … repositories: repository tool is listed by the gitlab group example-org on gitlab.example.com and by the gitlab group example-org/platform on gitlab.example.com; a snapshot records one repository of a name`
- **F2, red:** `a_clone_under_a_name_that_is_no_repository_name_does_not_stop_the_snapshot` (assert at :383)
  `exit Some(1) … repositories: the gitlab group example-org on gitlab.example.com: "checks copy" is not a repository name`
- **F3, red:** `an_exclude_entry_naming_a_checked_out_project_by_its_path_excludes_it_or_is_reported` (assert at :407)
  `the excluded project is recorded as [("checks", true)] and nothing names the entry: … exit Some(0), stdout "<snapshot id>", stderr ""`
  The `config validate` in this case exits 1 only on the fixture's missing `session_prefix`; it says nothing about the entry.
- **Green, kills the `seen` mutant** (`gitlab.rs:608`): `a_project_listed_twice_is_one_repository_under_its_checkout`.
- **Green, kills the case-blind mutant** (`gitlab.rs:627`): `an_origin_written_in_another_case_is_its_projects_checkout`.
- **Green, characterises strays:** `a_checkout_no_listed_project_owns_is_no_repository_and_a_symlink_is_a_checkout`. A checkout of an unlisted project and one of a project outside the group are no rows; a symlink to a checkout is a second repository.

**3. Suite, run after the cases existed**
`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast --test collect_gitlab --test run --test watch --test spawn --test config_workspace --test checkout_name_adversary`
`Summary [ 22.697s] 184 tests run: 181 passed, 3 failed, 0 skipped`, exit 100. The three failures are F1 to F3.
Clippy (`cargo clippy -p conductor-cli --all-targets -- -D warnings`) on the final file printed `Finished` with no error. Its exit file is stale from the earlier run, so the log is the evidence. The test binary was not re-run after the helper removal and rustfmt; clippy compiled it.

**4. Findings** (all cover d30500383)

| id | severity | where | verdict | origin | what reaches it |
|---|---|---|---|---|---|
| F1 | blocker | `crates/conductor/src/collect/repositories.rs:503`, fed by `gitlab.rs:643` | NEEDS-CHANGE | introduced | Two `gitlab` sources whose groups nest. Any checkout of an inner-group project gets the same name from both, so every snapshot fails. The base recorded it (as `platform/tool` and `tool`). |
| F2 | warning | `crates/conductor/src/collect/gitlab.rs:641` | NEEDS-CHANGE | introduced | One extra clone whose directory is no repository name (`checks copy`) fails the whole snapshot until it is renamed or excluded. On the base the directory name of a GitLab checkout was never read. A `local` source fails the same way, so this is consistent, but new for existing roots. |
| F3 | warning | `crates/conductor/src/collect/gitlab.rs:601` and `:629` | NEEDS-CHANGE | introduced | `excluded` is computed from the project path but only applied when the project has no checkout. An existing `exclude: [example-checks]` stops excluding checkout `checks`, with exit 0 and no message from `config validate` or the snapshot. |
| F4 | note | `crates/conductor/src/watch.rs:1938` and `:2019` | CONFIRMED (read, not executed) | introduced | Until the first snapshot after the upgrade completes, the watch's names come from the old snapshot (`example-checks`) and its projects are keyed `checks`. In an instance that also has a `github` source the miss falls to `gh … -R <organization>/example-checks`. With F1 or F2 standing, no snapshot completes and it stays. |
| F5 | note | `~/.local/state/worktree/trees/b10x/conductor/w50-u4/website/docs/guides/gitlab.md` | CONFIRMED | introduced | The table rows for merge requests, pipelines and releases still say "each project". With two checkouts of one project each is recorded twice, once per directory name. |

The fix for F3 is either to honour a project-path entry or to name an entry that matches no repository. For F1, either one source wins, or `config validate` refuses nested groups.

**5. Attacked and not broken**
- Forge calls: every GitLab API path uses `project.id` (`gitlab.rs` `observe` and `read`, `watch.rs` `newest_pipeline` and `open_merge_requests`). The guard's own-project rule and `repository refresh-hooks` read the checkout's `origin`. None sends the directory name.
- Old records: before the change no controller could start under a project name without a directory of that name, so none is orphaned. A `charters/example-checks.md` was unused before and stays unused. This is from reading, not a case.
- Name-taken failure: the message names both projects, and renaming the directory clears it. `exclude: [alpha]` drops both the checkout and the bare project (read, no case).
- `github` source: no case written. Its names come from the forge listing, and the GitLab arm reaches it only through the duplicate-name check at `repositories.rs:503`. A bare GitHub `alpha` beside a GitLab checkout `alpha` would fail like F1; not executed.
- Spec comments, design document, guide and generated reference agree with the code except F5.
- Other new branches in `Forge::projects`: the unit's own cases hold the checkout-level exclude, the `found`-versus-`checkouts` split and the duplicate bail.

**6. Paths written outside the worktree**, all in `~/.cache/scratch/041da894-c3d7-4881-90c9-e7f85e9b66c9/w50/u4/adv/`: `build.zsh`, `suite.zsh`, `part2.rs`, `red.log`, `red.exit`, `suite.log`, `suite.exit`, `suite.done`, `clippy.log`, `clippy.exit`. Failed cases keep their directories under the shared target's `tmp/checkout_name_adversary/`.

```findings
- file: crates/conductor/src/collect/repositories.rs
  line: 503
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"two gitlab sources whose groups nest name one checkout alike, so every snapshot fails where the base recorded one"'
- file: crates/conductor/src/collect/gitlab.rs
  line: 641
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"a clone of a group project under a directory that is no repository name fails the whole snapshot, where the base never read the directory name"'
- file: crates/conductor/src/collect/gitlab.rs
  line: 601
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"an exclude entry written as the project path no longer excludes its checkout and nothing reports the entry"'
- file: crates/conductor/src/watch.rs
  line: 1938
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"until a new snapshot completes the watch misses the old project name in its project map and, with a github source, asks gh for it; read from code, not executed"'
- file: website/docs/guides/gitlab.md
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: '"the guide''s merge request, pipeline and release rows say each project while two checkouts of one project are recorded twice"'
```
