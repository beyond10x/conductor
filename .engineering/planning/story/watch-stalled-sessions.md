---
format: aep.planning-md/3
id: story:watch-stalled-sessions
kind: story
status: draft
title: The watch names a session that makes no progress and a build that waits too long
relations:
- decomposes: epic:controller-health
revision: 2
---
# Story: watch-stalled-sessions

## Outcome

The watch names a driven session that is making no progress, even while its transcript keeps moving. It also names a build request that has waited too long. Conductor checks in on each such session within its cycle.

## Context

Conductor's NEED N-20261017-08. The operator's words (2026-10-17, DEC-20261017-30): "tell conductor about things ended up stuck, its conductors responsibility to check in on you and make sure things keep moving".

What was missed: the llm w08 wave made no commit for 63 min while its builds sat in the host.lock convoy (story:build-queue-convoy). `session silent` (`thresholds.silent_session`, 30m) did not fire, because the session kept writing to its transcript. The 2 h cycle was too slow to catch it. Meanwhile conductor runs the check as a 30-minute job with a 45-minute threshold (DEC-20261017-31).

## Acceptance

1. The watch prints `session stalled: <name> <m>m` for a driven session that has had, for `thresholds.stalled_session` (default 45m), neither a report received by conductor nor a commit in its checkout or in any of its trees. Tests: a stalled session is flagged; a session with a recent report or a recent commit is not.
2. The watch prints `build queued: <grant> <repository|session> <m>m` for a grant `Requested` longer than `thresholds.build_queued` (default 30m). Test.
3. Both keys are in the config spec, with `config show` and `config validate` covering them. The dashboard's attention list shows both lines.
4. `.agents/conductor.md`, "What you do, every cycle", names the check-in: for each stalled session or long-queued build, ask the session what blocks it and act on the answer, quoting the operator's words above. The profiles test pins the sentence.

## Out of Scope

Acting on a stall automatically, for example by restarting the session.

## Ambiguities

None.
