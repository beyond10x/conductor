---
format: aep.planning-md/3
id: story:guard-git-c-outside
kind: story
status: implemented
title: The guard decides git -C and GIT_DIR writes on their target
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T15:22:36Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T15:22:37Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T15:40:45Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

An evaluation run on 2026-10-08 found that the guard allows `git -C <path>` where the path is
outside `checkouts.root` and the records: a controller can run git in any repository on the machine.
The README's security model says the guard bounds writes by path; `git -C` moves the write target
and the guard did not follow it.

## What

The guard reads a git command's target from `-C <path>`, `--git-dir`, `--work-tree` and
`GIT_DIR`/`GIT_WORK_TREE` assignments, and decides a writing git subcommand (commit, push, merge,
rebase, reset, checkout, switch, branch -d/-D, tag, stash, clean, worktree, fetch into a ref) on
that target as on a file write: allowed inside the session's writable roots, denied elsewhere with
the denial naming the path and the root it is outside. Reading subcommands (status, log, diff,
show, rev-parse, ls-files, config --get) stay allowed.

## Acceptance

- A controller session's `git -C /tmp/other commit -m x` is denied, naming `/tmp/other`.
- `git -C <its checkout> commit` is allowed; `git -C /tmp/other status` is allowed.
- `GIT_DIR=/tmp/other/.git git commit` and `git --work-tree=/tmp/other add .` are denied.
- The guard's latency check stays under its bound.
- The security page's guard section names the rule.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_rules.rs`,
`website/docs/security/the-guard.md`. Under `epic:session-confinement`.
