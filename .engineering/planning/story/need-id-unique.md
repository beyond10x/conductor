---
format: aep.planning-md/3
id: story:need-id-unique
kind: story
status: draft
title: Each NEED carries an id that was never used before
revision: 2
---
# Story: need-id-unique

## Outcome

Every `[NEED conductor <id>]` conductor sends has an id no earlier NEED used, so each report answers exactly one NEED.

## Context

Conductor sent one NEED id twice (2026-10-15): at ~01:29Z, asking for restart 8 (hand-over docs/handoff/2026-10-15-0129.md), and at 02:18Z, asking for restart 9 (hand-over docs/handoff/2026-10-15-0218.md). Each session minted the id by hand, and the second did not know the first had used it.

## Acceptance

- One command prints the next unused `N-<date>-<n>` from the records, either a new `conductor need new` or the command that already mints ids if one exists. Two consecutive calls across a simulated restart return different ids.
- `.agents/conductor.md` tells conductor to send a NEED only with an id from that command; tests/profiles.rs holds the sentence.

## Out of Scope

Ids of other messages (REPORT, RESTART).

## Ambiguities

None.
