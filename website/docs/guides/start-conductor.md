---
title: Run conductor
sidebar_position: 3
description: Run conductor day to day. Start, check and stop conductord; the loop each instance's runtime runs and conductor run by hand; the controllers the loop starts and who merges; attach to a session; start the maintainer and the host roles; a new binary.
lede: conductord keeps one runtime per instance running, and each runtime runs the instance's loop, which starts the controllers. This page operates all three.
source: crates/conductor/src/attach.rs, upgrade.rs; crates/conductor/src/daemon/mod.rs, daemon/reconcile.rs, daemon/install.rs, instance.rs, run.rs, run/dispatch.rs, spawn.rs, session.rs; crates/conductor/roles/controller.md; conductor daemon --help, conductor run --help, conductor session --help
---

[Getting started](../getting-started.md) takes a host to a running instance. This page runs it
from there: what each process is and how they fit is [the host](../concepts/the-host.md).

:::caution[Unattended sessions]

Each session runs with `--permission-mode bypassPermissions`, and the guard is its only check.
Read the [security overview](/docs/security/overview) first.

:::

## conductord

```console
conductor daemon serve --install   # install and start conductord as the user unit conductord.service
curl -s http://127.0.0.1:7300/v1/instances   # every instance: placement, state, binary version, last seen
journalctl --user -u conductord.service      # one line per action: <instance>: <action> (<reason>): <result>
conductor daemon serve --uninstall # stop conductord and remove the unit
```

conductord starts each instance's runtime, `conductor instance serve`, stops the runtime of an
instance gone from the config file, and restarts one whose placement or binary changed. It does so
at its start, when the config file changes, when a runtime stops, and every
`daemon.cadence.reconcile` (60 s). `conductor doctor` prints `daemon unit: conductord.service
running` while it runs. `conductor daemon serve` without `--install` runs it in the terminal, which
shows why it does not start ([troubleshoot](./troubleshoot.md#the-daemon-does-not-start)).

## The loop

Each instance's runtime runs the instance's loop beside its API, under the lock `conductor run`
takes, `<state>/run/lock`: a `conductor run` started by hand while the runtime runs is refused, and
a runtime that finds a `conductor run` holding the lock yields until it ends.
The loop runs once per instance and makes no model call but the bounded one that
answers a class-C decision request. Every `cadence.watch` it runs the watch pass, a handler for each
line and the mailbox. Every `cadence.cycle` it takes a snapshot, publishes the board, runs
`records check` and dispatches. Once a day at `cadence.daily` it runs `metric compute`, writes the
brief `REPORT.md` and commits the records ([the loop](../concepts/the-loop.md)).

```console
conductor run --once               # without conductord: one watch tick and one cycle tick, then exit
conductor session run-ticks        # each tick the loop recorded
```

While the runtime runs, `conductor run` exits 1 with `run: another loop holds <state>/run/lock,
the instance runtime's (conductor instance serve) or a conductor run's; one runs per instance`. A
runtime started with `CONDUCTOR_RUNTIME_LOOP=off` serves the API alone. conductord never sets it.

The runtime writes `the loop runs, holding <state>/run/lock` when it takes the loop, and `the loop
yields` while another process holds it. A runtime in a box writes to
`<state>/sessions/runtime.log`. A loop that fails is named on standard error and started again a
`cadence.watch` later.

## Controllers

Each cycle, the loop starts a controller for each Active repository that has a charter,
`<records>/charters/<repo>.md`, and no running or paused controller, in name order, while fewer
than `controllers.max_working` run. It runs `conductor spawn --repository <repo>` and gives the
controller its charter and the repository's open dispatch. The session is named
`<session_prefix>-<repo>`. How it starts, and why `spawn` refuses, is in
[sessions](/docs/security/sessions#who-starts-a-session).

A controller reports with `conductor message send --to conductor --from <session name> --text -`,
the message on standard input. The loop reads the mailbox each watch tick and types its answers,
relayed watch lines and hand-overs into the controller's session with `conductor session deliver`.

**Who merges.** A charter merges with `merge: controller` by default: the controller merges its
pull request once its full gate is green. With the line `merge: human` in the charter, the
controller still runs its gate and opens one pull request, but never merges it and never enables
auto-merge. It reports `[REPORT <repo> <dispatch>] review <url>`, and the dispatch stays open until
a snapshot shows the pull request merged. The loop starts no controller for such a repository while
`controllers.max_open_reviews` of its pull requests wait for review. The guard does not enforce the
line: it is an instruction in the controller profile.

```console
conductor controller controllers
conductor controller pause-controller --controller-id <id>
conductor controller resume-controller --controller-id <id>
conductor controller stop-controller --controller-id <id>
```

Pause and stop end the session through the instance's provider. Resume starts it again from its
transcript.

## Sessions

```console
conductor session list             # one line per live session of every instance
conductor session list --instance <name>
conductor session attach           # a picker of every instance's sessions
conductor session attach --name <session name> --watch
```

In the picker, Enter attaches and `w` watches read-only. `ctrl-]` then `d` detaches, and the
session runs on. A session started before the tmux launcher can only be watched until it restarts.

The maintainer and the host roles start through `conductor session start --role <role>`, from the
profile the binary carries for the role unless the role names its own:

| session | start |
|---|---|
| maintainer, which develops conductor in a checkout of it | `conductor session start --role maintainer` in that checkout, then `conductor session attach --role maintainer` |
| supervisor, teacher | conductord, for each role `globals.roles` lists. `conductor daemon start-role --role supervisor` or `--role teacher` asks it to start one now |

The maintainer starts without the guard unless its role names a `settings` file.

## A new binary

`conductor upgrade` replaces the binary on your `PATH` and changes no runtime:
at an instance's first start conductord copies its own binary to `binaries/<version>/` in its
directory, and the runtime runs that copy. `conductor instance upgrade --instance <name> --tag
v<x.y.z>` restarts one runtime on a release, and rolls it back unless it answers within
`daemon.upgrade_timeout` ([upgrade and roll back](./upgrade-and-rollback.md)). To run conductord
itself on the new binary, `conductor daemon serve --uninstall`, then `--install`.

The loop keeps nothing in memory that is not in the store or its state files, so a restarted
runtime hands nothing over.
