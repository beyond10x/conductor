---
title: Upgrade and roll back
sidebar_position: 7
description: Replace the conductor binary on a host, restart the daemon on it, move each instance's runtime to a release with conductor instance upgrade, and read how the daemon rolls an upgrade back.
lede: Each instance runtime runs a copy of the binary it was first started on, so installing a new conductor changes the daemon and the commands, and conductor instance upgrade moves a runtime.
source: crates/conductor/src/daemon/upgrade.rs, daemon/reconcile.rs, daemon/api.rs, daemon/install.rs; crates/conductor/src/upgrade.rs, instance.rs, placement.rs; conductor instance upgrade --help, conductor upgrade --help
---

## The binaries on a host

| binary | who runs it | changed by |
|---|---|---|
| `conductor` on `PATH` | every command you type, and conductord, which keeps the binary it started with until it restarts | `conductor upgrade`, or a new release archive ([getting started](../getting-started.md#1-install)) |
| `~/.b10x/conductor/daemon/binaries/<version>/conductor` | each instance runtime, `conductor instance serve` | the daemon: a copy of its own binary at an instance's first start, then each upgrade |

The daemon keeps each instance's binary, and the one it ran before its last upgrade, in
`binaries.json` of its directory, `~/.b10x/conductor/daemon` unless `--state-dir` names another.
A runtime stays on its copy until an upgrade moves it, so installing over the `conductor` on
`PATH` moves no runtime. An instance removed from the config file loses its entry, and starts on a
fresh copy of the daemon's binary when it comes back.

## 1. Install a new binary on this host

```console
$ conductor upgrade --check
$ conductor upgrade
$ conductor upgrade --release 0.25.0
```

`conductor upgrade` reads the releases of the repository the binary was built from, downloads the
archive for this target and `SHA256SUMS`, checks the sum, runs the new binary's `--version`, and
replaces this binary. The old file is kept beside it as `conductor.previous`. `--check` only
compares versions, and `--release` installs a named release, older or newer. A refusal changes
no file and exits 1. The binary carries the role profiles and the `claude-code` settings, so
nothing else is copied. A binary built from source is installed as
[developer onboarding](../develop/onboarding.md#build-from-source) says.

Then restart the daemon on the new binary:

```console
$ systemctl --user restart conductord.service
$ conductor doctor | grep '^daemon unit'
daemon unit: conductord.service running
```

## 2. Upgrade an instance

```console
$ conductor instance upgrade --instance personal --tag v0.26.0
personal: upgraded to 0.26.0
$ conductor instance upgrade --all --tag v0.26.0
```

`--tag` is a release `v<x.y.z>`, older or newer than the one the runtime runs. `--all` takes every
instance of the config file in its order, one at a time, and stops at the first rollback. Give
`--instance` or `--all`, not both. The command asks the daemon of the config file on
`127.0.0.1:<daemon.port>`, with the token the daemon keeps in `api.token` of its directory, mode
0600. With `--daemon <url>` it asks that daemon as your login, with the `upgrade-instances` grant
([remote daemon](./remote-daemon.md)).

The daemon takes one running instance through four steps, each recorded as one `Reconciliation`
of action `Upgrade` in its store:

| step | does |
|---|---|
| `fetch` | downloads the release's archive for this host's target, checks it against `SHA256SUMS` and unpacks it |
| `install` | copies the binary to `binaries/<version>/conductor` in the daemon's directory |
| `doctor` | runs the new binary's `conductor doctor` with the runtime's environment and directory. A failure ends the upgrade, and the runtime runs on, untouched |
| `restart` | stops the runtime and starts it on the new binary. The binary it ran is kept as its previous one |

The instance is then `Upgrading` in `GET /v1/instances` until its runtime answers on `api.sock`.
The command waits up to `daemon.upgrade_timeout` and a minute, and prints one line per instance.

## 3. Rollback

The daemon puts the previous binary back and starts the runtime on it when the upgraded runtime:

- does not answer within `daemon.upgrade_timeout`, 120 s by default, or
- stops running.

The upgrade is then `RolledBack`, and the rollback is recorded as the daemon's own, the upgrade as
the operator's. The command exits 1:

```text
instance upgrade: personal: rolled back to 0.25.0: 0.26.0 did not come up
```

A failed `doctor` step exits 1 as `rolled back:` with the step's detail, and the runtime stays on
its binary. An instance that is not running, or is upgrading already, is refused:

```text
instance upgrade: personal: refused: InstanceStateConflict: personal is not running, or is upgrading already; only a running instance is upgraded
```

To go back after a successful upgrade, upgrade to the older tag:

```console
$ conductor instance upgrade --instance personal --tag v0.25.0
```

To raise the bound for a slow host, write it in the file's `daemon` section. `config put` writes
`globals` and the instances only.

```yaml
daemon:
  upgrade_timeout: 300s
```

## Runtimes in a remote box

The `install` step writes the copy on the daemon's host, which a runtime of a local placement
runs. The daemon reaches the instance API of a runtime in a box on a remote substrate daemon
through `conductor instance relay`: one pipe session per request in the box, which bridges
standard input to the runtime's socket (`CONDUCTOR_API_SOCKET`, else `<state>/api.sock`) and its
answer to standard output ([remote daemon](./remote-daemon.md#5-place-an-instance-on-a-remote-substrate-daemon)).
