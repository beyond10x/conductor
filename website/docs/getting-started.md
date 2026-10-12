---
title: Getting started
sidebar_position: 2
description: Install the conductor CLI, check the programs it starts, write a one-instance config, validate it and take one snapshot of two local repositories.
---

This page installs the `conductor` CLI, checks the programs it starts, writes the smallest config,
checks it, and takes one snapshot of two tiny local repositories. It starts no session;
[Start conductor](./guides/start-conductor.md) does that.

The CLI prints absolute paths; the output here writes the home directory as `~`. Version lines,
ids, times and sizes differ on your machine.

## Install

You need a Rust toolchain (edition 2024) with `cargo`, and `git`. Taking a snapshot also runs `aep`
and `claude` (Claude Code), `gh` for a `github` source and `ess` for repositories with a
specification; the [README](https://github.com/beyond10x/conductor#requirements) lists every tool.

Install from the repository:

```console
cargo install --git https://github.com/beyond10x/conductor --tag v0.2.0 --locked conductor-cli
```

or, from a checkout, `task install`, which runs `cargo install --path crates/conductor --locked`.
Either installs one binary, `conductor`; `conductor --help` lists its commands.

## Check the programs

`conductor doctor` lists every program conductor starts, present with the first line its
`--version` prints, or missing:

```console
$ conductor doctor
git: present, git version 2.55.0
gh: present, gh version 2.100.0 (2026-09-04)
claude: present, 2.1.296 (Claude Code)
jq: present, jq-1.8.2
task: present, 3.53.1+ff3372fc
aep: present, aep 0.71.2
worktree: present, b10x-worktree-cli 0.14.0
ess: present, ess 0.59.0
pins: no pins, no b10x.toml in ~/work or a directory above it
```

It writes nothing and exits 1 when a program is missing. Run at or below a directory holding a
`b10x.toml` (a checkout of the conductor repository has one), it also says whether each pinned
program meets its pin, and exits 1 when one is older.

## Two repositories to watch

A `local` source lists the repositories under a directory. Make two:

```console
$ mkdir -p ~/work/alpha ~/work/beta
$ git -C ~/work/alpha init -q -b main && git -C ~/work/beta init -q -b main
$ echo "# alpha" > ~/work/alpha/README.md && git -C ~/work/alpha add README.md && git -C ~/work/alpha commit -q -m "Add a README"
$ echo "# beta" > ~/work/beta/README.md && git -C ~/work/beta add README.md && git -C ~/work/beta commit -q -m "Add a README"
$ echo "fn main() {}" > ~/work/alpha/main.rs && git -C ~/work/alpha add main.rs && git -C ~/work/alpha commit -q -m "Add the program"
```

## A config

Write `~/.b10x/conductor/conductor.yaml`, the default path; `--config` or `$CONDUCTOR_CONFIG` names
another. An instance needs a name, its sources and its checkouts; every other key has a default.
The repository's `conductor.example.yaml` has two complete instances to copy from.

```yaml
version: conductor.config/1
instances:
  - name: demo
    sources:
      - local: ~/work
    checkouts:
      root: ~/work
      trees: ~/.local/state/worktree/trees/work
```

Check it:

```console
$ conductor config validate
~/.b10x/conductor/conductor.yaml: valid, 1 instance(s): demo
```

See the effective values after defaults. The first lines name where the instance keeps its
records, its store and its cache:

```console
$ conductor config show | head -10
version: conductor.config/1
default: demo
instances[0].name: demo
instances[0].session_prefix: null
instances[0].sources[0].local: ~/work
instances[0].checkouts.root: ~/work
instances[0].checkouts.trees: ~/.local/state/worktree/trees/work
instances[0].records: ~/.b10x/conductor/demo/records
instances[0].state: ~/.b10x/conductor/demo/state
instances[0].cache: ~/.cache/b10x/conductor/demo
```

`conductor config show --format json` prints the same as JSON; the profiles read a value such as
`thresholds.disk_low` from it. Every key is in the [config reference](/docs/reference/config).

## One snapshot

```console
$ conductor snapshot start-snapshot
01a11c9d-4c98-7057-9d12-f1bf3c34ae9f
$ conductor snapshot snapshots
snapshot_id                           state     started_at                      disk_free_bytes  failure_reason
01a11c9d-4c98-7057-9d12-f1bf3c34ae9f  Complete  2026-10-08T17:43:53.196033707Z  39316787200      null
```

`start-snapshot` ran the five collectors and completed the snapshot. Its rows are in the views:

```console
$ conductor snapshot repositories --format json | jq -c '.[] | {repository, main_commits_7d, real_commits_7d, dirty_files, in_catalog}'
{"repository":"alpha","main_commits_7d":2,"real_commits_7d":1,"dirty_files":0,"in_catalog":null}
{"repository":"beta","main_commits_7d":1,"real_commits_7d":0,"dirty_files":0,"in_catalog":null}
$ conductor repository activity
repository  activity  decided_by  marked_by  reason
alpha       Active    Snapshot    null       null
beta        Inactive  Snapshot    null       null
```

`beta`'s only commit touches documentation, which is not a real commit, so the snapshot decides it
Inactive; `alpha` has one real commit in 7 days and is Active. Conductor processes issues and pull
requests only for Active repositories ([the loop](./concepts/the-loop.md)). `in_catalog` is `null`
because the instance names no catalog.

This instance has no records directory yet, so the blockers collector has no workspace to list and
records no blockers. [Initialise the records](./guides/initialise-the-records.md) writes one.

## When a program is missing

A program a collector would start must be on `PATH`. When one is missing, `start-snapshot` refuses
up front, names it, and takes no snapshot. With `aep` missing:

```console
$ conductor snapshot start-snapshot
conductor: snapshot start-snapshot: `aep` is not on PATH, and a snapshot's collectors starts it; `conductor doctor` lists every program conductor starts
$ echo $?
1
```

A collector that runs and does not answer fails the snapshot and names itself. The failed snapshot
is kept with its reason and is never read as the current state
([troubleshoot](./guides/troubleshoot.md#a-failed-snapshot)).

## Next

- [Configure an instance](./guides/configure-an-instance.md) for a real organization.
- [Initialise the records](./guides/initialise-the-records.md) with `conductor init`.
- Read the [security overview](/docs/security/overview), then
  [start conductor](./guides/start-conductor.md) and its controllers.
- [The CLI reference](/docs/reference/cli) for every command.
