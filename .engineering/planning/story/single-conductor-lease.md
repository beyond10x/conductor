---
format: aep.planning-md/3
id: story:single-conductor-lease
kind: story
status: draft
title: 'One conductor per instance: a second start or resume is refused, a stale holder is taken over'
scope:
- confidence: cited
  path: crates/conductor/src/guard.rs
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: spec/domains
revision: 2
---
## Why

On 2026-10-12 two sessions named `conductor` were live: the managed background session in the
records directory (355bf50a) and the first conductor session of 2026-10-06 (d5745017, transcript
`25c3323a…`), resumed by the operator with cwd `~/beyond10x`. It came back under the same name,
and nothing refused it. Two conductors can each dispatch, decide, write the records and grant
build slots; a peer that addresses `conductor` gets an ambiguity error or reaches the wrong one
(the second session's own NEED to conductor-dev, 2026-10-12, operator request). `task
conductor:start` refuses only while a session of that name runs, and only for a start through
the task; a `--resume` or a start by hand passes.

## Design

A conductor lease in the instance's state directory: the session id and pid of the one conductor.
A new noun, so it is modelled in `spec/` first (`ess:specifying`): `conductor.session.Lease`
(instance, session id, pid, taken at) and the commands that take, show and release it.

- Taken at session start: a `SessionStart` hook in the conductor role's settings runs `conductor
  session claim`. A live holder that is not this session refuses the claim; the hook's output tells
  the session it is a second conductor, names the holder's session id and pid, and says to stop.
- Held by the guard: every conductor-role tool call already reaches `conductor guard`. While a live
  holder exists, a call from another session id is denied for every write (records, SendMessage,
  Bash writes), naming the holder. Reads stay allowed.
- A holder whose pid is gone is stale: the next claim takes the lease over and records that it did.
- A session resumed without the instance's settings (no guard, as d5745017 in `~/beyond10x`) is
  outside both. `conductor watch` reports two live sessions named `conductor` (from `claude agents
  --json`) as an incident, so conductor-dev and the operator hear of it.

## Acceptance

- A second claim while the holder's pid lives exits non-zero and names the holder's session id and
  pid; the guard denies that session's records write and its SendMessage with the same names.
  Tests.
- A claim over a holder whose pid is gone takes the lease and records the takeover. Test.
- `conductor watch` reports two live `conductor` sessions as an incident line. Test over a fixed
  `claude agents --json` listing.
- The spec models the lease and validates with the newest `ess`.

## Scope

`spec/domains/` (a session or guard domain), `crates/conductor-model/`, `crates/conductor/src/`
(a `session` command, `guard.rs`, the watch), `crates/conductor/tests/`, the conductor role's
settings template, `docs/design/conductor.md` § 7 and § 8.
