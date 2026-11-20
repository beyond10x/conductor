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
| a session the runtime starts: the maintainer, a controller, the director | in the runtime's box when the runtime runs in one, as `claude -p` with stream-json input ([the child launch](../security/sessions.md#in-a-box-the-child-launch)), else unboxed as your user through `claude --bg`, with `--permission-mode bypassPermissions` |
| a host role, a session you start by hand | unboxed, as your user |

A runtime's box reads the config file, the directories of `conductor` and the harness, each
forge's CLI and login, and git's and the `worktree` CLI's configuration, and writes the
instance's state, records and cache, the directory of a sqlite store, the checkouts root and the
managed trees. Its network is the egress aperture `conductor` to the model host and one aperture
per forge host of the sources, `forge_<host>`, each one destination that you declare on
`substrate-daemon` ([the host](../concepts/the-host.md#placement)); a forge whose aperture the
daemon does not declare is not reached, and `conductor doctor` names the flag. The token the
daemon proxies writes with reaches the runtime through a sealed slot, not through the box's
environment, where you declare that slot on `substrate-daemon`; without it the box starts and
its runtime refuses each proxied write. The sessions the runtime starts run in its box, children
of the runtime: they write what the box writes and reach what it reaches.

A toolchain or a tool cache of the host reaches the box through the instance's `box` key: each
directory of `box.tools` is read by the box at its own path, and `box.env` is set in the box's
start, which every session of the box inherits:

```yaml
box:
  tools: [~/.cargo, ~/.rustup]
  env: {CARGO_HOME: ~/.cargo, RUSTUP_HOME: ~/.rustup}
```

A declared directory that holds the directory of `conductor` or of the harness (`~/.cargo` over
`~/.cargo/bin`) takes its place. One box reads at most four directories: with the directories of
`conductor` and the harness, two declared directories usually fit, and more are refused
(`placement.mounts-exceeded`), which `conductor config validate` names. The box reads everything
under a declared directory, `~/.cargo/credentials.toml` included. A declared directory that is not
there is left out, and `conductor doctor` names it (`box <instance> tools:`); one that holds the
config file or `~/.b10x/conductor/daemon` is refused (`placement.tool-directory-shared`).
`box.env` names no variable conductor sets itself: `HOME`, `PATH`, `CLAUDE_CONFIG_DIR`,
`XDG_STATE_HOME` or a `CONDUCTOR_*` variable.

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

For an instance whose `placement` opens a box, accept Claude Code's bypass-permissions
disclaimer for the box once: the box keeps its harness state in `<state>/claude`, and its
sessions are refused `bypassPermissions` until its user settings accept it. Write
`{"skipDangerousModePermissionPrompt": true}` into `<state>/claude/settings.json` (merge the key
into the file when it is there), or set it in the managed settings
`/etc/claude-code/managed-settings.json`. Doctor prints `box <instance> bypass` and is not ready
without it ([the host](../concepts/the-host.md#placement)).

Doctor ends `ready` only once each instance writes its forge route: `forge: {route: direct}`,
with which a controller writes its pull requests with the forge login inside the box, or a
`gates_policy` for the bot route
([configure an instance](./configure-an-instance.md#1-write-the-file)).

On the bot route, the box's records commit runs `b10x-gates` with `XDG_CONFIG_HOME` set to the
bot route's config directory of the box: `<cache>/xdg-config` under the instance's `cache`, or
the directory `box.env` names as `XDG_CONFIG_HOME`; and with `B10X_GATES_POLICY` set to the
instance's `gates_policy`, which must lie under a directory the box reads, such as the `cache`.
Put the bot's config and key there yourself, `b10x/b10x-bot.json` and
`b10x/b10x-bot.private-key.pem` under that directory: conductor neither copies nor reads them.
Doctor names each one that is missing, with its path (`box <instance> bot route:`), and does not
fail on it.

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
