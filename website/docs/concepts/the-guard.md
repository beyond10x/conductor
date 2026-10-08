---
title: The guard and its limits
sidebar_position: 4
description: The PreToolUse hook that keeps a controller in its repository and conductor in its records, what it checks, and what it does not.
lede: The guard catches a session's mistakes on five tools; it is not a sandbox, and a Bash command can write anywhere its user can.
source: docs/design/conductor.md section 7; README.md, Security model; crates/conductor/src/guard.rs, crates/conductor/src/guard/; .claude/conductor-settings.json, .claude/controller-settings.json; crates/conductor/tests/guard_rules.rs, crates/conductor/tests/guard_hook.rs
---

Every session runs Claude Code with `--permission-mode bypassPermissions`: no tool call asks you
first. Conductor and controllers work unattended because of it. **The guard is the only check.**
Read this page before you start sessions on repositories you care about.

## What it enforces

| rule | enforced by |
|---|---|
| a controller's working directory is its checkout | the session is started there |
| a controller's Edit, Write, NotebookEdit only inside its repository's checkout and managed worktrees, or in its scratch | the guard |
| a controller's SendMessage only to `conductor` or to a sub-agent it started | the guard |
| no `gh` writes; writes to the forge go through the repository's own route | the guard |
| no `cd`, `pushd` or `git -C` into another repository or into conductor's records | the guard (a heuristic) |
| no write to conductor's records or conductor's config | the guard: file tools; Bash only for the config |
| no session writes a `.claude/` settings file (`settings*.json`, `controller-settings.json`, `conductor-settings.json`), a role's `settings` file or a role's `profile` file, by a file tool or a Bash write form that names it | the guard |
| no `conductor` command that writes, only its views | the guard |
| conductor's Edit, Write, NotebookEdit only to its records, or in its scratch | the guard |
| a build over `thresholds.build_size` while free disk is under `thresholds.build_slot` asks for a slot | the profile and `[RESOURCE]` |

Scratch is a session's own temporary space: under `$TMPDIR` for every session, and for a controller
also under `~/.cache/<repo>-*/`, with `<repo>` its repository and `/` written `-`. Scratch is never a
repository's checkout or managed worktree, conductor's records or its config, wherever `$TMPDIR`
points, and a `$TMPDIR` that is the home directory or holds it is no scratch at all.

## How it runs

The guard is a Claude Code PreToolUse hook. The repository ships it wired in
`.claude/conductor-settings.json` (conductor) and `.claude/controller-settings.json` (each
controller, named by the controller role's `settings`), once per matcher: `Edit`, `Write`,
`NotebookEdit`, `SendMessage`, `Bash`.

```console
conductor guard record-guard-decision --from-pre-tool-use || exit 2
```

It reads the hook payload on standard input and decides by the payload's working directory:

- in a repository's checkout or managed worktrees: the controller rules, unless a source of the
  instance excludes that repository;
- in conductor's records (the instance's `records` when a config file names it): conductor's
  rules, which allow edits to the records (`decisions/`, `dispatches/`, `charters/`,
  `docs/handoff/`, `goals.jsonl`, `STATUS.md`, `NORTHSTAR.md`) and to scratch, and deny `gh`
  writes;
- in no repository: every tool that has a rule is denied.

Exit 0 allows the call. Exit 2 blocks it, with one line of reason on standard error. Anything that
fails before a verdict exists is a denial, and the `|| exit 2` covers a missing binary.

Only denials are recorded: in the store under `--state-dir`, else under the instance's `state` when
a config file names it, else nowhere, because the hook runs in any session's working directory and
must not plant a store there. `conductor guard guard-decisions` reads them back. An allowed call is
answered without opening the store, because every tool call of every guarded session runs the hook.

## What it does not check

- The guard checks five tools: Edit, Write, NotebookEdit, SendMessage and Bash. Any other tool, an
  MCP server's tools included, is never shown to it.
- For Edit, Write and NotebookEdit the guard resolves the target path, links included, and denies
  a write outside the session's own repository, records or scratch directory.
- The guard does not check where a Bash command writes. A controller can write, overwrite or
  delete any file its user can: other checkouts, conductor's records and the `conductor` binary
  included. For Bash the guard denies only a `cd`, `pushd` or `git -C` into another checkout or
  the records, a `gh` write, a `conductor` write, and a write that names a settings file, a
  role's profile file or conductor's config, and a command written to avoid those forms is
  allowed. A controller that reads an issue or pull request written by someone else can be told
  to run such a command.
- `git push` is not checked; the forge's branch protection is the place for that.
- The conductor-dev session starts without the guard.
- `task trust` marks every checkout under `checkouts.root` as a trusted Claude Code workspace, so
  the project settings and hooks of each of those repositories run without a prompt.

Each snapshot reports dirty files in every checkout, which turns an unexpected Bash write into a
finding. The guard catches mistakes; it is no defence against a hostile prompt.

Run conductor under a user account, and with forge credentials, whose reach you accept for an
unattended agent.

:::caution[Planned]

An operating-system sandbox for Bash, which bounds where a command writes rather than parsing its
command line, is planned. So is routing every tool call to the guard and denying a tool it does
not know.

:::
