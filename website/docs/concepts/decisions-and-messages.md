---
title: Decision classes and messages
sidebar_position: 3
description: Class C conductor decides, class O the operator decides, class H needs the operator's hands; the fixed first lines every message between sessions carries.
lede: Every decision has a class that says who may take it, and every message has a first line a program can parse.
source: docs/design/conductor.md sections 5 and 6; crates/conductor/src/decide.rs, crates/conductor/src/message.rs, crates/conductor/tests/decision.rs, crates/conductor/tests/adv_decision_ids.rs
---

## Three classes

**Class C: conductor decides.** It records the decision (`conductor decision
record-conductor-decision`) and tells the operator only by exception.

- Wave approvals; commit, push, pull request and merge words.
- Releases through each repository's own process.
- Dependency upgrades, CI repair, baseline moves of the repository's gates.
- Disk and build scheduling. Cleanup of merged worktrees.
- Order and priority inside an accepted epic or initiative. Design choices inside an approved
  design.
- Issue triage, which profile or model a dispatch uses, and which repository waits for which.

**Class O: the operator decides.** Conductor prepares the options and a recommendation, and
escalates (`conductor decision escalate-request`).

- New, renamed, split, merged or retired repositories.
- Architecture-level changes: a contract between repositories, a noun crossing repositories, the
  direction or an objective.
- Public-facing breakage, data loss, outages.
- Security trade-offs outside an approved design.
- Credentials, accounts and spending.
- Deleting another session's or person's work.
- Changes to the operator's own rules.

**Class H: the operator's hands, not judgement.** Setting a secret, a login, a paid account, a
permission only the operator holds. Conductor keeps one numbered list, with the exact command or
page for each item and the controllers waiting on it (`conductor decision hands-todo`).

`record-conductor-decision` refuses a class-O or class-H decision as `ReservedForOperator`, and
records nothing. The operator records what was decided or done with `record-operator-decision`.

When a class is unclear, conductor treats it as class O and records why. That record is how the
boundary moves over time. A request already answered is answered again from the record, never
asked of the operator a second time.

## Answers and reversals

| who | answers | reverses |
|---|---|---|
| conductor | an open request (`decision answer-request`) | only its own decisions (`decision reverse-conductor-decision`) |
| the operator | an escalated request (`decision answer-escalated-request`) | any decision (`decision reverse-operator-decision`) |

A decision id carries its UTC date and a number. Given no `--decision-id`, the record commands
allocate the next id for the UTC date of `--decided-at`.

## Authority

Conductor decides class C on the operator's standing grant, written in the instance's
`authority.class_c`. The instance's `rules.md` says where the operator wrote that grant and how the
operator decides, so conductor can pick the option the operator would. Every class-C decision
conductor sends carries its decision id; one without an id is a recommendation.

The controller profile tells a controller to act on a `[DECISION …]` that carries an id as the
operator's, within conductor's grant. Where the operator's own harness instructions would make a
controller ask the operator instead, they need the same grant; without it, the loop stalls on
routine approvals.

## Messages

A message's first line is machine-readable. The rest is free text, at most 20 lines; detail goes
in a file in the sender's own repository, and the message carries its path.

| first line | from → to | meaning |
|---|---|---|
| `[DISPATCH <repo> <dispatch-id>] <goal id> <what to do>` | conductor → controller | a brief |
| `[REPORT <repo> <dispatch-id>] <started\|progress\|blocked\|unblocked\|done\|failed> <one line>` | controller → conductor | progress on a dispatch |
| `[DECISION-REQUEST <repo> <request-id>] <question>` | controller → conductor | options A/B/C and a recommendation below |
| `[DECISION <request-id>] <option> <one-line reason> (<decision-id>)` | conductor → controller | the answer |
| `[NEED <repo> <request-id>] <other-repo> <what is needed>` | controller → conductor | a cross-repository request |
| `[RESOURCE <repo>] <build-slot\|disk\|usage> <amount>` | controller → conductor | asked before a large build |
| `[ESCALATION <request-id>] <O\|H> <question>` | conductor → operator | class O or H |

Conductor records every message it receives (`conductor message receive-message`). It rejects one
whose first line does not parse (`message reject-message`) by answering with the format, and does
not act on it.

Each `REPORT` verb but `progress` moves the dispatch through its `dispatch report-*` command:
`started`, `blocked`, `unblocked`, `done`, `failed`. A `progress` report is a message only.

A decision request also lands in the controller's own planning store as a decision blocker, where
the repository has one. The controller clears the blocker stating what was decided, with no
reference to conductor. Conductor records each request it receives (`decision
raise-decision-request`, with the blocker reference); the blockers collector is the cross-check,
because an open blocker with no recorded request is a controller that did not report.

A `[RESOURCE]` request is granted or refused against the instance's thresholds
(`conductor resource grant-resource`, `refuse-resource`, `release-resource`).

:::caution[Planned]

Two more first lines are in the design but not in the specification yet:
`[HANDOVER <repo>] context <n>k` (conductor asks a controller to write a hand-over and stop above
`thresholds.context_handover`) and `[RESTART conductor] <reason>` (conductor-dev asks conductor to
hand over before a restart). The profiles use them; no command records them.

:::
