---
title: Start conductor and its controllers
sidebar_position: 3
description: Install the binary, name the profiles, trust the checkouts, accept the bypass-permissions disclaimer once and start the conductor session in the instance's records directory.
lede: The tasks in the repository's Taskfile start conductor in the instance's records directory, on the harness, model, profile and settings its config names, with the guard wired in.
source: Taskfile.yml; docs/design/conductor.md sections 2, 7, 11 and 12; README.md, Run sessions; .claude/conductor-settings.json, .claude/controller-settings.json; .agents/conductor.md
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

`task install` runs `cargo install --path crates/conductor --locked`. The guard hook, the watch and
the tasks all call `conductor` from `PATH`. `conductor doctor` lists every program conductor starts
and exits 1 when one is missing.

## 2. Name each role's profile and settings

Sessions start with `--setting-sources project,local`, so Claude Code loads neither your user-level
settings nor the agents in `~/.claude/agents/`. Each role therefore names its profile, the file the
session starts with through `--append-system-prompt-file`, and its settings file, which wires the
guard. With this checkout at `~/example-org/conductor`:

```yaml
    roles:
      - role: conductor
        harness: claude
        model: opus
        profile: ~/.b10x/conductor/profiles/conductor.md
        settings: ~/example-org/conductor/.claude/conductor-settings.json
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

`task install` writes the three profiles (`.agents/conductor.md`, `.agents/conductor-dev.md`,
`.agents/repo-controller.md`) of the checkout it installs from into `~/.b10x/conductor/profiles/`
(beside the config file `CONDUCTOR_CONFIG` names, when it names one), so each profile matches the
installed binary and a restart reads no checkout. A `profile` inside a git checkout is refused while
that checkout is behind its upstream.

Without a `profile`, `task conductor:start` starts the role's `agent` (default `conductor`) with
`--agent`, and finds it only in `<records>/.claude/agents/`; it refuses when that file is not
there. `conductor config validate` refuses a controller role that names an `agent` and no
`profile`.

`task agents:link REPO=<path>` serves none of these sessions either. It is for an interactive
session you start yourself in one repository: it links this checkout's three adapters
(`conductor.md`, `repo-controller.md`, `conductor-dev.md`) into `<path>/.claude/agents/` and adds
each linked path to that repository's `.git/info/exclude`, so `git status` does not show it. `REPO`
is required and must be the top of a git repository. The task writes nothing to
`~/.claude/agents/`, where every Claude Code session on the machine would see a controller agent;
it removes a link of one of those three names there that points into this checkout, as an earlier
version of the task made, and leaves any other file there with a note.

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
`claude --dangerously-skip-permissions` once interactively in the records directory and accept it,
or set `"skipDangerousModePermissionPrompt": true` in the role's settings file.
`task conductor:start` checks the role's settings file, `~/.claude/settings.json` and
`~/.claude.json`, and refuses, naming all three, until one of them holds the acceptance.

## 5. Start conductor

```console
task conductor:start      # start the conductor session in the background
task conductor:attach     # open it in this terminal
task sessions             # list the conductor and conductor-dev sessions
```

`task conductor:start` runs `conductor session start --role conductor`, which starts the session
through the confinement provider the instance's `confinement` names and records its measurement
(see [confinement](/docs/security/confinement#how-a-session-is-started)). Until conductor sets up a
`systemd` box, that is Claude Code in the background, in the instance's `records` directory, as:

```text
claude --bg -n <session name> --append-system-prompt-file <profile> --model <model>
  --permission-mode bypassPermissions --setting-sources project,local --settings <settings> <prompt>
```

The session name is the conductor role's: `<session_prefix>-conductor`, or `conductor` when the
instance has no `session_prefix`. The settings default to this checkout's
`.claude/conductor-settings.json`. The prompt tells conductor to arm its two recurring jobs (at
`cadence.cycle` and `cadence.daily` of `conductor config show`) and its watch
(`conductor watch run --follow`), then read the hand-over committed last in `docs/handoff/`, act on
its first task and run one cycle. A hand-over therefore keeps only open work.

The task refuses, and starts nothing, when:

- the role's harness is not `claude`, or the instance names no `conductor` role;
- the records directory is not a directory (`conductor init` writes it);
- the profile is not a file, or, without a profile, the agent is not in `<records>/.claude/agents/`;
- the bypass-permissions disclaimer was not accepted (step 4);
- a live session of that name runs already.

[Troubleshoot](./troubleshoot.md#a-refused-start) lists each refusal with its fix.

At session start conductor reads its profile, the instance's `rules.md`, the design and its newest
records, schedules its cycles and starts `conductor watch run --follow`.

:::note[Prefixed instances]

`task sessions` and `task conductor:attach` look for the bare names `conductor` and
`conductor-dev`. For an instance with a `session_prefix`, list its sessions with
`claude agents --json` and attach with `claude attach <id>`.

:::

## 6. Controllers

Conductor starts one controller per repository with `conductor spawn --repository <repo>`, in that
repository's checkout, through the same provider as `conductor session start`:

```text
cd <checkouts root>/<repo> && claude --bg -n <session name> --append-system-prompt-file <controller profile>
  --model <controller model> --permission-mode bypassPermissions --setting-sources project,local
  --settings <controller settings> "$(cat <records>/charters/<repo>.md)"
```

The session name is `<session_prefix>-<repo>`, or `<repo>` without a prefix. The settings are the
controller role's, such as this checkout's `.claude/controller-settings.json`, which wires the
guard. `conductor spawn` records each controller running, and refuses a second running
controller for the same repository.

**Who merges.** A charter merges by default with `merge: controller`: the controller merges its
pull request once its full gate is green. Write the line `merge: human` in a repository's charter
(`<records>/charters/<repo>.md`) when a person must merge there: the controller still runs its
gate and opens one pull request, but never merges it and never enables auto-merge; it reports
`[REPORT <repo> <dispatch>] review <url>`, and the dispatch stays open until a snapshot shows the
pull request merged. Conductor answers a controller's request to merge in such a repository with
"a person merges here". The guard does not enforce the line yet: it is an instruction in both
profiles.

:::caution[Planned]

`conductor spawn` and `conductor session start` are the only start forms conductor's profile
names, and each starts the session with its role's settings. A session someone starts by hand
with `claude` is not one of them, and nothing in code prevents it.

:::

## 7. Restart and development

`task conductor:restart` stops the background conductor, removes it from the session list (its
transcript stays) and starts a new one that reads the newest hand-over first. It stops nothing when
`conductor:start` would refuse the new one. conductor-dev runs it after conductor answers a
`[RESTART conductor]` with a hand-over.

`task dev:start` and `task dev:attach` start and open the conductor-dev session, which develops
conductor in this checkout. It starts without the guard unless its role names a `settings` file.

## What you see

`task sessions` lists the running sessions with their kind, id and state. The read-only dashboard
shows what each one works on: [read the dashboard](./read-the-dashboard.md).
