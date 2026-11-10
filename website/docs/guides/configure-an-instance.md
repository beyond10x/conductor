---
title: Configure an instance
sidebar_position: 1
description: Write a conductor.config/2 file, validate it, read its effective values and the level each came from, change one key with config put, and migrate a conductor.config/1 file.
lede: Every command reads the config file before it runs, and the daemon reads only conductor.config/2, so write the file, validate it and read its effective values before you start the daemon.
source: docs/config.md; crates/conductor/src/config.rs, config/convert.rs, config/effective.rs, config/put.rs; crates/conductor/src/daemon/reconcile.rs; crates/conductor/tests/config.rs, config_globals.rs; conductor config --help
---

## 1. Write the file

The file is `--config <FILE>`, else `$CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml`.
This one, `~/example-org/conductor.yaml`, holds one instance that reads a GitHub owner's
repositories, checked out under `~/example-org`:

```yaml
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/store/conductor.sqlite
globals:
  cadence: {cycle: 2h}
default: personal
instances:
  - name: personal
    placement: linux
    session_prefix: personal
    sources:
      - github: example-org
    checkouts:
      root: ~/example-org
      trees: ~/.local/state/worktree/trees/example-org
    records: ~/records/example-org
```

| key | what it sets |
|---|---|
| `daemon.store` | the deployment's one database, `sqlite:<path>` or `postgres://…`. The daemon starts only on a `daemon.store` the file writes. A box writes the whole directory of a sqlite store, so the file is in a directory of its own, apart from the config file and `~/.b10x/conductor/daemon`; `config validate` names a box that would be refused (`placement.writable-root-shared`). Write the path absolute or under `~/`: a box is refused a store without one (`placement.store-not-absolute`) |
| `globals` | the defaults every instance starts from. An instance key overrides the global one, and a role's key overrides both |
| `placement` | where the instance's runtime runs: `linux` or `systemd` opens a box through the local `substrate-daemon`, and a word that names no adapter runs it as a user unit with no box ([the host](../concepts/the-host.md)) |
| `session_prefix` | the prefix of every session name the instance starts, such as `personal-conductor`. Required |
| `sources` | where the repositories come from: `github: <owner>`, `local: <path>` or `gitlab: <group>` ([GitLab](./gitlab.md)) |
| `checkouts.root`, `checkouts.trees` | where their checkouts and managed worktrees are |
| `records` | the directory conductor keeps its records in. Default `~/.b10x/conductor/<name>/records`. With a box placement, keep it outside `checkouts.root`: a box reads at most four directories, and a checkouts root that holds the records is read entry by entry |

With no `roles`, the instance takes the built-in roles on `claude` and `opus`. A `roles` list
replaces them. The old role names `conductor`, `conductor-dev` and `controller` are read as
`director`, `maintainer` and `lead` for this release, with a note. Who reads each key is in
[config](../concepts/config.md), every type and default in the
[config reference](../reference/config.md).

## 2. Validate it

```console
$ conductor config validate
~/example-org/conductor.yaml: valid, 1 instance(s): personal
```

Each problem is printed with its YAML path, and the command exits 1:

```yaml
version: conductor.config/2
instances:
  - name: demo
    placement: linux
    session_prefix: demo
    sources:
      - local: ~/work
    checkouts:
      root: ~/work
      trees: ~/.local/state/worktree/trees/work
    cadence:
      cycle: 90
    thresholds:
      disk_low: 100GB
```

```console
$ conductor config validate --config broken.yaml
conductor: config validate: broken.yaml: instances[0].cadence.cycle: 90 is not a duration: write a whole number above 0 with s, m, h or d, such as 180s or 2h
conductor: config validate: broken.yaml: instances[0].thresholds.disk_low: "100GB" is not a size: write a whole number of GiB with G, such as 15G
$ echo $?
1
```

Every other command resolves the instance first and stops on the same problems.

## 3. Read the effective values

`config show` prints the instance after defaults, every path absolute (written `~` here):

```console
$ conductor config show --format json | jq -c '.instances[0] | {records, state, cache}'
{"records":"~/records/example-org","state":"~/.b10x/conductor/personal/state","cache":"~/.cache/b10x/conductor/personal"}
```

`--format yaml` prints it as a config file. `--effective` prints every key once with the level it
came from: `default`, `global`, `instance` or `role`. A role's key is written under
`roles.<role>.`:

```console
$ conductor config show --effective | grep -E '^(placement|cadence\.(cycle|ci)):'
placement: linux (instance)
cadence.cycle: 2h (global)
cadence.ci: 900s (default)
```

With `--format json` it prints the same as rows, keyed `<instance>/<role or ->/<path>`.

## 4. Change one key

`config put` changes one key at one level, validates the whole file, writes it, commits it to
the git checkout the file is in, and records the change in the store `daemon.store` names:

```console
$ conductor config put --level instance --instance personal --path thresholds.silent_session --value 45m --reason "controllers run long gates"
```

| flag | rule |
|---|---|
| `--level` | `global`, `instance` or `role`. A global change names no instance, an instance change `--instance`, a role change `--instance` and `--role`, else `LevelScopeMismatch` |
| `--path` | the key under that level, its words joined by `.` |
| `--value` | read as YAML. Without it the key is removed at that level, so the level below applies again |
| `--reason` | written into the commit body and the revision. Required |

It prints the revision id, the key, the level and the commit. The file is written whole, so its
comments are lost, and a change that would not validate writes nothing. An agent's change is
refused `ActorNotHuman` unless `globals.tuning.keys` allows the key and value. With
`--daemon <url>` the change goes to that daemon's `PUT /v1/config`
([remote daemon](./remote-daemon.md)).

## 5. Migrate a conductor.config/1 file

The daemon refuses a `/1` file:

```text
the config file ~/example-org/conductor.yaml is conductor.config/1, which the daemon does not read: run conductor config migrate to write conductor.config/2
```

The other commands still read a `/1` file for this release. `conductor config migrate` converts it
once and prints one note per change. `--check` prints the notes and writes nothing.

```console
$ conductor config migrate
version: conductor.config/2 (was conductor.config/1)
instances[0].dashboard_port: dropped (the daemon serves the one dashboard, at daemon.dashboard)
instances[0].confinement.limits: moved to instances[0].limits (the instance's box as a whole)
instances[0].placement: systemd
host.roles: moved to globals.roles (from ~/.b10x/conductor/host/host.yaml, which is no longer read)
daemon.store: not written; the daemon starts only on a daemon.store the file writes
personal: session_prefix personal
```

| `/1` key | after `migrate` |
|---|---|
| `version` | `conductor.config/2` |
| `dashboard_port`, `store`, `confinement.mode`, `confinement.provider` | dropped |
| `confinement.limits` | the instance's `limits` |
| `placement` | written after `name`: `systemd`, or `none` for a `none` provider |
| `~/.b10x/conductor/host/host.yaml` roles | `globals.roles`, each without its `session_name` |
| `session_prefix` | written into each instance without one: its name in letters, digits and `-`, with `2`, `3`, … appended when another instance holds it |

The old file is kept as `<file>.config-1.bak`, and comments are lost. A converted file that would
not validate is not written. Then write `daemon.store` by hand: no command writes it. A new prefix
renames the instance's sessions and slice, so stop them first with
`conductor session stop --role <role>`.

Next: [initialise the records](./initialise-the-records.md).
