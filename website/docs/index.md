---
title: Conductor
slug: /
sidebar_position: 1
description: Conductor coordinates coding-agent sessions across the repositories of one organization, and what it leaves to its neighbours.
---

Conductor coordinates coding-agent sessions across the repositories of one organization. It has two
parts:

- **A session.** One background coding-agent session, named `conductor`, runs in the directory that
  holds the instance's records. It tracks what happens in every repository, starts one controller
  session per repository, and takes the routine decisions those controllers raise. Architectural
  decisions go to a person, the operator.
- **A CLI.** The `conductor` binary is the session's tool. It reads the state of the repositories
  into snapshots, keeps the records of what the session decides and dispatches, and guards the
  conductor and controller sessions with a hook.

The sessions run on Claude Code today. Each session follows a generic profile shipped in the
repository, then the instance's own `rules.md`; one config file names the instances.

## What it does

| | |
|---|---|
| **Sense** | `conductor snapshot start-snapshot` reads every repository of the instance: git state, pull requests, `main` CI, releases, open planning blockers, specification status and live sessions. |
| **Decide** | The session answers the decision requests controllers send. Routine ones (class C) it decides and records; architectural ones (class O) and ones that need the operator's hands (class H) go to the operator. |
| **Dispatch** | The session sends each controller a brief. A controller works only in its own repository and talks only to conductor. |
| **Guard** | A PreToolUse hook denies a controller's file edits outside its repository and its messages to anyone but conductor. |
| **Watch** | Between cycles, `conductor watch run --follow` wakes conductor when a session exits or fills its context, when `main` CI changes colour, or when disk runs low. |

## What it is not

- **Not a planner for a repository.** The work lives in each repository's planning store, and each
  controller plans its own repository. Conductor holds the cross-repository plan only.
- **Not a copy of the forge.** Code, issues, pull requests, CI and releases stay with the
  repositories and their forge. Conductor's records hold decisions, dispatches, goals, controllers,
  messages, guard verdicts and snapshots, nothing else.
- **Not an editor of other repositories.** Conductor never edits another repository; a controller
  does that, in its own checkout.
- **Not a sandbox.** The guard catches mistakes. It does not bound where a Bash command writes,
  and it is no defence against a hostile prompt ([the guard and its limits](./concepts/the-guard.md)).
- **Not unattended-safe by default.** Every session runs Claude Code with
  `--permission-mode bypassPermissions`. Read [the guard](./concepts/the-guard.md) before you start
  sessions on repositories you care about.

## Neighbours

| component | relation |
|---|---|
| [ESS](https://beyond10x.github.io/ess/) ([GitHub](https://github.com/beyond10x/ess)) | Conductor's nouns and commands are an ESS specification; the CLI's model crate is generated from it. The specifications collector runs `ess` against each repository's specification. |
| [AEP](https://beyond10x.github.io/ecosystem/aep/) ([GitHub](https://github.com/beyond10x/aep)) | Each repository's planning store. Controllers plan and deliver through it; the blockers collector reads open decision blockers with `aep plan workspace list`. |
| [Eventlog](https://beyond10x.github.io/ecosystem/eventlog/) ([GitHub](https://github.com/beyond10x/eventlog)) | Conductor's store is an eventlog tree store: one stream per record, one immutable file per event. |
| [Worktree](https://beyond10x.github.io/ecosystem/worktree/) ([GitHub](https://github.com/beyond10x/worktree)) | Managed worktrees beside each checkout. The guard treats a repository's managed worktrees as its own; the dashboard reads `worktree` cleanup dry-runs. |
| Gates ([GitHub](https://github.com/beyond10x/gates); no documentation site yet) | Common security and privacy checks. Conductor's own gate scans every tracked file with `b10x-gates` when a policy is configured. |

## Where to go next

- [Getting started](./getting-started.md): build the CLI, write a config, take one snapshot.
- [Roles](./concepts/roles.md) and [the loop](./concepts/the-loop.md): how the sessions divide
  the work.
- [Configure an instance](./guides/configure-an-instance.md) and
  [start conductor](./guides/start-conductor.md): run it for real.
- [The CLI reference](/docs/reference/cli) and [what exists today](/docs/status).
