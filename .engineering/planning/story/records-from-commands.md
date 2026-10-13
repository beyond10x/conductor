---
format: aep.planning-md/3
id: story:records-from-commands
kind: story
status: draft
title: The commands that record an event write the records' decision and dispatch logs
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/controller.rs
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/src/dispatch.rs
- confidence: cited
  path: crates/conductor/src/message.rs
- confidence: cited
  path: crates/conductor/src/records.rs
- confidence: cited
  path: crates/conductor/src/resource.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/records_log.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: spec/domains/decision.yaml
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

On 2026-10-13 conductor appended about 310 lines to its records' `dispatches/` log by hand
(`echo`/`printf`/`jq >>`, about 170 Bash calls, a fifth of its tool calls), with 36 different event
verbs, 21 key sets and two spellings of the repository key. The CLI's own dispatch, message,
request and resource stores held no row at all, so every view built on them (the silent-session
watch, `decision hands-todo`, the operator-items age) saw nothing. Decisions were kept twice and
split three ways between the store and the hand-written `decisions/` log; conductor filed two
decision lines into the dispatch log by mistake and moved them with a hand rewrite of a records
file. `controller revise-charter` was never called, so every controller row reads charter revision
1 while the charters were at revision 17 and 18. Design § 10 still says the CLI does not write the
logs.

## Acceptance

- Spec first: each command that records a dispatch, report, message, request, decision, resource
  grant or charter revision declares the one log line it appends (event name and fields) in
  `spec/domains/dispatch.yaml` and `spec/domains/decision.yaml`; `ess specify validate --path spec`
  exits 0 and `task regen` leaves no diff.
- `dispatch send-dispatch`, `report-*`, `cancel-dispatch`, `message receive-message`,
  `resource grant-resource|refuse-resource` and `controller revise-charter` each append exactly
  one line to `<records>/dispatches/YYYY-MM.jsonl`, and `decision record-conductor-decision`,
  `raise-decision-request`, `answer-request` and `reverse-conductor-decision` one line to
  `<records>/decisions/YYYY-MM.jsonl`, with the key `repo` only; a test in
  `crates/conductor/tests/records_log.rs` runs each command once and reads its line back.
- `conductor spawn` records the dispatch it starts (`sent`, with the session id) as well as the
  controller; a test in `crates/conductor/tests/spawn.rs` reads both.
- `conductor records check`, declared first as a view in `spec/domains/dispatch.yaml`, lists a
  decision id cited in `dispatches/` and absent from `decisions/` and the store, a line with an
  unknown key set, and a decision event in the dispatch file; over a fixture with one of each it
  prints three findings and exits 1 (`crates/conductor/tests/records_log.rs`).
- `.agents/conductor.md` says: once a command records an event, no hand-written line for it; a test
  in `crates/conductor/tests/profiles.rs` holds the sentence. Design § 10's "The CLI does not write
  them yet" is replaced by the list of commands that write them.

## Notes

- First of the records stories: `story:request-ids-allocated`, `story:decision-amend` and
  `story:handover-draft` depend on it.
- Importing the lines written by hand before this story (2026-10-01..13) into the store is a
  follow-up, not this story.
- Edits `crates/conductor/src/cli.rs` and regenerates `crates/conductor-model` and
  `website/docs/reference/cli.md`, like every spec-first story of this set; one at a time on those
  files.

## Scope

`spec/domains/dispatch.yaml`, `spec/domains/decision.yaml`, `crates/conductor-model`,
`crates/conductor/src/dispatch.rs`, `crates/conductor/src/message.rs`,
`crates/conductor/src/decide.rs`, `crates/conductor/src/resource.rs`,
`crates/conductor/src/controller.rs`, `crates/conductor/src/spawn.rs`,
`crates/conductor/src/records.rs` (new), `crates/conductor/src/cli.rs`,
`crates/conductor/cli.yaml`, `crates/conductor/tests/records_log.rs` (new),
`crates/conductor/tests/spawn.rs`, `crates/conductor/tests/profiles.rs`, `.agents/conductor.md`,
`docs/design/conductor.md`, `website/docs/reference/cli.md`.
