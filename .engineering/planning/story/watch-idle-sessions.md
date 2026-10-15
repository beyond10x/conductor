---
format: aep.planning-md/3
id: story:watch-idle-sessions
kind: story
status: draft
title: The watch names a session that holds work and has gone silent, or waits on a build that ended
revision: 1
---
# Story: watch-idle-sessions

## Outcome

Conductor learns within minutes that a controller or worker holding a dispatch has gone quiet, or is waiting on a build that already exited, instead of finding out hours later.

## Context

The operator, 2026-10-15, through thalamus: sub-agents spent 7.8 h in 91 wait loops polling their builds, and a session stayed idle after its build ended. Rules item 139 now forbids polling. This story is the watch's side of it.

## Acceptance

- `conductor watch` prints `idle <session> <minutes>` once when a session holding a Running dispatch has shown no transcript activity for `thresholds.session_idle` (default 20 min). It prints the line again only after the session has been active in between.
- It prints `build-ended-unheard <session> <grant> <minutes>` once when a host grant the session started was Released more than 5 min ago and the session has shown no activity since.
- Tests use a fixed clock and fixture transcripts and grants.

## Out of Scope

Waking or restarting the session; conductor decides that.

## Ambiguities

- How a grant is attributed to its session: `inferable` from story:build-run-session-bound, which records the session on the request.
