---
format: aep.planning-md/3
id: story:limit-mark-expires-mailbox-kept
kind: story
status: active
title: A stopped limit mark with no reset ahead waits no more; no banner, no limit mark; the mailbox keeps every handled name
summary: waiting_at_limit expires marks, exited resumes, mailbox.handled whole, cycle line in the runtime log
refs:
- provider: github
  reference: beyond10x/conductor#219
relations:
- decomposes: epic:usage-continuity
scope:
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/run/mailbox.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:40:01Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-19T18:40:01Z", actor: "human:unknown", revision: 3}
---
## Why

Issue #219. On 1.15.0 `<state>/run/limited.json` held marks `{"repo": ..., "stopped": true}` with
no `resets_at`, of sessions that ended in box restarts, not at a usage limit; every cycle wrote
"<repo> waits: usage limit until -" for ever (`run/dispatch.rs` `waiting_at_limit` keeps any
stopped mark). `run/handlers.rs` `exited` marks a session the list keeps blocked with no pid as a
usage limit though no banner was seen. The mailbox replayed reports handled days before:
`run/mailbox.rs` `read_all` rewrote `mailbox.handled` with only the names up to the file it
just handled, dropping handled names that sort after it. The cycle line went only to the
records, not to the runtime's log.

## Outcome

- A stopped mark with no reset that reads, or whose reset passed `token::LIMIT_WINDOW` ago, keeps
  no repository waiting: the cycle takes it out of `limited.json` and starts the repository as
  any other (`dispatch.rs` `waiting_in`).
- A session that ended with no limit banner is never marked limited: it is resumed from its
  transcript at once when its repository has an open dispatch (`handlers.rs` `exited`).
- `mailbox.handled` keeps every handled name present each time it is written.
- The cycle line, with the tick's id, also goes to standard error, the runtime's log.

## Acceptance (fail on main)

- `run/dispatch.rs` unit test `a_stopped_mark_with_no_reset_ahead_waits_no_more`.
- `tests/run.rs` `a_cycle_starts_a_repository_whose_stopped_limit_mark_names_no_reset`,
  `a_controller_blocked_with_no_pid_and_no_limit_banner_is_resumed_at_once_not_marked_limited`,
  `the_mailbox_keeps_every_handled_name_when_a_file_sorting_before_them_arrives`.

## Boundaries

- Place counting and dispatch reasons in `dispatch.rs` are another unit's.
