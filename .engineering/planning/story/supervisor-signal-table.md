---
format: aep.planning-md/3
id: story:supervisor-signal-table
kind: story
status: draft
title: 'The supervisor''s routine is one table of signals: source, condition, action'
summary: 11 of 13 supervisor misses in one night were signals not looked at
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
# Story: supervisor-signal-table

## Why

Host teacher's retro, 2026-11-08: of 13 misses of the supervisor between 23:40 and 11:00 local
time, 11 were signals nobody looked at and 0 were wrong judgements. The routine of
`.agents/conductor-supervisor.md` is prose, and the profile is at its line cap (458,
`crates/conductor/tests/profiles.rs`), so each new rule displaces another.

## Acceptance

1. The routine section of `.agents/conductor-supervisor.md` is one table, one row per signal:
   source command, condition, action. It replaces the prose it came from.
2. It holds at least these rows: a `waiting` session; a `context:` line whose session id is not
   the controller's; `resume after a usage limit … exited <n>`; the route session listed again
   while `posted: pending` records exist; disk below the alarm; best pool headroom under 10 % or
   `no_better_account`; the maintainer idle with a hand-over newer than its start; the maintainer
   over `thresholds.context_handover`; a restart by hand only 15 min after idle; `LaunchFailed`;
   `session gone` without a spawn within 15 min.
3. The profile stays under its cap; `crates/conductor/tests/profiles.rs` pins the table's
   header and the rows above.

## Out of Scope

New watch lines for these signals; those are their own stories.

## Ambiguities

None.
