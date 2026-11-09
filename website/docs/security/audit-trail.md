---
title: Audit trail
sidebar_position: 5
description: What conductor records about its sessions and its operators, what it does not, how to read it back, and how a stored record changes.
lede: Conductor records denials, decisions, dispatches, messages, snapshots and the operator of every write; the full trail of what a session did is the harness's own transcript.
source: crates/conductor/src/guard.rs, crates/conductor/src/store.rs, crates/conductor/src/decide.rs, crates/conductor/src/actor.rs, crates/conductor/src/broker.rs, crates/conductor/src/daemon/reconcile.rs, crates/conductor/src/dashboard.rs; docs/design/architecture-next.md section 8
---

## What is recorded

One database per deployment holds every instance's records, each instance in its own tenant:
`daemon.store`, `sqlite:<path>` or `postgres://…` ([records and store](/docs/concepts/records-and-store)).
A `--state-dir` of its own opens `<state>/store.sqlite`.

| record | what it says | read it with |
|---|---|---|
| guard denials | repository, session id, tool, target (file, recipient or command), verdict, reason, time | `conductor guard guard-decisions` |
| decisions and decision requests | who decided what, its class, its actor and `recorded_by`, what it answers, what reversed it | `conductor decision decisions`, `decision show --decision-id <id>`, `decision requests`, `decision hands-todo` |
| dispatches | which repository was sent which work, and what it reported | `conductor dispatch dispatches` |
| messages | messages conductor received, handled or rejected | `conductor message messages` |
| controllers | each repository's controller record and its state | `conductor controller controllers` |
| leases, deliveries, run ticks | the conductor lease, texts delivered to sessions, each tick of `conductor run` | `conductor session leases`, `session deliveries`, `session run-ticks` |
| snapshots | what each pass saw: repositories (dirty files, worktrees, how far behind `main`), pull requests, workflow runs, sessions, releases | `conductor snapshot snapshots` and the other `snapshot` views |
| tool calls | every call through the instance's tool broker | no view yet |
| reconciliations | each start, restart, stop or refusal of a runtime, in the daemon's tenant | no view yet |

Every command records its actor: a person, an agent or the daemon
([actors](/docs/security/operators-and-credentials#actors)). A write through the daemon is
recorded as the operator it admitted, the Unix user on loopback or the bearer's principal on the
TLS listener. The token pool records owners, seats and states, and the daemon's secrets records
names and sources. Neither holds a value.

Conductor also keeps logs in its records directory, a repository it commits: `decisions/`,
`dispatches/`, `charters/` and `docs/handoff/`. A controller cannot write them by a file tool or by
a Bash write form that names them. Each snapshot counts the dirty files of every checkout, so a
Bash write the guard did not catch shows up there.

## What is not recorded

- **Allowed tool calls.** The guard records only denials and answers an allowed call without
  opening the store, since every tool call of every guarded session runs the hook.
- **What a session did.** The store records outcomes conductor was told about, not the commands a
  session ran. The full trail is Claude Code's session transcripts, in its own configuration
  directory.
- **Denials with no store.** Without `--state-dir` and without a config file that names a `state`
  directory, the guard still denies, and says on standard error that the verdict was not
  recorded.
- **Denials the store did not take in time.** Recording gets at most 2 seconds. When another
  writer holds the store's lock that long, the call is still denied and the recording dropped,
  with one line on standard error.
- **Unguarded sessions.** A session that started without a settings file that wires the guard,
  such as the maintainer's by default or one resumed by hand, never reaches it
  ([limits](/docs/security/limits#unguarded-sessions)).

## How to read it

```console
conductor guard guard-decisions                 # every recorded denial, as a table
conductor guard guard-decisions --format jsonl  # one JSON object per line
conductor decision decisions --format markdown
conductor decision show --decision-id <id>
```

Every list view takes `--format text|json|jsonl|markdown`. `decision show` takes `text` or `json`
([CLI reference](/docs/reference/cli)). A view opens only an existing store and creates nothing.
It lists every row it can read, names each record it cannot on standard error, and then exits 1.

The dashboard, `http://127.0.0.1:<daemon.port>/?instance=<name>`, shows sessions, dispatches, the
decision and dispatch logs, CI, usage and disk. It does not show guard denials
([dashboard](/docs/guides/dashboard)). An answer sent from its Decisions page is recorded as the
operator's decision with `recorded_by` `dashboard`, and its request's `answered` line follows.
Find such answers with `conductor decision decisions --format json` by `recorded_by`
([dashboard writes](/docs/security/operators-and-credentials#dashboard-and-api-writes)).

## How a record changes

The store keeps each record as a stream of events in the deployment's database, through
Eventlog's `eventlog-sqlite` or `eventlog-postgres` adapter.

- A record changes by appending its new state to its stream. A read folds the stream back. A
  deletion is an event too.
- A write expects the stream where its handle last read it: a creation another writer made first
  is refused (`Exists`), an update on a stale read too (`Moved`).
- An append cut off before it commits commits nothing.

One exception: each snapshot's observations live in a tenant of their own,
`<instance>/observations/<snapshot>`, and the store drops the tenants of the snapshots beyond the
newest `retention.snapshots` (12 by default). The `Snapshot` records stay.

A store from before v1, eventlog's tree under `state/tree/`, is read once into the database by
`conductor store migrate`. Until then every command refuses and names that command.

Anyone who can write the database, or its `sqlite:` file as the session's user, can change or
delete every record. The default file is `~/.b10x/conductor/conductor.sqlite`. A controller's
file tools write only its own places. A Bash command that writes the file is caught only where it
names a path the guard protects ([limits](/docs/security/limits)).
