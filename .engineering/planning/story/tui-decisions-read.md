---
format: aep.planning-md/3
id: story:tui-decisions-read
kind: story
status: draft
title: The TUI lists the open escalated decisions with their options, and the decision history
relations:
- decomposes: epic:tui-frontend
- depends_on: story:tui-overview
revision: 2
---
# Story: tui-decisions-read

## Outcome

A Decisions screen in `conductor tui` lists the open escalated decisions with what the operator chooses from, and the decision history, from the same read model as the dashboard's Decisions page.

## Context

- `/api/decisions` answers `{open, history, errors}`: each open row has `request_id`, `class`, `repository`, `question`, `kind` (`yes-no`, `one-of`, `several-of`), lettered `options` with `cost`, `recommended`, `default`, `deadline`, `reason`, `options_text` and `recommendation_text`; the history is every line of the decision log, newest first (crates/conductor/src/dashboard/decisions.rs:1-30, :36-45).
- Open items are ordered as `conductor status` orders them (`crate::status::unanswered`, decisions.rs:48-58); a request escalated before options were recorded reads `one-of` with no options and shows its words alone (decisions.rs:14-17).
- The screen is a second view over the function `story:tui-overview` shares; no TUI code reads the decision log.
- The screen registers in the screen registry `crates/conductor/src/tui/screens.rs` (`story:tui-overview`) with its key, and calls the read-model module that story extracts.

## Acceptance

1. Spec first: no new noun; the `tui` command's model (`conductor.observation.TuiRun`, `story:tui-overview`) is unchanged, validated with `ess specify validate --path spec`.
2. `2` or `tab` opens the Decisions screen: one row per open item oldest first with class, repository, question and its deadline; a detail pane shows `reason`, the lettered options with cost, the recommendation and the default.
3. A snapshot test over a fixture with a `yes-no`, a `one-of`, a `several-of` and an option-less item matches stored frames and the row order equals `/api/decisions`'s `open` order.
4. The history tab lists the log newest first, with `decided_by`, `choice` and `reason`; a line that does not read is listed last with a marker, as the page does.
5. `errors` of the answer (unreadable log files) show in the screen's footer.
6. The screen's count of open items equals the overview's unanswered count in the same fixture.

## Out of Scope

Answering (`story:tui-decisions-answer`).

## Ambiguities

None.
