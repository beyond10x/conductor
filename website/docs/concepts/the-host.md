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

Both listeners answer three paths to `GET` without a bearer, for a cluster's probes and a
Prometheus. The loopback listener still refuses a foreign `Host`.

| Path | Answer |
|---|---|
| `/healthz` | `200 {"state":"serving"}` while the process serves, whatever the store does |
| `/readyz` | `200 [{"reachable":true,"reason":null}]` once the config is read, the store answered a read within the last 10 s, the reconciler published once, and its last record did not fail; otherwise `503`, `reachable` false and a one-line `reason` |
| `/metrics` | `200`, Prometheus text format |

A daemon whose store does not answer keeps serving: `/healthz` is `200` and `/readyz` is `503`. It
starts no runtime and records nothing until the store answers, then takes up its work without a
restart. Until the reconciler has published once, `GET /v1/instances`, the federated reads, a
proxied instance route, `POST /v1/ask` and `POST /v1/roles/{role}/start` answer `503` with the
readiness reason. A record the store refuses is held in the daemon's memory and written when the
store takes writes again; until then the reconciler acts on nothing, an upgrade asked through
`POST /v1/instances/{name}/upgrade` answers `503` with the readiness reason and does nothing, and
`/readyz` is `503`. The pass that failed on the record then runs again for its own reason, or for
the config change when the file changed since. A daemon stopped meanwhile loses the held records
but not what it did: the failed pass wrote `runtimes.json`, so the next daemon starts no runtime
that runs and stops one whose instance left the file; the store misses those records.

`/metrics` holds five gauges:

| Metric | Value |
|---|---|
| `conductor_instances{state}` | instances by the state the daemon last saw, every state named |
| `conductor_sessions{instance,state}` | sessions by instance and state |
| `conductor_snapshot_age_seconds{instance}` | age of the instance's newest complete snapshot |
| `conductor_build_queue_length` | disk requests waiting in the host's build ledger |
| `conductor_instances_unanswered` | instances the answer holds no sessions and no snapshot age of |

The daemon asks every instance for its sessions and its snapshot at the scrape, all at once,
within 3 s (`daemon.timeout` when that is shorter), and keeps one answer for 5 s. A value it could
not read has no line, never `0`. No body of the three paths carries a name, a path, a token or the
store URL, so `instance` is the instance's position in the config file, `1` for the first.

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
| the state directory, the records, the cache, the sqlite store's directory | the operator declares `substrate-daemon --writable-root state-<instance>=<state>`, `records-<instance>=<records>`, `cache-<instance>=<cache>` and `store=<directory>`, one root where one holds the others; without one the box is refused `exec.writable-roots-host-unserved` |
| each of those directories there | the four roots exist before the box opens: conductord creates the state directory and the store's directory at its start, and nothing else; `conductor doctor` names one that is missing, `placement.writable-root-missing` |
| the substrate daemon | `substrate-daemon`, at least the version `conductor doctor` pins (0.20.0), kept running by the operator and answering on `$XDG_RUNTIME_DIR/substrate/daemon.sock`, or on the socket `CONDUCTOR_SUBSTRATE_SOCKET` names; one that does not answer is `placement.substrate-unreachable` |
| a delegated cgroup | the substrate daemon runs inside a delegated cgroup subtree, its `--cgroup-root`, carrying `cpu`, `memory` and `pids`: the `systemd` adapter requires the `limits` family and whole-tree kill, which the daemon publishes only there |
| nothing else written | a root is a whole directory: a box whose state, records, cache or store directory holds the config file or lies with `~/.b10x/conductor/daemon` is refused `placement.writable-root-shared` before substrate is asked, so a sqlite store is in a directory of its own |
| the runtime's output | `<state>/sessions/runtime.log` |
| the model token | a slot of `daemon.substrate_slots`, which the daemon fills from the instance's `model_token` before the box opens |
| a remote daemon | `placement` also writes `bearer: secret:<name>`, `trust_roots` and `server_identity`; one `conductor instance relay` pipe session carries each API request; an unanswering daemon shows the instance `Unreachable` |

For a config file that writes `daemon.store`, the only kind conductord starts on, `conductor doctor` dry-runs the box of each instance on the local daemon: it builds the request conductord would send, checks conductord's own refusals and that each writable root is a directory, and asks the substrate daemon for its facts, once for all boxes, as conductord does before anything starts. It prints one `box <instance>:` row, starts no box and writes nothing. A box that would be refused makes the verdict `not ready: box <instance> (<refusal>)`. A substrate daemon that does not answer within 5 s is one such refusal, `placement.substrate-unreachable`; the row names its socket and the 5 s. A box on a remote daemon is checked up to conductord's own refusals: its writable roots are directories of the remote daemon's host, so doctor does not look for them on this one, does not ask that daemon, and the row says so.

Three numbers decide what the daemon does with a runtime it finds stopped: 10 s, 60 s and 5 minutes.

- A runtime that exits within 10 s of its start is a failed start. The daemon looks every 2 s, so it is one found stopped at a look less than 12 s after its start. The daemon records it as a `Start` that was `Refused`, with the last lines the runtime wrote to `runtime.log` since that start.
- A runtime that stopped later and before it ran 60 s is recorded the same way, as found stopped and not known to have run the 60 s. The daemon looks every 2 s, so it is one found stopped at a look less than 62 s after its start.
- Both count in the row of failed starts. The next start waits 10 s after the first of the row and twice as long after each further one, up to 5 minutes. A change to the config file starts it at the next pass.
- A runtime that ran 60 s or longer ends the row: one found stopped at a look 62 s or more after its start. It is started again at once.

`/v1/instances` shows the instance `Unreachable` while it waits for its next start, and after a start that follows a failed one until a look 12 s after it finds the runtime running.

The daemon reads `runtime.log` only when it is a regular file: it follows no link and waits for no pipe, since the box can write that directory. Otherwise the record says the log is no regular file and carries none of it. Control characters and escape sequences of the log are replaced in the record.

A bare string that names no adapter keeps the runtime in the systemd user unit
`conductor_<instance>-runtime.service`. Sessions run in no box: substrate 0.20.0 starts no agent
session in one, so every session starts unboxed through `claude --bg`
([sessions](/docs/security/sessions)).

## Shared state

The store is one database per deployment, the URL in `daemon.store`, which the daemon hands to
every runtime ([records and store](./records-and-store.md)). The instances of one config file share
the host directory, `host/` beside the file: the disk ledger and its locks ([disk](./disk.md)).
