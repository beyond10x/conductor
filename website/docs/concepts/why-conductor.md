---
title: Why conductor
sidebar_position: 0
description: The problem conductor solves, one day with it on a sample instance of two repositories, what it costs to adopt, and what can be adopted alone before any session is started.
lede: Many repositories and many agent sessions leave nobody holding the whole picture; conductor holds it, answers what it may, and records every decision.
source: docs/design/conductor.md sections 1 to 6; docs/design/architecture-next.md; crates/conductor/src/run.rs, snapshot.rs, dispatch.rs, decide.rs, spawn.rs, doctor.rs, setup.rs, status.rs, trust.rs; conductor --help
---

## The problem

An engineer who works with coding agents across a dozen repositories runs several sessions at
once. Each session sees one checkout. Nobody sees all of them: which pull request waits on a
review, which `main` is red, which repository has had no real commit for a week, which session
stopped an hour ago on a question it asked into a terminal no one is watching. The engineer
becomes the message bus, and work waits on whoever reads the next question.

## What conductor does about it

| idea | how |
|---|---|
| One loop holds the picture | `conductor run`, a process with no model, takes a snapshot of every repository of the instance (git state, pull requests, CI, releases, planning blockers, live sessions) and compares it with the goals the operator set |
| One controller per repository does the work | conductor dispatches a brief to a controller session in that repository's checkout; the controller plans and delivers in waves and reports only to conductor ([roles](./roles.md)) |
| Questions are answered, not parked | every decision has a class; conductor takes the ones that can be undone (class C) and records why; only class O, what cannot be undone, and class H, what needs the operator's hands, reach a person ([decision classes](./decisions-and-messages.md)) |
| Everything is a record | decisions, dispatches, messages and snapshots are kept in the store, so a restarted process resumes from records and the operator reads what happened ([records and store](./records-and-store.md)) |

conductord, the daemon of the control host, keeps each instance's runtime running from the config
file ([the host](./the-host.md)).

## One day with conductor

A sample instance `demo` watches two local repositories, `alpha` and `beta`. The loop runs most
commands below. The operator reads the board, the dashboard and the escalations. The script to record this day is
[`docs/demo/walkthrough.md`](https://github.com/beyond10x/conductor/blob/main/docs/demo/walkthrough.md).

| when | command | what happens |
|---|---|---|
| morning | `conductor snapshot start-snapshot`, `conductor repository activity` | one snapshot; only `alpha` is Active, so only its issues and pull requests are processed |
| morning | `conductor snapshot publish-board --snapshot-id <id>` | `STATUS.md` in the records: deviations first, then an overview, specifications, open blockers and sessions |
| goal | `conductor goal propose-goal --goal-id G-1 …`, `goal confirm-goal --goal-id G-1` | the goal the operator wrote into `NORTHSTAR.md`, recorded |
| dispatch | `conductor dispatch send-dispatch --repository alpha --goal-id G-1 --brief …`, `conductor spawn --repository alpha` | `alpha`'s controller starts in its checkout with its role's profile, model and settings, so the [guard](/docs/security/the-guard) runs, and receives `[DISPATCH alpha DSP-<date>-01] G-1 plan and deliver the first release` |
| wave | `decision raise-decision-request`, `decision record-conductor-decision --decision-class C …`, `decision answer-request` | the controller asks `[DECISION-REQUEST alpha REQ-1] approve the wave …?`; approving a wave is class C, so conductor answers `[DECISION REQ-1] A … (DEC-<date>-01)` and no person is asked |
| escalation | `decision escalate-request --escalation-class O` | deleting the only copy of an old prototype branch cannot be undone, so it is class O, escalated with options and a recommendation |
| answer | `decision record-operator-decision --recorded-by … --operator-words "archive it first"`, `decision answer-escalated-request` | the operator answers in a sentence; conductor records it as the operator's decision, quoting the words |
| evening | `conductor dispatch report-done --dispatch-id DSP-<date>-01 --evidence "alpha v0.1.0 released"`, `conductor repository shipped --since <instant>` | the dispatch is `Done`; the daily brief lists deviations only |

The dashboard at `http://127.0.0.1:7300/?instance=demo`, served by conductord, shows each
session and its work, the dispatches, the decision and dispatch logs, CI, usage and disk. Its
writes are the operator's answers and settings.

## What it costs to adopt

| what | needs |
|---|---|
| programs | `git`, `claude`, `jq`, `aep`, `worktree`, `ess`; `gh` or `glab` for the forge; `substrate-daemon` for an instance in a substrate box. `conductor doctor` lists each against its pin and says what is missing ([getting started](/docs/getting-started)) |
| a config | `version: conductor.config/2` and, per instance, a name, `session_prefix`, `placement`, sources and checkouts; conductord also needs `daemon.store` ([config](./config.md)) |
| a first snapshot | install, a config, `conductor snapshot start-snapshot`; no session starts |
| a first dispatch | the records (`conductor init`), roles with a profile and settings each, `conductor trust`, and Claude Code's bypass-permissions disclaimer accepted once ([sessions](/docs/security/sessions)) |
| trust | sessions run unattended with no permission prompts; the guard narrows what they write and is no sandbox ([security overview](/docs/security/overview), [run isolated](/docs/guides/run-isolated)) |
| planning stores | blockers and wave plans come from each repository's AEP store; a repository without one is tracked, without its blockers |

In this release `conductor setup --apply` writes a `conductor.config/1` file, which conductord does
not read.

## What can be adopted alone

None of these starts a session or dispatches anything:

| piece | gives | writes |
|---|---|---|
| `conductor doctor`, `conductor setup` | the programs, the forge logins, the config, records and trust of a repository | nothing, without `--apply` |
| `conductor snapshot start-snapshot` and its views | every repository's state, pull requests, CI, releases, blockers, sessions | the store |
| `conductor repository activity`, `repository shipped` | which repositories are Active; what shipped since an instant | nothing |
| `conductor snapshot publish-board` | a `STATUS.md` board of one snapshot | the records directory |
| `conductor status` | what asks for attention in the newest complete snapshot | nothing |
