---
title: Troubleshoot
sidebar_position: 6
description: Check the programs with conductor doctor, read why a start was refused or a snapshot failed, read the guard's denials, and find a session that runs without the guard.
lede: Conductor refuses rather than half-works, and every refusal names its cause, so start from the message and the command that prints it.
source: crates/conductor/src/doctor.rs; crates/conductor/src/snapshot.rs, crates/conductor/src/collect/; crates/conductor/src/guard.rs; Taskfile.yml, conductor:start; .claude/conductor-settings.json, .claude/controller-settings.json
---

## Start with doctor

```console
conductor --version
conductor config validate
conductor doctor
```

`conductor --version` names the installed release, `conductor 0.12.0`; quote it when you report a
problem.

Every command resolves the instance first and stops on a config problem, named by its YAML path.
The tasks read the instance through `conductor config show`, so a config that does not validate
shows up there as a missing role or records directory; validate it first.

`conductor doctor` first names the machine: its host name, operating system and release,
architecture, and the conductor version, so a report you post says where it came from. It then
lists every program conductor starts (`git`, `gh`, `glab`, `claude`, `jq`,
`task`, `aep`, `worktree`, `ess`), present with its version line, missing, or not needed. A forge's
CLI is needed only for an instance with a source on that forge, `gh` for `github` and `glab` for
`gitlab`, across every instance of the config file; the other reads `not needed` whether it is
installed or not. A GitLab instance needs `glab` logged in to its source's `host`
([GitLab](./gitlab.md)).

For each host of the config file's sources, `github.com` for a `github` source and the `host` of a
`gitlab` source, across every instance, doctor asks the forge's CLI, when it is installed, which
account it is logged in as there (`gh auth status --hostname <host>`,
`glab auth status --hostname <host>`), and prints the login name only, never a token or an e-mail
address:

```console
gh: logged in to github.com as example-user
glab: not logged in to gitlab.example.com
```

A CLI not logged in to such a host makes the run not ready; `gh auth login --hostname github.com`
or `glab auth login --hostname <host>` logs it in.

Doctor then names the pins it checks against: the nearest `b10x.toml` in the working directory or
a directory above it, or `pins: none` when there is none. `aep`, `ess` and `worktree` keep the pins
the binary was built with (`meets the build's pin`) unless that file pins them itself. Its last
line is the verdict:

| last line | meaning |
|---|---|
| `ready` | every program it needs is present and meets its pin; exit 0 |
| `not ready: config (unreadable), …` | the config file does not load; the `config:` lines above say why |
| `not ready: <program> (missing), …` | the program is not on `PATH`; install it |
| `not ready: <program> (too old), …` | its version is older than the pin; upgrade it |
| `not ready: <program> (version unread), …` | it is pinned and its `--version` names no version doctor can read |
| `not ready: <program> (pin unread), …` | its pin is not `x.y.z` or `x.y`; correct the `b10x.toml` |
| `not ready: <program> (not logged in to <host>), …` | the forge's CLI is not logged in to a host of the config file's sources; log it in to that host |

Doctor exits 1 on any `not ready:` line, and writes nothing.

## A refused start

`task conductor:start` checks everything before it starts a session, and starts nothing when a
check fails. Each message names its fix:

| message | cause | fix |
|---|---|---|
| `conductor config show names no role conductor` | the instance's `roles` has no `conductor` entry, or the config does not load | `conductor config validate`; add the role |
| `the role conductor runs on codex …; this task starts claude only` | the role's `harness` is not `claude` | set `harness: claude` for this role |
| `the records directory … is not a directory` | the records were never written | [initialise the records](./initialise-the-records.md) with `conductor init` |
| `the profile … of the role conductor is not a file` | the `profile` path is wrong | run `task install` in a checkout of the conductor repository, which writes `~/.b10x/conductor/profiles/conductor.md`, and point it there |
| `the agent … is not in <records>/.claude/agents/` | the role names no `profile`, and sessions load no user-level agent | name a `profile` for the role ([start conductor](./start-conductor.md#2-name-each-roles-profile-and-settings)) |
| `Claude Code's bypass-permissions disclaimer is not accepted …` | `claude --bg` refuses `bypassPermissions` until the disclaimer was accepted once | run `claude --dangerously-skip-permissions` once in the records directory, or set `"skipDangerousModePermissionPrompt": true` in the role's settings file |
| `a session named … is already running; see task sessions` | a live session of that name exists | attach to it, or stop it before you start another; two conductors of one instance would answer the same controllers |

`task conductor:restart` makes the same checks before it stops the running session, so a restart
that would not start again stops nothing. `conductor config validate` refuses a controller role
that names an `agent` and no `profile`, before any controller is started.

## A failed snapshot

There are two ways a snapshot does not complete.

**Refused up front.** When a program a collector would start is not on `PATH`, `start-snapshot`
names it, takes no snapshot and exits 1:

```console
$ conductor snapshot start-snapshot
conductor: snapshot start-snapshot: `aep` is not on PATH, and a snapshot's collectors start it; `conductor doctor` lists every program conductor starts
```

Install the program and run `conductor doctor`.

**Failed by a collector.** A collector that runs and does not answer fails the snapshot. The
command prints the id and one line naming the collector and the command that failed, and exits 1.
The failed snapshot is kept with its reason and never read as the current state; the newest
complete one stays current. Read the reasons back with:

```console
conductor snapshot snapshots --format json | jq -r '.[] | select(.state == "Failed") | .failure_reason'
```

| reason names | likely cause | fix |
|---|---|---|
| `blockers: repository <repo>, tried twice: git fetch --quiet origin …` | that member's checkout has an `origin` that cannot be fetched; a checkout with no `origin` is read at its `HEAD` and not fetched ([initialise the records](./initialise-the-records.md#4-take-the-first-snapshot)) | make its `origin` reachable, with `main` pushed, or rewrite the workspace file |
| `blockers: list the members of …/workspace.yaml` | `aep` cannot read the workspace file | `conductor config workspace --check`, then `conductor config workspace` |
| `repositories: …` or `github: …`, with a `gh` command | `gh` is not logged in, or the forge did not answer | `gh auth status`, then take a new snapshot |
| `specifications: …` | `ess` is missing or did not run for a repository that holds a specification; a specification `ess` refuses is recorded as `Refused`, not a failure | `conductor doctor`, then the command the reason names |

A member whose checkout is absent is not a failure: the collector skips it and names it on one line
of standard error, `member <name>: checkout absent, skipped`.

## On macOS

| message | cause | fix |
|---|---|---|
| `ess generate synthesize` refuses files that carry `com.apple.provenance` | an `ess` older than 0.54.0 runs in that shell: macOS marks files a process Claude Code started wrote with `com.apple.provenance`, and only `ess` 0.54.0 and later accept it | run `command -v ess` and `ess --version` in the same shell; install the `ess` your `b10x.toml` pins (`b10x install ess`) and remove the older copy from `PATH` |

`xattr -l <file>` lists a file's extended attributes, `com.apple.provenance` among them.

## Reading guard denials

The guard records every denial, and only denials:

```console
$ conductor guard guard-decisions
guard_decision_id  repository  session_ref  tool  target  verdict  reason  decided_at
```

Each row names the repository, the session, the tool, the target (a path, a command or a
recipient), the reason and the time. `--format json` gives the same rows for `jq`. Run it with the
instance's config, since denials are kept in the instance's state directory.

The reason says which rule denied the call. Most denials are the guard doing its job: a controller
that tried to write outside its checkout, message another controller, or call `gh` or `glab` for a
write. A
denial is not the session's last word: the session sees the reason and can take another route.

- **A tool with no rule.** A tool not on the built-in read-only list, MCP tools included, is denied
  with the list in the reason. When the tool only reads, add it to the role's `read_only_tools`
  ([instances and config](../concepts/instances-and-config.md#roles-profile-settings-read-only-tools)).
- **A credential write.** A `gh secret`, `gh auth`, `gh ssh-key` or `gh gpg-key` write, and a
  `glab variable`, `glab auth`, `glab ssh-key`, `glab gpg-key`, `glab deploy-key`, `glab token`, `glab config` or `glab securefile`
  write, is class H: the denial carries the exact command for the operator to run themselves.
- **Every call denied.** When every tool call of a session is blocked, the hook could not run
  `conductor`: the settings run `conductor guard record-guard-decision --from-pre-tool-use || exit 2`,
  so a binary missing from the session's `PATH` blocks the call. Install it with `task install`.

What each rule allows and denies is in [the guard](/docs/security/the-guard).

## A session that started without the guard

A session is guarded only when it started with a settings file that wires the hook, such as this
repository's `.claude/conductor-settings.json` or `.claude/controller-settings.json`. The tasks and
conductor's start form for controllers pass one with `--settings`. A session started or resumed by
hand without `--settings`, or conductor-dev started without a `settings` for its role, runs with
`bypassPermissions` and no guard at all.

The guard records nothing for an allowed call, so an unguarded session leaves no trace in
`guard-decisions`; the absence of denials proves nothing. Check how the session was started. When
it was not started through the tasks or with the role's settings:

1. Stop it: `claude stop <id>` (find the id with `claude agents --json`).
2. Start it again the guarded way: `task conductor:start` for conductor; for a controller, let
   conductor start it with its `--settings`.
3. Review what it changed while unguarded, in its checkout and on the forge.

A second conductor of the same instance started with the role's settings is refused when it
starts, and the refusal names the session that holds the conductor lease
(`conductor session leases` lists it); one started without them is not. Neither is stopped from
running beside the first yet. See [the limits](/docs/security/limits).
