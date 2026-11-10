---
format: aep.planning-md/3
id: review-result:w49-u13-adversary-1
kind: review-result
status: active
title: Wave 49 U13 dispatch-refuses-unknown-repository, adversary pass 1
relations:
- reviews: story:dispatch-refuses-unknown-repository
revision: 3
---
unit: U13 story:dispatch-refuses-unknown-repository, commit 3a9e0c994 on base 3a27c6469, worktree ~/.local/state/worktree/trees/b10x/conductor/w49-u13
verdict: CONFIRMED (judgement only; no red case)
cases: executed 26→29, red 0
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 4 paths
needs-coordinator: whether the story's own trial case (a dispatch to `checks`) is meant to stay accepted

`26` is this run with my file's 3 cases subtracted (`dispatch`, `isolated_home`, `no_organization`); the full-package count the implementor reported is 1646.

## 1. git --no-pager diff --stat

Empty: nothing tracked changed. `git status --short`:

```
?? crates/conductor/tests/adv_dispatch_unknown_repository_w49.rs
```

One new test file, uncommitted. No implementation file touched.

## 2. Cases added

`crates/conductor/tests/adv_dispatch_unknown_repository_w49.rs`, all three green now and green on their first run (alone, before the suite). None is a finding; they pin lines the unit's one test does not.

- `only_the_newest_complete_snapshot_decides_which_repositories_are_known`: three snapshots stored out of start order. A name only an older complete snapshot holds, and a name only a newer `Failed` snapshot holds, are each refused, and the message lists exactly `alpha, beta`. Guards the `snapshot_id == newest` filter and the choice of snapshot; the unit's test has one snapshot, so dropping the filter would pass it. I did not build that mutant.
- `a_name_that_only_resembles_a_known_one_is_refused`: no config file. `Alpha`, `ALPHA`, `alpha/`, `example-org/alpha`, `./alpha`, `/alpha`, ` alpha`, `alpha `, `alph`, `alpha-`, `alpha-tool`, `alpha-tools-two`, `tools` are each refused with the exact full stderr line. None takes an id: the next sends get `-01` and `-02`.
- `a_refusal_leaves_the_records_as_they_were`: with a config file, a refusal changes no byte under the records directory. A workspace file with a member lacking `source` ends the command with exit 1 and `read the workspace: … <file>`, nothing sent, records unchanged, even for a name the snapshot holds.

First run, alone (`adv/own.log`):

```
    Starting 3 tests across 1 binary
        PASS [   0.452s] (1/3) conductor-cli::adv_dispatch_unknown_repository_w49 a_refusal_leaves_the_records_as_they_were
        PASS [   0.700s] (2/3) conductor-cli::adv_dispatch_unknown_repository_w49 a_name_that_only_resembles_a_known_one_is_refused
        PASS [   0.730s] (3/3) conductor-cli::adv_dispatch_unknown_repository_w49 only_the_newest_complete_snapshot_decides_which_repositories_are_known
     Summary [   0.730s] 3 tests run: 3 passed, 0 skipped
exit 0
```

## 3. Suite run

Not the full package: the coordinator capped me at one more build request, so I ran the binaries my file can affect.

```
conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test dispatch --test adv_dispatch_unknown_repository_w49 --test isolated_home --test no_organization --no-fail-fast
    Starting 29 tests across 4 binaries
     Summary [   2.935s] 29 tests run: 29 passed, 0 skipped
exit 0
```

`isolated_home` and `no_organization` scan test files, so they cover the new file. Full log: `adv/suite.log`.

## 4. Judgement findings (cover commit 3a9e0c994)

| # | file:line | finding | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| 1 | `crates/conductor/src/dispatch.rs:553` | The story's own trial case is still accepted. Its Why is a dispatch to `checks`; `story:checkout-name-differs-from-project` says the workspace member there *is* `checks` while the snapshot names `example-checks`. `named` accepts any workspace member, so that dispatch is sent as before and starts nothing. The unit's test avoids this by giving the workspace the member `alpha`. The acceptance line ("neither … nor the workspace") is met as written. | the trial instance's reported state; any GitLab checkout whose directory differs from its project name | CONFIRMED | pre-existing |
| 2 | `crates/conductor/src/dispatch.rs:553` | The refusal is a new error that `spec/` does not model. AGENTS.md rule 1 says a new error is modelled in `spec/` first, and rule 8 says a refused command names the specification's error. `DispatchExists` and `StateDirNotInstance` are both declared there; this one has no name. Not measured by a test: nothing in the gate checks it. | every refusal of `send-dispatch --repository` | CONFIRMED | introduced |
| 3 | `crates/conductor/src/dispatch.rs:570` | A workspace file that does not read now fails every `send-dispatch`, including one to a repository the snapshot names. Before the unit the file was not read. The `# Errors` doc of `send_dispatch` does not list it. Measured by my third case. | a hand-edited or damaged `workspace.yaml` only; `aep` refuses the same member shape, and `conductor config workspace` never writes it | INFEASIBLE | introduced |

Finding 1 is the one that matters for whether the story does what its Why asks; it is the coordinator's call whether that belongs to this story or to the checkout-name one.

## 5. Attacked and not broken

- Name comparison is exact: case, trailing slash, owner prefix, path, spaces, prefix and suffix forms are all refused.
- An older complete snapshot and a newer failed one do not widen the known set.
- The accept-everything branch: an initialised instance does not reach it, because `conductor config workspace` always writes the records directory as a member. It is reached only with no complete snapshot and no workspace file (or no config file and no complete snapshot).
- A side effect of the same fact, not tested: the records directory's name is itself a known "repository" and appears in the refusal's list.
- No bypass through the daemon: the CLI opens the store itself; `daemon.store` is only the URL.
- `dispatch::send` (the loop, `run.rs:888`) is unchanged and unchecked, as the commit says.
- Refusal: exit 1, one stderr line, empty stdout, no dispatch, no id consumed, no records byte changed.
- An empty workspace file reads as no members (read from the code, not run).
- The documented walkthroughs (`docs/demo/walkthrough.md`, `website/docs/guides/first-task-existing-repo.md`) send to a repository their earlier steps put in the workspace or a snapshot (read, not run).

## 6. Paths written outside the worktree

- `~/.cache/scratch/<session>/w49/u13/adv/build.sh`
- `~/.cache/scratch/<session>/w49/u13/adv/own.log`
- `~/.cache/scratch/<session>/w49/u13/adv/suite.log`
- `~/.cache/scratch/<session>/w49/u13/adv/ws/` (an `aep workspace members` probe: `.engineering/workspace.yaml`, `other/`)

Plus this report, `~/.cache/scratch/<session>/w49/u13/adv-report-1.md`, as instructed. The test runs wrote case directories under the shared target's `tmp/adv_dispatch_unknown_repository_w49/`.

## 7. Findings

```findings
- file: crates/conductor/src/dispatch.rs
  line: 553
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a dispatch to a workspace member the snapshot names differently (the story's own `checks` case) is still accepted and starts nothing
- file: crates/conductor/src/dispatch.rs
  line: 553
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the new refusal is an error spec/ does not model, against AGENTS.md rules 1 and 8
- file: crates/conductor/src/dispatch.rs
  line: 570
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a workspace file that does not read fails every send-dispatch, also for a repository the snapshot names; only a hand-damaged file reaches it
```
