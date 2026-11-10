---
title: The host
sidebar_position: 2
description: What runs on a conductor host. conductord reconciles each instance's runtime from the config file, at the placement the instance names, against one shared store; the loop, the host roles and the sessions beside them.
lede: One daemon, conductord, reads the config file and keeps one runtime per instance running at that instance's placement.
source: docs/design/architecture-next.md sections 2, 4, 5, 6 and 10; crates/conductor/src/daemon/mod.rs, daemon/reconcile.rs, daemon/upgrade.rs, daemon/host_roles.rs, daemon/api.rs, instance.rs, run.rs, placement.rs, confine/mod.rs, session.rs; conductor daemon --help, conductor instance --help
---

## What runs

| process | one per | started by | does |
|---|---|---|---|
| conductord, `conductor daemon serve` | control host | `conductor daemon serve --install`: the user unit `conductord.service` | reconciles the instance runtimes, serves the dashboard and `/v1/`, starts the host roles, upgrades instances |
| instance runtime, `conductor instance serve` | instance | conductord | serves the instance API |
| the loop, `conductor run` | instance | nothing in this release | the watch, cycle and daily ticks, with no model ([records and store](./records-and-store.md)) |
| supervisor, teacher | host | conductord, while `globals.roles` lists the role | the host roles ([roles](./roles.md#host-roles)) |
| maintainer | host | `conductor session start --role maintainer` | develops conductor |
| controller | repository | `conductor spawn` | one repository's work |

## conductord

conductord reads the config file that `--config` or `CONDUCTOR_CONFIG` names, else
`~/.b10x/conductor/conductor.yaml`. Its directory is the global `--state-dir`, else
`~/.b10x/conductor/daemon`.

A pass reconciles each instance of the file:

- it starts the instance's runtime when none runs;
- it restarts the runtime when it runs at another placement, or on another binary, than the
  config says;
- it rolls an upgrading instance whose runtime stopped back to its previous binary;
- it stops each runtime whose instance the file no longer names;
- it leaves a matching runtime alone and records nothing.

A pass runs at the first tick, when the file's text changes, when a runtime it started stops, and
every `daemon.cadence.reconcile` (60 s). Each instance action is one `Reconciliation` in the
daemon's store, under the tenant `daemon`. A start the service manager refuses is recorded
`Refused`, with what it said. `runtimes.json`, `binaries.json` and `boxes.json` in its directory
hold what it started, so a restarted daemon finds its runtimes and does nothing.

The daemon refuses to start on a file that writes no `daemon.store`, on a `daemon.store` whose
password is a `secret:<name>`, and on a `conductor.config/1` file, naming `conductor config
migrate` ([config](./config.md#migrate-a-conductorconfig1-file)).

It serves the dashboard app and `/v1/` on `127.0.0.1:<daemon.port>` (7300), and on the TLS
listener `daemon.tls` names. With `daemon.loopback: false` the loopback listener is off and
`--install` writes the system unit instead ([remote daemon](/docs/guides/remote-daemon)).

### Binaries

At an instance's first start the daemon copies its own binary to `binaries/<version>/` in its
directory, and the runtime runs that copy. An install over the daemon's path changes no runtime.
`conductor instance upgrade --instance <name> --tag v<x.y.z>` asks the daemon to fetch the release,
check it against `SHA256SUMS`, run its `conductor doctor` and restart the runtime on it. The daemon
rolls back to the previous binary unless the runtime answers within `daemon.upgrade_timeout`
(120 s). `--all` upgrades every instance one at a time and stops at the first rollback
([upgrade and roll back](/docs/guides/upgrade-and-rollback)).

## The instance runtime

`conductor instance serve` serves the dashboard's routes under `/v1/` on `<state>/api.sock`, mode
0600. Where that path is too long for a socket address, the socket is
`conductor-api-<hash>.sock` in the temporary directory. A write takes the token the daemon minted
for the instance at its start, in `CONDUCTOR_API_TOKEN`. conductord proxies
`/v1/instances/<name>/<route>` to it.

The runtime also runs the instance's loop, in its main thread, under the loop's lock.

## The loop

The loop runs once per instance, holding `<state>/run/lock`, with no model call. Every
`cadence.watch` it runs the watch pass and reads the mailbox. Every `cadence.cycle` it takes a
snapshot, publishes the board, runs `records check` and dispatches. Once a day at `cadence.daily`
UTC it runs `metric compute`, writes `REPORT.md` and commits the records. `--once` runs one watch
tick and one cycle tick and exits.

The runtime conductord starts runs the loop. `conductor run` runs the same loop by hand; while
the runtime holds the lock it is refused, and a runtime that finds the lock held yields until it
is released ([run conductor](../guides/start-conductor.md)).

## Placement

Every instance writes `placement`: `{daemon, adapter}`, or a bare string read as
`{daemon: local, adapter: <string>}`. `daemon` is `local` or an `https://` URL. `adapter` is `systemd`, `linux`,
`oci` or `pod`, and a local daemon refuses `oci` and `pod`.

Conductor implements no sandbox. Every adapter goes through `substrate-daemon`: one substrate box
per instance, the runtime a PTY session in it. `systemd` is `linux` inside a delegated systemd
scope. Before anything starts, the box's required facts are checked against what the substrate
daemon publishes. A miss is refused with substrate's name, `confinement.family-unserved` or
`confinement.fact-unserved`. Nothing falls back.

| need | how |
|---|---|
| the state directory, the records, the sqlite store's directory | the operator declares `substrate-daemon --writable-root state-<instance>=<state>`, `records-<instance>=<records>` and `store=<directory>`, one root where one holds the others; without one the box is refused `exec.writable-roots-host-unserved` |
| the runtime's output | `<state>/sessions/runtime.log` |
| the model token | a slot of `daemon.substrate_slots`, which the daemon fills from the instance's `model_token` before the box opens |
| a remote daemon | `placement` also writes `bearer: secret:<name>`, `trust_roots` and `server_identity`; one `conductor instance relay` pipe session carries each API request; an unanswering daemon shows the instance `Unreachable` |

A bare string that names no adapter keeps the runtime in the systemd user unit
`conductor_<instance>-runtime.service`. Sessions run in no box: substrate 0.20.0 starts no agent
session in one, so every session starts unboxed through `claude --bg`
([sessions](/docs/security/sessions)).

## Shared state

The store is one database per deployment, the URL in `daemon.store`, which the daemon hands to
every runtime ([records and store](./records-and-store.md)). The instances of one config file share
the host directory, `host/` beside the file: the disk ledger and its locks ([disk](./disk.md)).
