---
title: Confinement
sidebar_position: 3
description: How a conductor or controller session is started so that the guard runs, what bounds it, and the authority boundary the decision classes draw.
lede: A session is confined by how it is started, by the place it runs in, and by the decisions it may take; only the first two reach the guard.
source: Taskfile.yml (maintainer:start, trust); crates/conductor/src/run.rs; crates/conductor/src/confine/mod.rs; crates/conductor/src/confine/systemd.rs; crates/conductor/src/session.rs; crates/conductor/src/spawn.rs; crates/conductor/src/config.rs; crates/conductor/src/guard/rules.rs; crates/conductor/src/controller.rs; crates/conductor/src/decide.rs; docs/design/conductor.md sections 5 and 7
---

## How a session is started

Every session conductor starts goes through one command: `conductor session start --role
maintainer` (`task maintainer:start`) for conductor-dev, `--role conductor` for a conductor session
started by hand, and `conductor spawn --repository <repo>` for a controller, which `conductor run`
calls. Each hands the session to the confinement provider
the instance's `confinement` names, with the role's request: its limits, its writable places
resolved to absolute paths, its network and its `forge_token_env` names. Under `strict` a session
whose provider is not available, or whose box measures short of the request, does not start;
under `report` it starts unboxed and its measurement says why. The measurement is written to
`<state>/sessions/<session name>.envelope.json` and read into the `envelope_*` columns of
`conductor snapshot sessions`. `conductor doctor` prints whether each provider is available.

The `systemd` provider runs each session in a `systemd-run --user` unit,
`conductor_<session name>.service`, inside the instance's slice, `conductor_<session_prefix>.slice`
or `conductor.slice` without a prefix. systemd reads a `-` in a slice's name as one level down
(`conductor-ex.slice` would sit inside `conductor.slice`), so no slice name has one: a `-` of
the prefix is written `\x2d`, and every instance's slice is a sibling of every other's, capped by
its own limits only. The slice carries the instance's `confinement.limits`; the unit the role's
`limits` (`MemoryMax`, `CPUWeight`, `TasksMax`). The session runs in the foreground of the unit
under `script`, which gives it a terminal, and its first prompt is typed into it; `conductor
spawn` prints the session's id. `conductor session stop` stops the unit of conductor's session;
`conductor controller pause-controller` and `stop-controller` stop a controller's unit
(`systemctl --user stop`) before they record it paused or stopped, and `resume-controller` starts
a new unit of the same name running `claude --resume <id>` before it records it running. A unit of
the session's name that is still stopping is waited for, up to 30 s, before the new one starts;
one that still runs is refused. The provider reads the limits back from the unit's cgroup. It is
available where the user manager delegates the `cpu`, `memory` and `pids` controllers, and it
sets up no write fence yet, so its box is partial: `report` records that, `strict` stops the
session.

The `none` provider starts a session as a background Claude Code session:

```console
claude --bg -n <session name> \
  --append-system-prompt-file <profile> \
  --model <model> \
  --permission-mode bypassPermissions \
  --setting-sources user,project,local \
  --settings <role settings file> \
  "<first prompt>"
```

`conductor session start --role conductor` runs this for conductor, in the instance's records
directory, from the `conductor` role of `conductor config show`. `conductor spawn` starts a
controller in the repository's checkout with the same form: the `controller` role's profile and settings, and the
repository's charter as the first prompt.

### Setting sources

A session the `systemd` provider starts passes `--setting-sources project,local`: the project and
local settings of the start directory, not the user's own settings. The operator's personal
permissions, hooks and agents do not reach an unattended session there.

A session the `none` provider starts passes `--setting-sources user,project,local`. Claude Code
starts a `claude --bg` session in `bypassPermissions` only when the disclaimer acceptance,
`"skipDangerousModePermissionPrompt": true`, is in user or managed settings. In the role's
`--settings` file or in local settings it does not count: `claude --bg` exits with "Acceptance is
kept in your user settings, which this launch does not read", or starts the session in manual
mode. So these sessions also load the user's `~/.claude/settings.json` (its env, plugins and
hooks) and the agents in `~/.claude/agents/`. The role's `--settings` file wins on every key it
sets, and the guard's hook runs beside any hook of the user's. For a session without the user's
configuration, run the `systemd` provider, or give the session's OS user a user settings file that
holds only the acceptance.

Nothing in either source can switch the guard off. The repository's own project settings are
loaded, hooks included ([limits](/docs/security/limits)).

### The settings file

`--settings` names the role's settings file. The shipped files,
`.claude/conductor-settings.json` and `.claude/controller-settings.json`, wire the guard on every
tool and set `disableAllHooks` to false. `conductor session start --role conductor` uses the
`conductor` role's `settings`, or `.claude/conductor-settings.json` of the directory it runs in
when the role names none. A
controller's settings come from the `controller` role's `settings`: point it at
`.claude/controller-settings.json` or a file that wires the same hook.

No session writes a settings file: not by a file tool, and not by a Bash command that names it
and holds a write form ([the guard](/docs/security/the-guard#file-tools-edit-write-notebookedit)).

### Profile or agent

A role's `profile` is a file appended to the system prompt with `--append-system-prompt-file`.
Without one, a session starts with `--agent <name>`. Where user settings are not loaded (the
`systemd` provider), that agent is found only in the start directory's `.claude/agents/`, never in
`~/.claude/agents/`, so
`task agents:link`, which links into one named repository's `.claude/agents/`, serves none of
these sessions. `task maintainer:start` refuses when this checkout has no
`.claude/agents/conductor-dev.md` and the role names no profile. `conductor config validate` refuses a
`controller` role that names an `agent` and no `profile`: a controller starts only from its
profile.

### Bypass permissions, and the guard

`--permission-mode bypassPermissions` means no tool call asks first. The guard is the only check.
Claude Code refuses this mode in a background session until its disclaimer has been accepted
once by the session's OS user: run `claude --dangerously-skip-permissions` once interactively and
accept it, or set `"skipDangerousModePermissionPrompt": true` in that user's
`~/.claude/settings.json`. Both write the same key to user settings.

`task maintainer:start` refuses while a live session of the same name runs, when the profile is
not a file, and when the role runs on a harness other than `claude`.

## The working directory is the place

The guard decides by the session's working directory as the harness reports it in each call:

- conductor runs in its records directory and gets conductor's rules;
- a controller runs in its repository's checkout and gets the controller rules for that
  repository;
- a controller's workers run in its **managed worktrees**, the repository's directory under the
  instance's `checkouts.trees`, and are decided as the controller itself;
- a session anywhere else is denied every ruled tool and every tool off the read-only list.

A sub-agent's calls reach the same hook and are decided by the working directory they carry. So
the guard denies an Agent or Task call that sets `cwd` or `isolation: "remote"`.

## Protected files

| file | written by a file tool | written by a Bash write form that names it |
|---|---|---|
| `.claude/` settings files, any role's `settings` file | denied to every session | denied to every session |
| any role's `profile` file | denied to every session | denied to every session |
| conductor's config: the resolved config file, the default file, its directory, the instance's `state` | denied to a controller | denied to a controller |
| conductor's records | written only by conductor | denied to a controller |

Paths are resolved, links included, before a file tool is decided. A Bash command is matched on
its text; see [the guard](/docs/security/the-guard#bash-and-monitor) for what that misses.

## One controller per repository

The store keeps at most one running or paused controller record per repository:
`conductor controller start-controller` refuses a second one as `ControllerAlreadyRunning`. That
command records a controller; it does not start a session yet. That one session runs per
repository is a rule of the conductor profile, not something the code checks.

## Decision classes are an authority boundary

Every decision has a class ([decisions and messages](/docs/concepts/decisions-and-messages)):

- **class C**: conductor decides, within the instance's authority grant;
- **class O**: the operator decides what cannot be undone (data loss, a history rewrite), a new,
  renamed or retired repository, and an architecture decision; everything else conductor decides
  on the option that can be undone;
- **class H**: the operator's hands (setting a secret, a login, a paid account).

`conductor decision record-conductor-decision` refuses a class O or H decision as
`ReservedForOperator`. Only `answer-escalated-request` answers an escalated request, and conductor
reverses only its own decisions. The guard adds one edge in code: a `gh` credential write is
denied as class H in every guarded session.

The store tells the operator's commands from conductor's by the command, not by who runs it.
Conductor's own session may run every `conductor` command, `record-operator-decision` included,
to record a decision the operator gave in words. Such a record names the session that wrote it
(`--recorded-by`) and quotes those words (`--operator-words`), both required. A controller's
session runs only the read commands.
