---
format: aep.planning-md/3
id: story:records-check-legacy-months
kind: story
status: implemented
title: records check leaves legacy month files and quoted findings unjudged, so the daily records commit can pass
summary: a hand-written month and a refusal quoting findings block every records commit
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T09:28:04Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T09:28:04Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-07T11:11:22Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Observed 2026-11-07 on an instance running 0.20.1: the daily tick's records commit is refused at
step 1 (`records check`) every day, so the records directory has not been committed for three
weeks and `REPORT.md` is never committed. `records check` printed 655 findings:

- 643 on lines of one month file of `dispatches/` that a conductor session wrote by hand while the
  commands already wrote the log: 610 `UnknownKeySet` and 33 `DecisionInDispatchLog` on lines
  after the first command-written line, which `legacy` in `crates/conductor/src/records.rs` takes
  as the end of the hand-written lines for the whole log.
- 8 `UnknownDecision` on hand-written lines citing decision ids from before the decision log
  existed; `check` judges citations on legacy lines too.
- 4 `UnknownDecision` on the `records-commit-refused` line the tick wrote that day: its `line`
  field quotes the check's own findings, and `decision_ids` reads ids out of any text. Each refusal
  therefore adds findings of its own, and the commit can never pass once one is refused.

## Acceptance

- A month file of a log is legacy when it comes before the first file of that log whose first line
  a command wrote (its key set is the log's). `records check` judges no line of a legacy file, for
  any finding, and prints on standard error how many lines it left unjudged and in which files.
- Every month file from that first one on is judged on every line, so a hand-written line after
  the switch is still found.
- `UnknownDecision` reads decision ids from every field of a dispatch line but `line`, the field
  that carries another program's output; a `records-commit-refused` line quoting a finding is no
  finding.
- On a fixture with one mixed legacy month followed by a command-written month, `records check`
  exits 0, and the daily tick's records commit passes step 1.
- Spec first: the `RecordsFindingKind` comment in `spec/domains/dispatch.yaml` says which lines are
  judged and which field is not read for decision ids.
