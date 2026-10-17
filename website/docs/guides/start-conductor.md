---
title: Start conductor and its controllers
sidebar_position: 3
description: Install the binary, name the profiles, trust the checkouts, accept the bypass-permissions disclaimer once and start the instance's loop, conductor run, as a user unit.
lede: conductor run --install starts the instance's loop as a user unit; the loop starts each controller on the harness, model, profile and settings its config names, with the guard wired in.
source: Taskfile.yml; docs/design/conductor.md sections 2, 7, 11 and 12; docs/design/conductor-run.md; crates/conductor/src/run.rs; README.md, Run sessions; .claude/controller-settings.json; .agents/repo-controller.md
---

Run these from a checkout of the conductor repository, after you have
[configured an instance](./configure-an-instance.md) and
[initialised its records](./initialise-the-records.md). They need `task` (go-task), `jq` and
Claude Code (`claude`), and they read the instance from `conductor config show`.

:::caution[Starting sessions]

Each session runs with `--permission-mode bypassPermissions`, and the guard is its only check. Read
the [security overview](/docs/security/overview) first.

:::

## 1. Install the binary

```console
task install
conductor doctor
```

`task install` runs `cargo install --path crates/conductor --locked`. The guard hook, the loop and
the tasks all call `conductor` from `PATH`. `conductor doctor` lists every program conductor starts
and exits 1 when one is missing.

## 2. Name each role's profile and settings

Sessions start with `--setting-sources project,local`, so Claude Code loads neither your user-level
settings nor the agents in `~/.claude/agents/`. Each role that starts a session therefore names its
profile, the file the session starts with through `--append-system-prompt-file`, and its settings
file, which wires the guard. With this checkout at `~/example-org/conductor`:

```yaml
    roles:
      - role: conductor
        harness: claude
        model: opus
      - role: conductor-dev
        harness: claude
        model: opus
        profile: ~/.b10x/conductor/profiles/conductor-dev.md
      - role: controller
        harness: claude
        model: opus
        profile: ~/.b10x/conductor/profiles/repo-controller.md
        settings: ~/example-org/conductor/.claude/controller-settings.json
```

The `conductor` role starts no session: `conductor run` is the loop, and the role's `model` answers
the class-C decision requests the loop decides.

`task install` writes the two profiles (`.agents/conductor-dev.md`, `.agents/repo-controller.md`)
of the checkout it installs from into `~/.b10x/conductor/profiles/` (beside the config file
`CONDUCTOR_CONFIG` names, when it names one), so each profile matches the installed binary and a
restart reads no checkout.

Without a `profile`, `task maintainer:start` starts conductor-dev with `--agent conductor-dev`,
which it finds only in this checkout's `.claude/agents/`; it refuses when that file is not there.
`conductor config validate` refuses a controller role that names an `agent` and no `profile`.

`task agents:link REPO=<path>` serves none of these sessions either. It is for an interactive
session you start yourself in one repository: it links this checkout's two adapters
(`repo-controller.md`, `conductor-dev.md`) into `<path>/.claude/agents/` and adds each linked path
to that repository's `.git/info/exclude`, so `git status` does not show it. `REPO` is required and
must be the top of a git repository. The task writes nothing to `~/.claude/agents/`, where every
Claude Code session on the machine would see a controller agent; it removes a link there named
`conductor.md` (the retired conductor adapter), `repo-controller.md` or `conductor-dev.md` that
points into this checkout, as an earlier version of the task made, and leaves any other file there
with a note.

## 3. Trust the checkouts

```console
task trust
```

This runs `conductor trust`, which marks every git checkout directly under `checkouts.root`, and
the instance's `records` directory, as a trusted Claude Code workspace in `~/.claude.json`, so a
session can start there without a prompt. It also means each of those repositories' own project
settings and hooks run without a prompt.

## 4. Accept the bypass-permissions disclaimer once

`claude --bg` refuses `bypassPermissions` until Claude Code's disclaimer was accepted. Either run
`claude --dangerously-skip-permissions` once interactively in a checkout and accept it, or set
`"skipDangerousModePermissionPrompt": true` in the role's settings file.

## 5. Start the loop

```console
conductor run --install   # start the loop as the user unit conductor_<instance>-run.service
conductor run --once      # or run one watch tick and one cycle tick in this terminal, then exit
task sessions             # list the instance's sessions
```

`conductor run` is one process per instance and makes no model call but the bounded one that
answers a class-C decision request. Every `cadence.watch` it runs the watch pass, a handler for each
line (a hand-over, a usage limit, a pull request's checks, a stalled or gone controller, low disk)
and the mailbox; every `cadence.cycle` the snapshot, the board, `records check` and dispatch; once a
day at `cadence.daily` `metric compute` and the brief `REPORT.md` in the records. It keeps nothing
in memory that is not in the store, so a restart loses nothing. `--install` starts it through
`systemd-run`; under the `none` provider it prints the command to run instead.
`conductor run --uninstall` stops the unit.

:::note[Watch or type into a running session]

`conductor session attach` opens a picker of every instance's sessions; Enter attaches, `w`
watches read-only, and `ctrl-]` then `d` detaches while the session runs on. Attach to one
directly with `conductor session attach --name <session name>`, and list them with
`conductor session list`. It works from any directory and for any harness. A session started
before the tmux launcher can only be watched until it restarts.

:::

## 6. Controllers

Each cycle, `conductor run` starts one controller for each Active repository that has a charter
in `<records>/charters/` and no running controller, up to `controllers.max_working`, as
`conductor spawn --repository <repo>` does: in that repository's checkout, through the provider
the instance's `confinement` names (see
[confinement](/docs/security/confinement#how-a-session-is-started)):

```text
cd <checkouts root>/<repo> && claude --bg -n <session name> --append-system-prompt-file <controller profile>
  --model <controller model> --permission-mode bypassPermissions --setting-sources project,local
  --settings <controller settings> "$(cat <records>/charters/<repo>.md)"
```

The session name is `<session_prefix>-<repo>`, or `<repo>` without a prefix. The settings are the
controller role's, such as this checkout's `.claude/controller-settings.json`, which wires the
guard. `conductor spawn` records each controller running, and refuses a second running
controller for the same repository.

A controller reports with `conductor message send --to conductor --from <session name> --text -`,
the message on standard input; the loop reads conductor's mailbox each watch tick and types its
answers, relayed watch lines and `[HANDOVER <repo>]` into the controller's session with
`conductor session deliver`.

**Who merges.** A charter merges by default with `merge: controller`: the controller merges its
pull request once its full gate is green. Write the line `merge: human` in a repository's charter
(`<records>/charters/<repo>.md`) when a person must merge there: the controller still runs its
gate and opens one pull request, but never merges it and never enables auto-merge; it reports
`[REPORT <repo> <dispatch>] review <url>`, and the dispatch stays open until a snapshot shows the
pull request merged. The loop starts no controller for such a repository while
`controllers.max_open_reviews` of its pull requests are open. The guard does not enforce the line
yet: it is an instruction in the controller profile.

:::caution[Planned]

`conductor spawn` and `conductor session start` are the only start forms conductor uses, and each
starts the session with its role's settings. A session someone starts by hand with `claude` is not
one of them, and nothing in code prevents it.

:::

## 7. Restart and development

`conductor run --uninstall`, then `conductor run --install`, restarts the loop, for example after
`task install` put a release that changed it in place. The loop keeps no context, so nothing is
handed over; it restarts controllers itself.

`task maintainer:start` and `task maintainer:attach` start and open the conductor-dev session,
which develops conductor in this checkout. It starts without the guard unless its role names a
`settings` file.

## What you see

`task sessions` lists the running sessions with their kind, id and state. The dashboard
shows what each one works on: [read the dashboard](./read-the-dashboard.md).
