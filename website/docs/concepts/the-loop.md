---
title: The loop
sidebar_position: 2
description: Sense, compare, decide, dispatch, record, report; the five collectors of a snapshot; the watch between cycles; which repositories are Active.
lede: Conductor runs one cycle at a time over a snapshot it keeps in its store, so a restarted session resumes from its records, not from its context.
source: docs/design/conductor.md sections 4, 8 and 10; crates/conductor/src/snapshot.rs, crates/conductor/src/collect/, crates/conductor/src/watch.rs, crates/conductor/src/repository.rs; conductor --help
---

## One cycle

```text
sense     conductor snapshot start-snapshot     one snapshot: repositories, pull requests, main CI,
                                                releases, planning blockers, specifications, sessions
compare   goals and servings                    which goal each repository's work serves; idle goals;
                                                work serving no goal; red signals
decide    decision requests                     answer controller requests; pick next work per
                                                repository; escalate class O and H
dispatch  conductor dispatch send-dispatch      start, steer or stop a controller; one charter each
record    the store, and the session's logs     one record per decision, dispatch and message
report    operator brief                        once a day and on escalation, deviations only
```

A cycle runs every `cadence.cycle`, and a daily cycle at `cadence.daily` (UTC) carries the operator
brief. The session schedules both. A cycle also runs when a controller reports. Steering includes
cancelling a dispatch (`conductor dispatch cancel-dispatch`) when its brief no longer serves its
goal.

Snapshots are records, so a restarted conductor resumes from the newest complete snapshot. A
snapshot a collector did not answer is failed with the collector's name; it is kept and never read
as the current state.

## The collectors

`conductor snapshot start-snapshot` runs five collectors in order and completes the snapshot only
when all five answered.

| collector | reads | records |
|---|---|---|
| repositories | `git` in each checkout, `gh` for each `github` source | one row per repository: visibility, archived, open issues, `main` head, real commits in 7 days, behind `main`, dirty files, worktrees, latest release, unreleased commits, planning-store version, and `in_catalog` when the instance names a catalog |
| github | `gh pr list`, `gh run list` and releases per repository | open pull requests, the newest `main` run per workflow, releases, merged pull requests |
| blockers | `git fetch`, an export of `origin/main`, `aep plan workspace list` | open decision blockers across the planning stores |
| specifications | the same exports, `ess specify validate`, `ess verify conform synthesize` | per repository: specification present, opted out or missing; valid or refused; scenario count |
| sessions | `claude agents --json` | live sessions, their working directory, the repository each is bound to |

A `gitlab` source is not read by the binary. The session reads that forge through its own
integration and records the rows with the `conductor snapshot record-*` commands.

The views (`snapshot repositories`, `pull-requests`, `workflow-runs`, `blockers`, `specifications`,
`sessions`, `releases`, `merged-pull-requests`, `snapshots`) list every kept snapshot's rows; filter
on the newest `snapshot_id`. Each takes `--format text|json|jsonl|markdown`.

When a snapshot completes, the store keeps the observations of the newest `retention.snapshots`
complete snapshots and drops the others'. The snapshot records themselves stay.

## Active repositories

Conductor processes issues and pull requests only for repositories that are **Active**.
`conductor repository activity` decides each one:

- A mark (`conductor repository mark-repository`, `activate-repository`, `deactivate-repository`),
  set by the operator or conductor with a reason, wins and is never overwritten by a snapshot.
- Without a mark, a repository is Active when the newest complete snapshot counts at least one real
  commit on any `origin` branch in 7 days and it is not archived.

A commit is not real when it is a merge, an organization-wide sweep (the same normalised subject in
at least 3 repositories in the window), a maintenance subject, or touches only documentation, CI,
planning or lock files. [Add a repository rule](../guides/add-a-repository-rule.md) shows a mark
at work.

`conductor repository shipped --since <RFC 3339>` lists the releases and merged pull requests since
an instant, from the newest complete snapshot.

## The watch

Between cycles, `conductor watch run` runs cheap probes and prints one line per change. Conductor
runs it with `--follow` under its harness's monitor, and each line wakes it. Session starts print
nothing.

| probe | source | line |
|---|---|---|
| sessions | `claude agents --json`, sessions in the instance's checkouts, trees or records | `session exited (blocked): …`, `session gone: …` |
| usage limit | the tail of each such session's transcript | `usage limit: …`, and a desktop notification through `notify-send` |
| context | the newest usage line of each live session's transcript | `context: <repo> <n>k tokens` above `thresholds.context_handover` |
| `main` CI | `gh run list` for each Active repository of the newest complete snapshot | `CI red on main: …`, `CI green again on main: …` |
| free disk | `df` on `/` | `disk low: <n>G free on /` under `thresholds.disk_low`, again only after it was back at `thresholds.disk_clear` |

A pass runs every `cadence.watch` (`--every`), and CI is read every `cadence.ci` (`--ci-every`).
A session exit that a dispatch log line explains is not reported. Without `--follow`, the watch
exits after the first pass that finds a change.

## The store

Conductor's records live under the state directory: `--state-dir`, else the instance's `state`,
else `state/` under the working directory. Each record is one stream of an
[Eventlog](https://beyond10x.github.io/ecosystem/eventlog/)
([GitHub](https://github.com/beyond10x/eventlog)) tree store: one immutable file per event.

| directory | holds |
|---|---|
| `tree/` | snapshots, goals, servings, marks, decisions, requests, controllers, dispatches, messages, resource requests, guard verdicts |
| `observations/` | the observations of each snapshot, one tenant per snapshot |
| `watch/` | the watch's state files |

A view opens only an existing store and creates nothing; a command creates the store where none
is. `conductor store migrate` moves an older store into this layout and keeps the old one beside
it.
