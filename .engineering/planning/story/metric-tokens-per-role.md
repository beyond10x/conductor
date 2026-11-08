---
format: aep.planning-md/3
id: story:metric-tokens-per-role
kind: story
status: draft
title: conductor metric reads billed tokens per session role
summary: 9.27 G input tokens billed in 14.5 h with no reading per role or per merged unit
revision: 2
---
## Why

Host teacher's day retro, 2026-11-08, relayed by the supervisor: from 00:00 to 14:30Z the host
billed 9.27 G input and 44.8 M output tokens over 60,006 assistant turns. The teacher's own script
shared them out as about 47 % to sessions the operator runs, 39 % to 13 controllers, 11 % to the
maintainer and one other session, and 3 % to the host roles. No conductor reading holds this, so
"tokens per merged unit" cannot be tracked.

## Acceptance

1. `conductor metric compute` reads, for the day, the billed input and output tokens of each
   session role (controller, maintainer, supervisor, teacher, foreign) from the harness transcripts'
   assistant turns (`.message.usage`), and records one reading per role and one per instance. A
   test with fixture transcripts of two roles.
2. A reading `tokens per merged unit`: the instance's tokens over its merged requests of the day.
   Test.
3. `docs/design/self-improvement.md` names both readings. The teacher's prototype script is
   deleted once this ships or is refused.

## Ambiguities

How a transcript maps to a role: by the session name in the host's session list, else `foreign`.
