---
format: aep.planning-md/3
id: story:records-check-quoted-message
kind: story
status: active
title: records check reads no decision citation out of a field that quotes another's text
summary: A DEC- id a message's first line or a report's evidence quotes is no UnknownDecision
refs:
- provider: github
  reference: beyond10x/conductor#232
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: crates/conductor/src/records.rs
- confidence: cited
  path: crates/conductor/tests/records_log.rs
- confidence: cited
  path: crates/conductor/tests/run_records.rs
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: website/docs/concepts/records-and-store.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T01:04:25Z", actor: "human:unknown", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-20T01:04:26Z", actor: "human:unknown", revision: 3}
---
## Why

Issue #232. `conductor records check` reports `UnknownDecision` for dispatch-log lines whose
`first_line` (a `delivered` or `message-received` event) quotes a controller's text, such as "the
DEC-05 urgency change" or "cap 512 KiB, DEC-04": a repository's own short ids, not conductor
decisions. The daily tick then refuses the records commit (`records-commit-refused`), so the
records are not committed while such a line is in the log, and a command's line may not be edited
to get past it. `check` (`crates/conductor/src/records.rs:754`) removes only the field `line`
(`QUOTED`, `crates/conductor/src/records.rs:731`) before `decision_ids`
(`crates/conductor/src/records.rs:706`) reads every `DEC-…` of the rest.

## Outcome

- Every field of a dispatch line that quotes another's text is no citation: `line` (another
  program's output, or a watch line), `first_line` (a message's first line, as its sender wrote
  it) and `evidence` (a controller's report, or what `gc` or the bot route printed). A decision id
  in one of them is not judged.
- The fields conductor writes itself stay citations: `reason`, `brief`, `grant`, `id` and the
  rest; an unknown decision id in them is still `UnknownDecision`.
- Spec: `conductor.dispatch.RecordsFindingKind` in `spec/domains/dispatch.yaml` names the quoted
  fields; the module doc of `records.rs` and the records pages say the same.

## Acceptance (fail on main)

- `crates/conductor/tests/records_log.rs`
  `records_check_reads_no_citation_out_of_a_quoted_message_or_evidence`: a dispatch log whose
  first line a command wrote, then a `message-received` line whose `first_line` quotes "DEC-05",
  a `delivered` line quoting "DEC-04" and a `done` line whose `evidence` quotes one: `records
  check` exits 0.
- `records_check_names_an_unknown_decision_an_unknown_key_set_and_a_decision_in_the_dispatch_log`
  still holds: an unknown id in `reason` is `UnknownDecision`.

## Boundaries

The check only: no log line is rewritten, and what a command writes into a line does not change.
