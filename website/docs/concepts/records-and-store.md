---
title: Records and store
sidebar_position: 5
description: The records directory, a git repository of logs and charters a person reads, and the store, one SQLite or Postgres database per deployment that every command writes; how a v0 state directory moves into the database with conductor store migrate.
lede: The store is the authoritative copy of what happened; the records directory is a readable projection of it, committed once a day.
source: docs/design/architecture-next.md section 8; crates/conductor/src/store.rs, store/migrate.rs, records.rs, init.rs, run/records.rs, state.rs, doctor.rs; conductor store migrate --help, conductor records check --help
---

## The records directory

Each instance's `records`, by default `~/.b10x/conductor/<name>/records`, is a git repository of its
own. `conductor init` writes its skeleton and commits only the files it wrote. It overwrites
nothing, and writes nothing when any of them is there ([initialise the
records](/docs/guides/initialise-the-records)).

| path | holds | written by |
|---|---|---|
| `NORTHSTAR.md` | what the organization works towards | conductor, with the operator |
| `rules.md` | the organization's rules, sections Director, Lead and Maintainer, each rule with its source | the maintainer |
| `decisions/YYYY-MM.jsonl` | the decision log | the commands that record a decision event |
| `dispatches/YYYY-MM.jsonl` | dispatches, reports, messages, deliveries, resource answers, charter revisions | the commands that record a dispatch event |
| `charters/<repo>.md` | one charter per controller | conductor |
| `docs/handoff/` | hand-overs; conductor starts from the one committed last | conductor |
| `goals.jsonl`, `STATUS.md`, `REPORT.md` | the goals, the newest board, the daily brief | conductor |
| `.engineering/workspace.yaml` | the workspace the blockers collector reads | `conductor config workspace` |

A log line is appended once the store has kept the record: one JSON object with `at`, `event` and
`repo`, then the fields its event declares, in the file of the line's UTC month. A line that cannot
be appended fails the command, after the store kept the record. Without a config file a line is
appended only where `conductor init` made the log's directory.

`conductor records check` reads both logs and prints one finding per line, `<file>:<line>:
<finding>: <detail>`, and exits 1 when it finds any:

| finding | means |
|---|---|
| `DecisionInDispatchLog` | a decision event in `dispatches/` |
| `UnknownKeySet` | a line that is no JSON object, misses a key its log holds, or holds one it does not declare |
| `UnknownDecision` | a `DEC-…` id cited in `dispatches/` that no decision holds |
| `RawThreshold` | a size in a charter that a threshold key holds |
| `RoleFileRules` | a role file over 20 rules, or with a rule of more than one line |

The daily tick of `conductor run` commits `decisions`, `dispatches`, `charters`, `docs/handoff`,
`REPORT.md` and `STATUS.md` after `records check` passes. The message is `Conductor: records of
<day>: <n> dispatch lines, <m> decision lines`. Under `forge.route: gates` it commits and pushes
through `b10x-gates bot`, under `direct` with plain git. `conductor run --once --commit-records`
runs that step alone.

## The store

The store is one database per deployment, opened through Eventlog with one of two adapters:
`sqlite:<path>` for a deployment on one host, `postgres://…` for every other. `daemon.store`
names it, by default `sqlite:~/.b10x/conductor/conductor.sqlite`. The daemon hands the URL to every
runtime it starts in `CONDUCTOR_STORE`.

| a command on | opens |
|---|---|
| its instance's own `state` | `CONDUCTOR_STORE`, else the `daemon.store` the file writes, else `<state>/store.sqlite` |
| any other `--state-dir` | `<state>/store.sqlite` |

Every event carries its instance. On the shared database an instance's records are the tenant of
its name. Each snapshot's observations are the tenant `<instance>/observations/<snapshot>`. The
daemon's own records are the tenant `daemon`. The store keeps the observations of the newest
`retention.snapshots` complete snapshots (12). A write expects the stream where the handle last
read it. A creation decided on a stale read is refused (`Exists`), and so is an update (`Moved`).

A view opens only an existing store and creates nothing. A command that writes the instance's
records refuses a `--state-dir` other than the instance's `state` (`StateDirNotInstance`).
`conductor doctor` prints `store: <backend> <where>` and whether it answers.

The daemon refuses a `daemon.store` whose Postgres password is written `secret:<name>`: no secrets
source resolves it yet.

## `conductor store migrate`

Before v1 the store was Eventlog's tree under the state directory: `tree/`, `observations/` and
`series/`. `conductor store migrate` reads that tree once into the database:

1. An older state directory is first brought to the tree: an Eventlog file store at `store/` is
   moved into `tree/` and `observations/` and kept as `store.eventlog-file-<UTC stamp>/`, and a
   `tree/` that holds observations is split, the old one kept as `tree.split-<UTC stamp>/`.
2. Every event is appended to its stream in feed order, at the version it held in the tree.
3. Each stream's event count and digest is read back and compared with the tree's.
4. The migration is recorded in the database, keyed by the instance, and the counts are printed
   (`--format text|json`).

The tree is left where it is, and nothing reads it afterwards. Every other command refuses a state
directory whose tree the database holds no migration of, naming `conductor store migrate`. The
command refuses, copying nothing, a non-human actor (`ActorNotHuman`), a tree already migrated
(`AlreadyMigrated`) and a state directory with no tree (`SourceMissing`).
