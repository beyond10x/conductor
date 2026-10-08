---
title: Getting started
sidebar_position: 2
description: Build the conductor CLI, write a one-instance config, validate it and take one snapshot of two local repositories.
---

This page builds the `conductor` CLI, writes the smallest config, checks it, and takes one snapshot
of two tiny local repositories. It starts no session; [Start conductor](./guides/start-conductor.md)
does that.

Every command below was run as shown. The CLI prints absolute paths; the output here writes the
home directory as `~`.

## Install

You need a Rust toolchain (edition 2024) with `cargo`, and `git`. Taking a snapshot also runs `aep`
and `claude` (Claude Code), `gh` for a `github` source and `ess` for repositories with a
specification; the [README](https://github.com/beyond10x/conductor#install) lists every tool.

Install from the repository:

```console
cargo install --git https://github.com/beyond10x/conductor --locked conductor-cli
```

or, from a checkout, `task install`, which runs `cargo install --path crates/conductor --locked`.
Either installs one binary, `conductor`.

```console
$ conductor --help
The records conductor and the repository controllers keep

Usage: conductor [OPTIONS] <COMMAND>
...
```

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

Write `~/.b10x/conductor/conductor.yaml`. An instance needs a name, its sources and its checkouts;
every other key has a default. The repository's `conductor.example.yaml` has two complete instances
to copy from.

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
$ conductor config show | head -9
version: conductor.config/1
default: demo
instances[0].name: demo
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

A collector that cannot run fails the snapshot and names itself. With `aep` missing from `PATH`:

```console
$ conductor snapshot start-snapshot
01a11c9d-8a1c-7341-be7d-eb6d9b53578a
conductor: snapshot start-snapshot: snapshot 01a11c9d-8a1c-7341-be7d-eb6d9b53578a failed: blockers: list the members of ~/.b10x/conductor/demo/records/.engineering/workspace.yaml, tried twice: `aep plan workspace members --format json --root ~/.b10x/conductor/demo/records`: start `aep`: No such file or directory (os error 2)
```

The failed snapshot is kept with its reason and is never read as the current state.

## Next

- [Configure an instance](./guides/configure-an-instance.md) for a real organization.
- [Start conductor](./guides/start-conductor.md) and its controllers.
- [The CLI reference](/docs/reference/cli) for every command.
