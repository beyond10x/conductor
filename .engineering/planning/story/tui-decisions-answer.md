---
format: aep.planning-md/3
id: story:tui-decisions-answer
kind: story
status: draft
title: The operator answers an escalated decision in the TUI after a confirm step
relations:
- decomposes: epic:tui-frontend
- depends_on: story:tui-decisions-read
revision: 2
---
# Story: tui-decisions-answer

## Outcome

The operator answers an open escalated decision (class O or H) from the Decisions screen, chooses among its options, sees what will be recorded and confirms; the answer is recorded as his decision exactly as the dashboard records one, and the row leaves the open list.

## Context

- The recording path exists: `decide::answer_as_operator` (crates/conductor/src/decide.rs:607) over `answer-escalated-as-operator`, which records the operator's decision and answers the request in one step (spec/domains/decision.yaml:617-632; crates/conductor/src/decide.rs:551). `POST /api/answers` is a batch of these, row by row, recorded by `dashboard` (crates/conductor/src/dashboard/answers.rs:1-45; dashboard.rs:20-26).
- A decision recorded by the operator needs the session that ran it and his words (`--recorded-by`, `--operator-words`, decide.rs:319-330; spec/domains/decision.yaml:193). The TUI records `tui` as the session and the words the operator typed or the label of the option he chose, as the page does for its clicks.
- Only class O and H items are the operator's (decide.rs:283-308): the screen offers an answer for no other class.
- The confirm step is the product's rule for the operator's irreversible act: nothing is recorded on the first keypress.
- Shared surface: the guard table in `docs/design/conductor.md` § 7 and the guard code are also changed by `story:session-attach-refuses-sessions` and `story:demo-mode`; the merge cost is accepted, and each command's rule is its own entry.

## Acceptance

1. Spec first: the recorded-by value `tui` and the TUI's answer path are modelled with the other recorders of an operator answer in `spec/domains/decision.yaml` (near :617-632), validated with `ess specify validate --path spec`.
2. On an open row, `a` opens the answer form: the lettered options (several selectable for `several-of`), the recommendation preselected as a hint only, a free reason line; `enter` leads to the confirm step.
3. The confirm step prints the exact command that would be recorded (`conductor decision answer-escalated-as-operator ...` with request id, choice letters and the words), says "recorded as you", and requires `y`; any other key returns to the form; `esc` cancels and records nothing (a test shows the decision log and store unchanged).
4. After `y`, the item is answered in the store, a decision line is written with `decided_by` the operator and recorded by `tui`, and the item disappears from the open list on the next refresh; the same answer through the dashboard's endpoint for another item writes a line identical but for the recorder (one test compares both).
5. An answer refused by the specification (`RequestNotFound`, `RequestStateConflict`, `ChoiceInvalid`, `DecisionExists`, decide.rs:545-550) shows its name and message in the form and records nothing; a request answered meanwhile by the dashboard shows `RequestStateConflict`.
6. This story is the sole owner of keeping sessions out of `conductor tui`: a guard rule keeps a conductor, controller or worker session from running `conductor tui` or answering through it, with its entry in the guard table of `docs/design/conductor.md` § 7 (no `conductor` command that writes); a test shows the denial, and the security overview (`website/docs/security/overview.md`) states that a local process that can run the command is recorded as the operator, as it says for the page.

## Out of Scope

Batch answering (the page's batch is one request; the TUI answers one item per confirm); withdrawing or escalating requests.

## Ambiguities

- Whether to offer a batch confirm (several items marked, one confirm) later, as the page does.
- Whether the operator's free words are required, as `record-operator-decision` requires non-blank words (decide.rs:324); the label of the chosen option is proposed as the default.
