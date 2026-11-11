---
title: Troubleshoot
sidebar_position: 8
description: Read conductor doctor's verdict, why the daemon does not start or a runtime is refused, why a snapshot failed, what a guard denial means, and a fix on macOS.
lede: Conductor stops with a message that says what failed, so start from the message and the command that printed it.
source: crates/conductor/src/doctor.rs; crates/conductor/src/daemon/mod.rs, daemon/reconcile.rs; crates/conductor/src/placement.rs, secret.rs; crates/conductor/src/snapshot.rs, collect/; crates/conductor/src/guard.rs; crates/conductor/src/run.rs; crates/conductor/src/session.rs; spec/domains/session.yaml
---

## Start with doctor

```console
conductor --version
conductor config validate
conductor doctor
```

Quote `conductor --version` when you report a problem. Every command stops on a config problem,
printed with its YAML path. `conductor doctor` writes nothing, and its full output is in
[getting started](../getting-started.md). Its last line is the verdict, and any `not ready:`
exits 1:

| reason in `not ready:` | fix |
|---|---|
| `config (unreadable)` | the `config:` lines above say why |
| `<program> (missing)` | run the `install:` line under its row |
| `<program> (too old)` | upgrade it to the pin of the nearest `b10x.toml`, else the build's pin |
| `<program> (version unread)` | its `--version` prints no version doctor can read |
| `<program> (pin unread)` | correct its pin in `b10x.toml` to `x.y.z` or `x.y` |
| `<program> (not logged in to <host>)` | `gh auth login --hostname github.com`, or `glab auth login --hostname <host>` |
| `worktree workspace (not activated: <root>)` | run the `activate:` line under the root's row, `conductor setup --apply` for a checkout under it, which writes the profile and activates the root |
| `agent kind <kind> (not implemented)` | set the role's `kind` to `claude-code`, the only kind |
| `forge route <instance> (no gates_policy)` | the instance is on `forge.route: gates`, the default, and names no `gates_policy`: write its `gates_policy`, or `forge: {route: direct}` |
| `forge route <instance> (no bot route for <host>)` | the bot route writes to GitHub only: write `forge: {route: direct}` on the instance whose `gitlab` source is on `<host>` |
| `guard <role> (denied)` | the guard denies that role the pull or merge request create it makes by design. Its `guard <role>:` line carries the guard's reason; on the route `gates` with a `gitlab` source, the instance's `forge route` line names the fix. With several instances in the file the role reads `<role> of <instance>` |

`substrate-daemon` is needed only for a `placement` that opens a box, a forge's CLI only for a
source on that forge. The `secrets:`, `store:` and `daemon unit:` lines change no verdict, nor
does a `guard <role>: not checked (…)` line: that role has no checkout whose `origin` names a
repository to ask the guard about.

`conductor instance upgrade` runs the new binary's doctor first, and rolls back when it is not
ready: an upgrade that prints `rolled back: doctor: … not ready: forge route <instance> (no
gates_policy); forge route <instance>: …` goes through once that instance writes its
`gates_policy` or `forge: {route: direct}`, the change the quoted `forge route` line names. A
policy only in `B10X_GATES_POLICY` stays not ready: with the variable set, the line names its
path as the `gates_policy` to write.

## The daemon does not start

Run it in the foreground to see why, or read its unit's log:

```console
conductor daemon serve
journalctl --user -u conductord.service
```

| message | fix |
|---|---|
| `the config file <file> is conductor.config/1, which the daemon does not read: run conductor config migrate to write conductor.config/2` | [migrate the file](./configure-an-instance.md#5-migrate-a-conductorconfig1-file) |
| `the config file <file> writes no daemon.store: write sqlite:<path> or postgres://…` | write `daemon.store`. No command writes it |
| `daemon.store writes its password as secret:<name>, which the daemon does not resolve yet` | use a `sqlite:` store in this release |
| `daemon serve: daemon.tls.cert: the TLS listener needs it` | write `cert`, `key` and `identity` beside `daemon.tls.listen` |
| `daemon serve: listen on 127.0.0.1:7300` | another process holds `daemon.port`: stop it, or set another port |

## A runtime is refused

The daemon prints one line per action, `<instance>: <action> (<reason>): <result>`, with the
detail of a `Refused` result. For a `placement` that opens a box, the detail is substrate's:

| detail | fix |
|---|---|
| `exec.writable-roots-host-unserved` | start `substrate-daemon` with the `--writable-root <name>=<path>` the detail prints, such as `state-<instance>=<state>` |
| `placement.writable-root-shared` | a directory the box writes holds the config file or lies with `~/.b10x/conductor/daemon`. For the store, write `daemon.store` as the detail prints it, a file in a directory of its own, and move the database there |
| `placement.store-not-absolute` | `daemon.store` names a sqlite file without an absolute path, such as `sqlite:conductor.sqlite`, and the instance runs in a box. Write an absolute path or one under `~/`, as the detail prints it. `conductor config validate` notes it |
| `placement.mounts-exceeded` | the records lie under a checkouts root whose other entries are more directories than one box reads (four, those of `conductor` and `claude` among them): name a `records` directory outside it |
| `confinement.family-unserved`, `confinement.fact-unserved` | the local `substrate-daemon` does not serve the adapter. `systemd` needs it in a delegated systemd scope |
| `placement.substrate-unreachable` | the remote substrate daemon did not answer within 10 s |
| `SecretMissing` | `conductor secret put --name <name>` for each name printed |

The loop runs once per instance. While the runtime holds the lock, `conductor run` refuses with
`run: another loop holds <state>/run/lock, the instance runtime's (conductor instance serve) or a
conductor run's; one runs per instance`.

## A refused session start

`conductor session start --role <role>` starts nothing when a check fails, and prints one line,
`session start: refused: <refusal>: role "<role>": <detail>`:

| refusal | fix |
|---|---|
| `UnknownRole` | the config names no such role, or the role does not run on `claude`: add the role, or set `harness: claude` |
| `InstanceNotPinned` | the role's own `settings` file is missing, is no JSON, or pins another instance: fix it, or remove `settings` |
| `SessionRunning` | a live session holds the name: attach to it with `conductor session attach --role <role>`, or stop it with `conductor session stop --role <role>` |
| `ProviderUnavailable`, `MeasurementShort` | under `strict` confinement, the provider is missing or the box is short of the request |
| `LaunchFailed` | the start or the first prompt failed; the detail names why |

## A failed snapshot

**Refused up front.** When a program a collector starts is not on `PATH`, `start-snapshot` takes
no snapshot and exits 1:

```console
$ conductor snapshot start-snapshot
conductor: snapshot start-snapshot: `aep` is not on PATH, and a snapshot's collectors start it; `conductor doctor` lists every program conductor starts
```

**Failed by a collector.** The command prints the id and
`conductor: snapshot start-snapshot: snapshot <id> failed: <reason>`, and exits 1. The newest
complete snapshot stays current. Read the reasons back:

```console
conductor snapshot snapshots --format json | jq -r '.[] | select(.state == "Failed") | .failure_reason'
```

| reason | fix |
|---|---|
| `blockers: list the members of …/workspace.yaml, tried twice` | `conductor config workspace --check`, then `conductor config workspace` |
| `repositories: …` or `github: …`, with a `gh` command | `gh auth status`, then take a new snapshot |
| `specifications: …` | `conductor doctor`, then the command the reason prints. A specification `ess` refuses is recorded as `Refused`, not a failure |

A member whose fetch or export fails twice is a member failure, not a failed snapshot.
`conductor snapshot member-failures` and the Health view list them: make its `origin` reachable
with `main` pushed, or rewrite the workspace file. A member whose checkout is absent is skipped,
with `member <name>: checkout absent, skipped` on standard error.

## Guard denials

```console
conductor guard guard-decisions --format json
```

The guard records denials only, in the store `daemon.store` names. The session sees the reason
and can take another route. When every tool call of a session is blocked, the hook could not run
`conductor`: put it on the session's `PATH` ([getting started](../getting-started.md#1-install)). An empty list does not show that a session ran under
the guard ([the guard](../security/the-guard.md), [sessions](../security/sessions.md)).

## On macOS

| message | fix |
|---|---|
| `ess generate synthesize` refuses files that carry `com.apple.provenance` | macOS marks files a process Claude Code started wrote, and an `ess` older than 0.54.0 refuses them. Check `command -v ess` and `ess --version` in that shell, install the `ess` your `b10x.toml` pins, and remove the older copy from `PATH`. `xattr -l <file>` lists the attribute |
