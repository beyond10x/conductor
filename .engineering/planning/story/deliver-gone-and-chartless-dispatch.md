---
format: aep.planning-md/3
id: story:deliver-gone-and-chartless-dispatch
kind: story
status: active
title: deliver names a gone session gone, deliver-message leaves the help, a chartless open dispatch ends
summary: 'Box sessions: a gone session, the unwritten leaf and a dispatch no controller can take'
refs:
- provider: github
  reference: beyond10x/conductor#215
relations:
- decomposes: epic:architecture-next
scope:
- confidence: cited
  path: crates/conductor-docs/src/cli.rs
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/confine/child.rs
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/tests/box_session.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: website/docs/reference/cli.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-19T18:33:50Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-19T18:33:50Z", actor: "human:unknown", revision: 3}
---
## Why

Issue #215. From the host, `conductor session deliver --name <prefix>-slot` for a session that was
gone (its broker record left) answered `SessionNotReachable … read the Claude session list claude
agents --json: start claude: No such file or directory`: the child registry knew the session had
ended, but delivery went on to the tmux socket and the session list
(`crates/conductor/src/session.rs:609`, `:639`), whose start error hid the cause.
`conductor session deliver-message` answered `not implemented` (`crates/conductor/src/cli.rs:433`)
while `conductor session --help` listed it (`cli.rs:1754`). And a controller whose session ended
while its dispatch was `Started` was never restarted when its charter was parked: the restart's
spawn refused `NoCharter`, and the loop only wrote `<repo> waits: …` (`src/run/dispatch.rs:251`) or
failed the handler (`src/run.rs:960`), leaving the dispatch open for ever.

## Outcome

- A delivery to a child session the registry holds and that ended names it gone (`the session is
  gone: its child session <id> ended …`), without asking the session list; from the host, a
  session of the box whose every entry ended is named gone the same way, without the runtime.
- `session deliver-message`, a placed leaf not written yet, is hidden from the help and the CLI
  reference; it still parses, as `cli_tree` requires.
- A restart for a gone (or stalled, or handed-over) session that spawn refuses `NoCharter` ends the
  repository's open dispatches with the reason: a started or blocked one `ReportFailed`, a sent one
  `CancelDispatch`.

## Acceptance (fail on main)

- `tests/box_session.rs` `a_delivery_to_a_gone_child_session_names_it_gone`.
- `tests/box_session.rs` `the_unwritten_deliver_message_leaf_is_not_in_the_help`.
- `tests/run.rs` `a_gone_session_without_a_charter_fails_its_open_dispatch` (binds a Unix socket:
  runs on CI).

## Boundaries

- Writing the `deliver-message` leaf (a delivery recorded with nothing typed) is not part of this.
