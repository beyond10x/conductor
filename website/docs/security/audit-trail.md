---
title: Audit trail
sidebar_position: 4
description: What conductor records about its sessions, what it does not, how to read it back, and why a stored record cannot be changed in place.
lede: Conductor records denials, decisions, dispatches, messages and snapshots; the full trail of what a session did is the harness's own transcript.
source: crates/conductor/src/guard.rs; crates/conductor/src/store.rs; crates/conductor/src/decide.rs; crates/conductor/src/dashboard.rs; docs/design/conductor.md section 10; eventlog-tree (crates/conductor/Cargo.toml)
---

## What is recorded

Conductor's store lives in the instance's `state` directory (or `--state-dir`). It holds:

| record | what it says | read it with |
|---|---|---|
| guard denials | repository, session id, tool, target (file, recipient or command), verdict, reason, time | `conductor guard guard-decisions` |
| decisions and decision requests | who decided what, its class, what it answers, what reversed it | `conductor decision decisions`, `decision show --decision-id <id>`, `decision requests`, `decision hands-todo` |
| dispatches | which repository was sent which work, and what it reported | `conductor dispatch dispatches` |
| messages | messages conductor received, handled or rejected | `conductor message messages` |
| controllers | the controller record of each repository and its state | `conductor controller controllers` |
| snapshots | what each pass over the instance saw: repositories (dirty files, worktrees, how far behind `main`), pull requests, workflow runs, sessions, releases | `conductor snapshot snapshots` and the other `snapshot` views |

Conductor also keeps readable logs in its records directory, a repository it commits:
`decisions/`, `dispatches/`, `charters/` and `docs/handoff/`. A controller cannot write them by a
file tool or by a Bash write form that names them.

Each snapshot counts the dirty files of every checkout. A Bash write the guard did not catch
shows up there as a finding.

## What is not recorded

- **Allowed tool calls.** The guard records only denials. An allowed call is answered without
  opening the store, because every tool call of every guarded session runs the hook.
- **What a session did.** The store records outcomes conductor was told about, not the commands a
  session ran. The full trail is the harness's session transcripts, which Claude Code keeps in its
  own configuration directory for each session.
- **Denials when no store is named.** Without `--state-dir` and without a config file that names
  a `state` directory, the guard still denies, and says on standard error that the verdict was not
  recorded.
- **Denials the store did not take in time.** Recording gets at most 2 seconds. When another
  writer holds the store's lock that long, the call is still denied and the recording is dropped,
  with one line on standard error.
- **Unguarded sessions.** A session started without the instance's settings file, such as
  conductor-dev by default or a session resumed by hand, never reaches the guard.

## How to read it

```console
conductor guard guard-decisions                 # every recorded denial, as a table
conductor guard guard-decisions --format jsonl  # one JSON object per line
conductor decision decisions --format markdown
conductor decision show --decision-id <id>
```

Every list view takes `--format text|json|jsonl|markdown`; `decision show` takes `text` or `json` ([CLI reference](/docs/reference/cli)). A view
opens only an existing store and creates nothing. It lists every row it can read; a record it
cannot read is named on standard error, and the view exits 1.

`conductor dashboard serve` shows the sessions, their dispatches, the decision and dispatch logs,
CI, usage and disk on a page that reloads itself. It listens on `127.0.0.1` only (port 7313 by
default), answers only `GET`, refuses a request naming another `Host`, and writes nothing. It does
not show guard denials; use `guard guard-decisions` for those.

## Records cannot be changed in place

The store is an eventlog tree store: a directory of files, one file per event, each written once
and named by the digest of its content.

- A record changes by a new event, never by rewriting a file. A deletion is an event too.
- A file whose content no longer matches its digest is refused when the store opens: an edited
  record does not read back silently.
- One writer at a time holds the store's lock.
- An append cut off before its commit file was written is never read.

One exception, by design: when a snapshot completes, the store keeps the observations of the
newest `retention.snapshots` complete snapshots and drops the others' observation files. The
`Snapshot` records themselves stay.

Anyone who can write the state directory as the session's user can still delete or replace the
whole store. For a controller the guard denies a file tool there, and a Bash write form that names
it; a command that reaches it another way is not caught ([limits](/docs/security/limits)).
