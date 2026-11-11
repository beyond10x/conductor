---
title: Run conductor isolated
sidebar_position: 9
description: What conductor can reach on a host today, how to run it in a VM, a container or a dedicated account, what works on a runner, and what is planned.
lede: An instance's runtime can run in a substrate box, but the sessions conductor starts run unboxed as your user. The account and the machine they run on are their boundary, and this guide draws it.
source: crates/conductor/src/placement.rs; crates/conductor/src/confine/mod.rs, crates/conductor/src/systemd.rs; crates/conductor/src/guard.rs; crates/conductor/src/doctor.rs, setup.rs, trust.rs, store.rs; crates/conductor/src/daemon/mod.rs; .engineering/planning/epic/session-confinement.md
---

Read the [security overview](../security/overview.md) first.

## What runs where today

| process | boundary |
|---|---|
| conductord | your user, as the user unit `conductord.service`, or the system unit on a box with `daemon.loopback: false` |
| an instance's runtime, `conductor instance serve` | a substrate box when `placement` names an adapter (`linux`, `systemd`), else a user unit with no box |
| every session: the maintainer, a controller, a host role | unboxed, as your user, with `--permission-mode bypassPermissions` |

A runtime's box reads the config file, the directories of `conductor` and the harness, the
checkouts root and each forge's CLI and login, and writes the instance's state, records and
cache and the directory of a sqlite store. Its network is the one egress aperture `conductor`,
one destination, which you declare on `substrate-daemon` as `api.anthropic.com`: on substrate
0.20.0 the box reaches no forge. The token the daemon proxies writes with reaches the runtime
through a sealed slot, not through the box's environment, where you declare that slot on
`substrate-daemon`; without it the box starts and its runtime refuses each proxied write
([the host](../concepts/the-host.md#placement)). The runtime starts no session yet.

A session is narrowed by [the guard](../security/the-guard.md) only, which narrows writes and not
reads, the shell or the network ([known limits](../security/limits.md)). What the account can
reach, a session can reach.

## A VM or a container for a laptop

Should conductor run in a VM or a container, to keep it away from other data and the SSH keys on
the laptop? Yes. The guard does not keep a session from reading `~/.ssh/id_*`, or from running a
command its shell rule does not recognise.

Put conductor, its sessions and their programs in a
VM, a dev container or a dedicated operating-system user whose home holds only the directories
below and the logins they need: no SSH key, no cloud credentials, no other checkouts.

### What to mount

Mount these read-write at the paths `conductor config show` prints, or write the config for the
box's paths:

| directory | holds |
|---|---|
| `checkouts.root` and `checkouts.trees` | the repositories and the managed worktrees the controllers work in |
| the instance's `records` | conductor's records, a git repository |
| the instance's `state` | the conductor lease, the watch's files, `run/`, `sessions/` and the runtime's `api.sock` |
| the instance's `cache` | the collectors' exports |
| `~/.b10x/conductor/` | the config file, the default store `conductor.sqlite`, and `daemon/`: the runtimes' binaries and the daemon's token |
| `~/.claude/`, `~/.claude.json` | Claude Code's login, settings and session transcripts. `conductor trust` writes `~/.claude.json` |

Install inside the box what `conductor doctor` lists: `conductor`, `claude`, `git`, `jq`,
`aep`, `worktree`, `ess`, `gh` or `glab` for the forge of the instance's sources, and
`substrate-daemon` when a `placement` opens a box.

### What credentials it needs

| needs | how, inside the box |
|---|---|
| the forge CLI logged in, for each source's host | `gh auth login`, or `glab auth login --hostname gitlab.example.com` |
| git over HTTPS through that login | `gh auth setup-git`, or a deploy key per repository |
| the Claude Code login | `claude auth login`, or `claude setup-token` for a long-lived token |

It does not need your personal SSH key. Give the forge login a token scoped to the instance's
repositories, without admin, secrets or organization rights, and protect `main`
([hardening](../security/hardening.md)). Run each login yourself: the guard denies a session
`gh auth` and `glab auth` writes as class H.

### Check it

```console
conductor doctor          # `ready` last
conductor config validate
conductor snapshot start-snapshot
```

Doctor ends `ready` only once each instance writes its forge route: `forge: {route: direct}`,
with which a controller writes its pull requests with the forge login inside the box, or a
`gates_policy` for the bot route
([configure an instance](./configure-an-instance.md#1-write-the-file)).

Then provoke one denial inside the box, as the [hardening checklist](../security/hardening.md)
shows.

The daemon's loopback listener is on `127.0.0.1` only. Reach it in a VM with a forward that ends
inside the box, such as `ssh -L 7300:127.0.0.1:7300`. A container needs host networking or such a
forward: a published port does not reach the container's own `127.0.0.1`.

## A runner or an account without permissions

The sessions cannot run on a runner without permissions: they need an account that can start
`claude` in bypass-permissions mode, write its checkouts and the state directory, and hold a
forge login. Checks, snapshots and views do run there, and write only the store, or nothing.

| command | writes | without what it needs |
|---|---|---|
| `conductor doctor` | nothing | ends `not ready: gh (not logged in to github.com)`, `not ready: config (unreadable)`, or lists the program missing or too old. Exits 1 |
| `conductor setup` (no `--apply`) | nothing | prints the same rows and `not ready: config (unreadable), instance (none), records (missing), trust (not trusted)`. Exits 1 |
| `conductor config validate` | nothing | `conductor: config validate: read the config file <path>: No such file or directory (os error 2)`. Exits 1 |
| `conductor snapshot start-snapshot` | the store | refuses up front, naming a program not on `PATH`. With `gh` not logged in it fails the snapshot, naming the collector and `gh`'s own message. Exits 1 |
| `conductor snapshot <view>`, `repository activity` | nothing | `conductor: no store at <path of the store file>` before the first snapshot. Exits 1 |

`conductor init` and `conductor trust` write the records and `~/.claude.json`.
`conductor setup --apply` writes a `conductor.config/1` file, which the daemon refuses until
[`conductor config migrate`](./configure-an-instance.md#5-migrate-a-conductorconfig1-file)
converts it. `claude --bg` refuses `bypassPermissions` until Claude Code's disclaimer was
accepted once.

## What is planned for sessions

The epic `session-confinement`, drafted and not built, runs every session conductor starts in a
box with CPU, memory and process limits, writes to its own places only, and read-only forge
tokens. A role's `limits`, `writable` and `network` already go into each session's request, and
the `none` provider applies none of them. Until the epic lands,
the VM, the container or the dedicated account is the box. Each open gap is in
[known limits](../security/limits.md), and how a session starts in
[sessions](../security/sessions.md).
