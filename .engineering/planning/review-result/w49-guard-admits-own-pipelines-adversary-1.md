---
format: aep.planning-md/3
id: review-result:w49-guard-admits-own-pipelines-adversary-1
kind: review-result
status: active
title: Wave 49 U5 adversary, pass 1
relations:
- reviews: story:guard-admits-own-pipelines
revision: 1
---
unit: story:guard-admits-own-pipelines, working tree of w49-u5 on top of b4b6e61a8
verdict: CONFIRMED (8 findings, 9 red cases: a `glab ci retry|run` or `glab api` call without `-R` still reaches another repository, and one own-repository retry is wrongly denied)
cases: executed 1608→1618, red 9
origin: introduced 1 / pre-existing 0 / undecided 7
wrote-outside-worktree: 10 paths under the unit's scratch adv/ directory
needs-coordinator: a base-commit run to settle the 7 undecided origins; build_run::a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger fails with or without these cases

Diff: crates/conductor/tests/guard_rules.rs, 175 insertions, test cases `adversary_*` only.

Cases (run alone: 10 tests run: 1 passed, 9 failed):
0 the writes the cases rely on are admitted alone (green): git remote add/set-url, git config
  include.path, git config url.….insteadOf, git init --bare sub, ln -s.
1 `git remote add upstream <other> && glab ci retry 123` is denied (red)
2 `export CDPATH=$HOME/clones; cd other && glab ci retry 123` is denied (red)
3 `ln -s ~/clones/other l && cd l && glab ci retry 123` is denied (red)
4 `for i in 1 2; do glab ci retry 123; cd ~/clones/other; done` is denied (red)
5 `f() { glab ci retry 123; }; cd ~/clones/other && f` is denied (red)
6 a bare repository inside the checkout whose origin is the other repository (red)
7 an upstream via include.path, and origin rewritten by insteadOf (red)
8 `printf -v GITLAB_HOST %s …; export GITLAB_HOST; glab ci retry 1 -R example-group/widget` (red)
9 `GIT_PAGER=cat glab ci retry 123` in the own checkout is allowed (red: Deny)

Cases 6 and 7 were checked with glab 1.115 in scratch: glab filled `:fullpath` with the other
repository in each state.

Suite with the cases: 1618 tests run: 1608 passed, 10 failed (the 9 cases plus the build_run
case); without them: 1608 run, 1607 passed, 1 failed (build_run).

The 7 bypasses are undecided because they were not run against the base; base rules.rs line 1835
admits every pipeline write without -R wherever it runs, so they are almost certainly not
regressions, but they bypass the check this unit added and contradict its doc row and
website/docs/security/the-guard.md:143.

Tried and could not break: -R in every spelling; `--` before -R and -bR (glab rejects the extra
arguments); cd -, cd $var, popd; env -C and GIT_DIR=; nested non-bare repo, a .git file, a linked
tree; glab ci view admitted as a read. Rule 6 latency not run.

```findings
- file: crates/conductor/src/guard/rules.rs
  line: 1765
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "implicit_denial reads the remotes before the line runs, so `git remote add upstream <other> && glab ci retry 123` is admitted and retries another repository's pipeline"
- file: crates/conductor/src/guard/rules.rs
  line: 1743
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "follow joins `cd other` to the current directory and ignores an exported CDPATH, so the guard thinks glab runs in the checkout while it runs in another clone"
- file: crates/conductor/src/guard/rules.rs
  line: 2171
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "word_path cannot resolve a link the same line creates, so `ln -s ~/clones/other l && cd l && glab ci retry 123` is admitted"
- file: crates/conductor/src/guard/rules.rs
  line: 1528
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "the line is followed in text order, so a loop's later pass or a function called after a cd runs a bare glab ci retry in another repository and is admitted"
- file: crates/conductor/src/guard/rules.rs
  line: 1771
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "the nearest ancestor holding .git is not the repository git discovers when a bare repository sits inside the checkout, so glab there reads another project's remote and the call is admitted"
- file: crates/conductor/src/guard/rules.rs
  line: 3383
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "git_remotes ignores include/includeIf, insteadOf, config.worktree and the global config that git remote -v and glab honour, so the-guard.md's every remote of the checkout naming it does not hold"
- file: crates/conductor/src/guard/rules.rs
  line: 1851
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "line.assigned sees only NAME=value words, so GITLAB_HOST set by printf -v or read and then exported is not caught and a -R own-path write goes to another host"
- file: crates/conductor/src/guard/rules.rs
  line: 3376
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "is_git_variable matches every GIT_ name, so GIT_PAGER=cat or GIT_TERMINAL_PROMPT=0 denies the own checkout's bare retry with the false reason that the line moves git's location"
```
