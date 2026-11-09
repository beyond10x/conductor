---
title: Conductor
slug: /
sidebar_position: 1
description: Conductor coordinates one coding-agent session per repository and records every decision it takes or relays.
---

Conductor coordinates coding-agent sessions across the repositories of one organization. Each
repository gets one controller session, which works only in its own checkout. Every decision
conductor takes or relays is a record with a class, a reason and its evidence. Decisions that
cannot be undone, and credentials, go to a person, the operator.

| part | command | does |
|---|---|---|
| conductord | `conductor daemon serve` | reads one config file, starts each instance's runtime, serves the dashboard on `127.0.0.1:<daemon.port>`, 7300 by default |
| instance runtime | `conductor instance serve` | serves one instance's API on `api.sock` in its state directory |
| the loop | `conductor run` | one process per instance, with no model call: the watch, the cycle and the daily brief |

In this release the runtime serves the API only and does not start `conductor run`.

## How it works

1. **Sense.** `conductor snapshot start-snapshot` reads git state, pull requests, `main` CI,
   releases, planning blockers, specification status and live sessions into one snapshot.
2. **Decide.** Conductor answers the decision requests controllers send. It decides and records
   class C. Class O and class H go to the operator
   ([decisions and messages](./concepts/decisions-and-messages.md)).
3. **Dispatch.** Each cycle, `conductor run` starts a controller for every Active repository with a
   charter in `<records>/charters/`, up to `controllers.max_working`.
4. **Guard.** Every tool call of a controller passes the PreToolUse hook
   `conductor guard record-guard-decision`, which allows or denies it and records each denial
   ([the guard](./security/the-guard.md)).
5. **Watch.** Every `cadence.watch`, `conductor run` reads hand-overs, usage limits, pull-request
   checks, stalled sessions and free disk, and acts on each line.

Every record goes to one store, a SQLite or Postgres database
([records and store](./concepts/records-and-store.md)). [Status](./status.md) lists every
capability with the test that checks it.

## What it is not

- Not a planner for a repository: each controller plans in its repository's AEP store.
- Not a copy of the forge: code, issues, pull requests, CI and releases stay on the forge.
- Not a sandbox: conductor confines nothing itself ([the host](./concepts/the-host.md)).

## Neighbours

| component | relation |
|---|---|
| [ESS](https://beyond10x.github.io/docs/ess/) ([GitHub](https://github.com/beyond10x/ess)) | conductor's nouns and commands are an ESS specification; the specifications collector runs `ess` per repository |
| [AEP](https://beyond10x.github.io/docs/aep/) ([GitHub](https://github.com/beyond10x/aep)) | each repository's planning store; the blockers collector reads it |
| [Eventlog](https://beyond10x.github.io/docs/eventlog/) ([GitHub](https://github.com/beyond10x/eventlog)) | the store's port, with the `eventlog-sqlite` and `eventlog-postgres` adapters |
| [Substrate](https://beyond10x.github.io/docs/substrate/) ([GitHub](https://github.com/beyond10x/substrate)) | opens the box an instance's runtime runs in, when its `placement` names an adapter |
| [Worktree](https://beyond10x.github.io/docs/worktree/) ([GitHub](https://github.com/beyond10x/worktree)) | managed worktrees, which the guard treats as their repository's own |
| [Gates](https://beyond10x.github.io/docs/gates/) ([GitHub](https://github.com/beyond10x/gates)) | the bot route for commits and pushes, and the secret scan of conductor's own gate |

## Where to go next

- [Getting started](./getting-started.md)
- [The host](./concepts/the-host.md)
- [Roles](./concepts/roles.md)
- [Config](./concepts/config.md)
- [Security overview](./security/overview.md)
- [One task in an existing repository](./guides/first-task-existing-repo.md)
- [Configure an instance](./guides/configure-an-instance.md)
- [The dashboard](./guides/dashboard.md)
- [Troubleshoot](./guides/troubleshoot.md)
- [CLI reference](./reference/cli.md) and [config reference](./reference/config.md)
