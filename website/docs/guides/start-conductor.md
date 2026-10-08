---
title: Start conductor and its controllers
sidebar_position: 2
description: Install the binary, link the adapters, trust the checkouts and start the conductor session in the instance's records directory.
lede: The tasks in the repository's Taskfile start conductor in the instance's records directory, on the harness and model its config names, with the guard wired in.
source: Taskfile.yml; docs/design/conductor.md sections 2, 7, 11 and 12; README.md, Run; .claude/conductor-settings.json, .claude/controller-settings.json, .claude/agents/
---

Run these from a checkout of the conductor repository, after you have
[configured an instance](./configure-an-instance.md). They need `task` (go-task), `jq` and Claude
Code (`claude`), and they read the instance from `conductor config show`.

:::caution[Starting sessions]

Each session runs with `--permission-mode bypassPermissions`, and the guard is its only check. Read
[the guard and its limits](../concepts/the-guard.md) first.

:::

## 1. Install the binary

```console
task install
```

This runs `cargo install --path crates/conductor --locked`. The guard hook, the watch and the
tasks all call `conductor` from `PATH`.

## 2. Link the adapters

```console
task agents:link
```

Conductor runs in its records directory and each controller in its repository's checkout, so the
Claude Code adapters must be reachable from anywhere. `task agents:link` links this checkout's
`.claude/agents/conductor.md`, `repo-controller.md` and `conductor-dev.md` into
`~/.claude/agents/`, one symbolic link each. A link to the same file is kept; any other file
already there is refused, never overwritten.

## 3. Trust the checkouts

```console
task trust
```

This marks every git checkout directly under `checkouts.root`, and the instance's `records`
directory, as a trusted Claude Code workspace, so a session can start there without a prompt. It
also means each of those repositories' own project settings and hooks run without a prompt.

## 4. Start conductor

```console
task conductor:start      # start the conductor session in the background
task conductor:attach     # open it in this terminal
task sessions             # list the conductor and conductor-dev sessions
```

`task conductor:start` starts Claude Code in the background with the working directory the
instance's `records`, the agent `conductor`, the model of the instance's `conductor` role and the
settings file that wires the guard (this checkout's `.claude/conductor-settings.json` unless the
role names another). It refuses unless the role's harness is `claude`, and while a session named
`conductor` is running. The guard gives conductor its rules only in that directory.

At session start conductor reads its profile, the instance's `rules.md`, the design and its newest
records, schedules its cycles and starts `conductor watch run --follow`.

## 5. Controllers

Conductor starts one controller per repository, in that repository's checkout, with the
`controller` role's model, `agent` (such as `repo-controller`) and `settings` (such as this
checkout's `.claude/controller-settings.json`, which wires the guard). It records each one with
`conductor controller start-controller`, which refuses a second controller for the same
repository.

:::caution[Planned]

`conductor controller start-controller` records the controller only; the conductor session starts
the harness session itself.

:::

## 6. Restart and development

`task conductor:restart` stops the background conductor and starts a new one that reads the newest
hand-over first. conductor-dev runs it after conductor answers a `[RESTART conductor]` with a
hand-over.

`task dev:start` and `task dev:attach` start and open the conductor-dev session, which develops
conductor in this repository. It starts without the guard.

## What you see

`task sessions` lists the running sessions with their kind, id and state. The read-only dashboard
shows what each one works on: [read the dashboard](./read-the-dashboard.md).
