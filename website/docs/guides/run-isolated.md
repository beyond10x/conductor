---
title: Run conductor isolated
sidebar_position: 8
description: What conductor's sessions can reach on a host today, how to run them in a VM, a container or a dedicated account, what works on a runner without permissions, and what the planned sandbox will change.
lede: Conductor has no operating-system sandbox yet, so the account and the machine its sessions run on are the boundary; this guide draws that boundary on purpose.
source: crates/conductor/src/guard.rs, crates/conductor/src/guard/rules.rs, crates/conductor/src/guard/shell.rs; crates/conductor/src/doctor.rs, crates/conductor/src/setup.rs, crates/conductor/src/config.rs, crates/conductor/src/trust.rs; crates/conductor/src/dashboard.rs; website/docs/security/; .engineering/planning/epic/session-confinement.md; conductor doctor, conductor setup, conductor snapshot start-snapshot, run without a config, without a store and without a forge login
---

Read the [security overview](/docs/security/overview) first. This guide answers two questions
engineers ask before they try conductor: can it be kept away from the rest of a laptop, and how
does it run where the account has few rights.

## What a session reaches today

Every session runs with `--permission-mode bypassPermissions`, as the user that started it. The
[guard](/docs/security/the-guard) decides each tool call, and it narrows **writes**:

- Edit, Write and NotebookEdit land only in the session's own checkout, managed worktrees, records
  or scratch;
- no `gh` or `glab` call that writes, and a credential write is handed to the operator (class H);
- no write to a settings file, a profile, conductor's config or (for a controller) its records;
- a controller messages only its conductor and its own sub-agents.

It does not narrow **reads**, the shell or the network ([known limits](/docs/security/limits)):

- Read, Glob, Grep, WebFetch and WebSearch are allowed anywhere, so a session can read any file
  its user can, `~/.ssh/` and other projects included;
- a Bash command the heuristic does not recognise can read, write or delete anything the user can,
  and `git push` and direct API calls are not checked;
- each repository's own `.claude/` hooks run in its controller's session, unseen by the guard.

So what the account can reach, a session can reach. The isolation is the account and the machine.

## A VM or a container for a laptop

**The question:** should conductor run in a VM or a container, to keep it away from other data
and the SSH keys on the laptop? **Yes, today that is the way.** The guard keeps a session from
writing outside its place by mistake; it does not keep a session from reading `~/.ssh/id_*` or from
running a command that the heuristic misses. Put conductor, its sessions and their programs in a
VM, a dev container or a dedicated operating-system user whose home holds only the directories
below and the logins they need. Nothing personal goes in: no SSH key, no cloud credentials, no
other checkouts.

### What to mount

Mount these read-write, at the paths the config names (`conductor config show` prints them), or
write the config for the paths inside the box:

| directory | holds |
|---|---|
| `checkouts.root` and `checkouts.trees` | the repositories and the managed worktrees the controllers work in |
| the instance's `records` | conductor's records, a git repository |
| the instance's `state` | the store, the conductor lease, the dashboard's pid file |
| the instance's `cache` | the collectors' exports |
| `~/.b10x/conductor/` | the config file and the installed profiles (`task profiles`) |
| `~/.claude/`, `~/.claude.json` | Claude Code's login, settings and session transcripts; `conductor trust` writes `~/.claude.json` |
| the conductor checkout | the role settings files the config points at (`.claude/conductor-settings.json`, `.claude/controller-settings.json`) and the `Taskfile.yml` the start tasks run from |

Install inside the box what `conductor doctor` lists: `conductor`, `claude`, `git`, `task`, `jq`,
`aep`, `worktree`, `ess`, and `gh` or `glab` for the forge of the instance's sources.

### What credentials it needs

| needs | how, inside the box |
|---|---|
| the forge CLI logged in, for each source's host | `gh auth login`, or `glab auth login --hostname gitlab.example.com` |
| git over HTTPS through that login | `gh auth setup-git`; or a deploy key per repository |
| the Claude Code login | `claude auth login`, or `claude setup-token` for a long-lived token |

It does **not** need your personal SSH key. Give the forge login a token scoped to the instance's
repositories, without admin, secrets or organization rights, and protect `main`
([hardening](/docs/security/hardening)). Run each login yourself: the guard denies a session
`gh auth` and `glab auth` writes as class H.

### Check it

```console
conductor doctor          # each program present, each forge CLI logged in; `ready` last
conductor config validate
conductor snapshot start-snapshot
```

Then provoke one denial as the [hardening checklist](/docs/security/hardening#verify-the-guard)
shows, inside the box.

The dashboard listens on `127.0.0.1` only. Inside a VM, reach it with a forward that ends inside
the box, such as `ssh -L 7313:127.0.0.1:7313` to the VM; a container needs host networking or such
a forward, since a published port does not reach a listener on the container's own `127.0.0.1`.

## A runner or an account without permissions

**The question:** how do people run conductor when their runners have no permissions? The
sessions cannot run there: they need an account that can start `claude` in bypass-permissions
mode, write its checkouts and the state directory, and hold a forge login. What does run is the
read side: checks, snapshots and views, which write only the state directory, or nothing. Use them
to try conductor out, or in CI, and run the sessions in the box above.

| command | writes | without what it needs |
|---|---|---|
| `conductor doctor` | nothing | ends `not ready: gh (not logged in to github.com)`, `not ready: config (unreadable)`, or names the program missing or too old; exits 1 |
| `conductor setup` (no `--apply`) | nothing | prints the same rows and `not ready: config (unreadable), instance (none), records (missing), trust (not trusted)`; exits 1 |
| `conductor config validate` | nothing | `conductor: config validate: read the config file <path>: No such file or directory (os error 2)`; exits 1 |
| `conductor snapshot start-snapshot` | the state directory | refuses up front naming a program not on `PATH`; with `gh` not logged in, fails the snapshot naming the collector and `gh`'s own message, and exits 1 |
| `conductor snapshot <view>`, `repository activity` | nothing | `conductor: no store at <state>/tree` before the first snapshot; exits 1 |

`conductor setup --apply`, `conductor init` and `conductor trust` write the config, the records
and `~/.claude.json`; `task conductor:start` refuses until Claude Code's bypass-permissions
disclaimer was accepted once ([start conductor](./start-conductor.md#4-accept-the-bypass-permissions-disclaimer-once)).

## What the sandbox will change

The plan holds an epic, `session-confinement`, to run every session in a box conductor sets up
and measures. It is drafted, **not built**: nothing on this page depends on it. Its stories:

- `session-limits-systemd`: each session in its own cgroup under the instance, with memory, CPU
  and process limits;
- `session-writes-bwrap`: a Bash write outside the session's own places fails with a filesystem
  error;
- `session-forge-credentials`: sessions hold read-only forge tokens, so a forge write goes only
  through the bot route;
- `session-envelope-seam`: a session that cannot be boxed is not started, or started and recorded
  as unboxed, as the config says.

The config already accepts the keys the box will read (`confinement`, and a role's `limits`,
`writable` and `network`; [config reference](/docs/reference/config)), but no command applies them
yet. Until those stories land, the VM, the container or the dedicated account is the box.

## Read next

- [Known limits](/docs/security/limits): each gap, what to do now, and what is planned.
- [Hardening checklist](/docs/security/hardening): the host, the forge and the config.
- [Start conductor](./start-conductor.md): the start tasks, inside the box.
