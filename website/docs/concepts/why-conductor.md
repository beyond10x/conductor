---
title: Why conductor
sidebar_position: 0
description: The problem conductor solves, one day with it on a sample instance of two repositories, what it costs to adopt, and what can be adopted alone before any session is started.
lede: Many repositories and many agent sessions leave nobody holding the whole picture, and work stops on questions nobody sees; conductor holds the picture, answers what it may, and records every decision.
source: docs/design/conductor.md sections 1 to 6; crates/conductor/src/snapshot.rs, crates/conductor/src/dispatch.rs, crates/conductor/src/decide.rs, crates/conductor/src/spawn.rs, crates/conductor/src/dashboard.rs; Taskfile.yml, status and dashboard; conductor --help; a sample instance run on conductor 0.8.1
---

## The problem

An engineer who works with coding agents across a dozen repositories runs several sessions at
once. Each session sees one checkout. Nobody sees all of them: which pull request waits on a
review, which `main` is red, which repository has had no real commit for a week, which session
stopped an hour ago on a question it asked into a terminal no one is watching. The engineer
becomes the message bus, and work waits on whoever reads the next question.

## What conductor does about it

- **One session holds the picture.** The conductor session takes a snapshot of every repository
  of the instance (git state, pull requests, CI, releases, planning blockers, live sessions) and
  compares it with the goals the operator set ([the loop](./the-loop.md)).
- **One controller per repository does the work.** Conductor dispatches a brief to a controller
  session in that repository's checkout; the controller plans and delivers in waves and reports
  only to conductor ([roles](./roles.md)).
- **Questions are answered, not parked.** Every decision has a class. Conductor takes the ones
  that can be undone (class C) and records why; only what cannot be undone (class O) and what
  needs the operator's hands (class H) reach a person
  ([decision classes](./decisions-and-messages.md)).
- **Everything is a record.** Decisions, dispatches, messages and snapshots are kept in an
  append-only store, so a restarted session resumes from its records, and the operator reads what
  happened instead of asking.

## One day with conductor

A sample instance `demo` watches two local repositories, `alpha` and `beta`, as in
[getting started](../getting-started.md). Most commands below are run by the conductor session,
not by a person; the operator reads the board, the dashboard and the escalations. Ids, times and
widths differ on your machine.

### 1. Morning: the snapshot and the board

```console
$ conductor snapshot start-snapshot
01a11c9d-4c98-7057-9d12-f1bf3c34ae9f
$ conductor repository activity
repository  activity  decided_by  marked_by  reason
alpha       Active    Snapshot    null       null
beta        Inactive  Snapshot    null       null
$ conductor snapshot publish-board --snapshot-id 01a11c9d-4c98-7057-9d12-f1bf3c34ae9f
01a135c5-cb2a-72c7-ac66-8a2eb5fd5180
```

`publish-board` writes `STATUS.md` in the records directory: deviations first, then an overview,
specifications, open blockers and sessions. `task status` runs both steps from a checkout of the
conductor repository. Only `alpha` is Active, so only its issues and pull requests are processed.

### 2. A goal and a dispatch

The operator has written a goal into `NORTHSTAR.md`. Conductor records it and sends `alpha` a
brief:

```console
$ conductor goal propose-goal --goal-id G-1 --title "alpha ships its first release" --exit-evidence "a v0.1.0 release of alpha"
$ conductor goal confirm-goal --goal-id G-1
$ conductor dispatch send-dispatch --dispatch-id DSP-20261013-01 --repository alpha --goal-id G-1 \
    --brief "plan and deliver the first release" --sent-at 2026-10-13T09:00:00Z --priority 1
DSP-20261013-01
$ conductor spawn --repository alpha
```

`spawn` starts `alpha`'s controller in its checkout, with the controller role's profile, model and
settings (so the [guard](/docs/security/the-guard) runs) and its charter, and records it running.
The brief reaches it as `[DISPATCH alpha DSP-20261013-01] G-1 plan and deliver the first release`.

### 3. The controller's wave

The controller plans the release in `alpha`'s planning store, picks the stories that can be built
at once, and asks before it starts:

```text
[DECISION-REQUEST alpha REQ-1] approve the wave of stories release-notes and version-flag?
```

Approving a wave is class C. Conductor records the request and its answer, and replies with the
decision id:

```console
$ conductor decision raise-decision-request --request-id REQ-1 --repository alpha \
    --question "approve the wave of stories release-notes and version-flag?" \
    --options "A approve; B hold" --recommendation A --raised-at 2026-10-13T10:00:00Z
$ conductor decision record-conductor-decision --decision-class C --repository alpha \
    --question "approve the wave of stories release-notes and version-flag?" --options "A approve; B hold" \
    --choice A --reason "inside the accepted goal; both can be reverted" --evidence REQ-1 \
    --decided-at 2026-10-13T10:02:00Z
DEC-20261013-01
$ conductor decision answer-request --request-id REQ-1 --decision-id DEC-20261013-01
```

The controller reads `[DECISION REQ-1] A … (DEC-20261013-01)`, works the wave with its own
workers, and reports `[REPORT alpha DSP-20261013-01] started …`, which conductor records with
`conductor dispatch report-started`. No person was asked.

### 4. An escalation, answered

Later the controller asks whether to delete an old prototype branch, the only copy of some
experiments. Data loss cannot be undone, so this is class O, and conductor escalates it with the
options and its recommendation:

```console
$ conductor decision raise-decision-request --request-id REQ-2 --repository alpha \
    --question "delete the old prototype branch, the only copy of its experiments?" \
    --options "A delete; B archive first" --recommendation B --raised-at 2026-10-13T11:00:00Z
$ conductor decision escalate-request --request-id REQ-2 --escalation-class O --reason "data loss cannot be undone"
$ conductor decision requests --format jsonl | jq -c '{request_id, state, escalation_class}'
{"request_id":"REQ-1","state":"Answered","escalation_class":null}
{"request_id":"REQ-2","state":"Escalated","escalation_class":"O"}
```

The operator answers in a sentence. Conductor records it as the operator's decision, quoting the
words and naming itself as the recorder, and closes the request:

```console
$ conductor decision record-operator-decision --decision-class O --repository alpha \
    --question "delete the old prototype branch?" --options "A delete; B archive first" --choice B \
    --reason "keep the only copy" --evidence REQ-2 --decided-at 2026-10-13T11:20:00Z \
    --recorded-by conductor --operator-words "archive it first"
DEC-20261013-02
$ conductor decision answer-escalated-request --request-id REQ-2 --decision-id DEC-20261013-02
$ conductor decision decisions --format jsonl | jq -c '{decision_id, decision_class, decided_by, choice}'
{"decision_id":"DEC-20261013-01","decision_class":"C","decided_by":"Conductor","choice":"A"}
{"decision_id":"DEC-20261013-02","decision_class":"O","decided_by":"Operator","choice":"B"}
```

### 5. The dashboard

```console
task dashboard          # or: conductor dashboard serve
```

A read-only page on `127.0.0.1:7313`: each session and what it works on, the dispatches, the
decision and dispatch logs, CI, usage and disk ([read the dashboard](../guides/read-the-dashboard.md)).

### 6. Evening

The controller reports `[REPORT alpha DSP-20261013-01] done alpha v0.1.0 released`:

```console
$ conductor dispatch report-done --dispatch-id DSP-20261013-01 --evidence "alpha v0.1.0 released"
$ conductor dispatch dispatches --format jsonl | jq -c '{dispatch_id, state, repository, goal_id}'
{"dispatch_id":"DSP-20261013-01","state":"Done","repository":"alpha","goal_id":"G-1"}
$ conductor repository shipped --since 2026-10-13T00:00:00Z
```

The daily cycle's brief to the operator lists deviations only. Everything else is in the records.

## What it costs to adopt

- **Programs.** Claude Code, `git`, `task`, `jq`, `aep`, `worktree`, `ess`, and `gh` or `glab`
  for the forge; `conductor doctor` lists each and says what is missing. The binary comes from
  `cargo install` or a release archive ([getting started](../getting-started.md#install)).
- **A config of a few lines.** A name, its sources and its checkouts; every other key has a
  default. Where there is no config file, `conductor setup --apply` writes one for the
  repository it runs in.
- **To first snapshot:** install, a config, `conductor snapshot start-snapshot`. Getting started
  goes that far without starting a session.
- **To first dispatch:** the records (`conductor init`), roles with a profile and settings each,
  `task trust`, and accepting Claude Code's bypass-permissions disclaimer once
  ([start conductor](../guides/start-conductor.md)).
- **Trust.** Sessions run unattended with no permission prompts. The guard narrows what they
  write; it is not a sandbox. Read the [security overview](/docs/security/overview), and
  [run conductor isolated](../guides/run-isolated.md) on a machine you care about.
- **Planning stores.** Blockers and wave plans come from each repository's AEP store; a
  repository without one is tracked, without its blockers.

## What can be adopted alone

None of these starts a session or dispatches anything:

| piece | gives | writes |
|---|---|---|
| `conductor doctor`, `conductor setup` | the programs, the forge logins, the config, records and trust of a repository | nothing |
| `conductor snapshot start-snapshot` and its views | every repository's state, pull requests, CI, releases, blockers, sessions | the state directory |
| `conductor repository activity`, `repository shipped` | which repositories are Active; what shipped since an instant | nothing |
| `conductor snapshot publish-board`, `task status` | a `STATUS.md` board of one snapshot | the records directory |
| `task dashboard` | the live sessions, CI, usage and disk, read-only | nothing |

Start there, and add sessions once the board says what you expected.

## A recording

[`docs/demo/walkthrough.md`](https://github.com/beyond10x/conductor/blob/main/docs/demo/walkthrough.md)
is a terminal script for recording this day on a fresh sample instance: from `conductor setup` to a
published board and the dashboard with no step that needs a person, then the session steps, each
marked where a person is needed.
