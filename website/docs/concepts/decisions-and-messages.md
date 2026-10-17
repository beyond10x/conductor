---
title: Decision classes and messages
sidebar_position: 3
description: Class C conductor decides, class O the operator decides, class H needs the operator's hands; the fixed first lines every message between sessions carries.
lede: Every decision has a class that says who may take it, and every message has a first line a program can parse.
source: docs/design/conductor.md sections 5 and 6; docs/design/conductor-run.md section 3; crates/conductor/src/decide.rs, crates/conductor/src/message.rs, crates/conductor/src/run/mailbox.rs, crates/conductor/src/run/handlers.rs, crates/conductor/tests/decision.rs, crates/conductor/tests/adv_decision_ids.rs
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
- Everything else that is not class O or H: conductor takes the recommended option that can be
  undone, records it and reports it as taken.
  Nothing is held for the operator.

**Class O: the operator decides.** Conductor prepares the options and a recommendation, and
escalates (`conductor decision escalate-request`).

- Data loss: deleting another session's or person's work, an archive that is the only copy, a
  repository.
- Rewriting published history.
- A new, renamed or retired repository.
- An architecture decision: a change an architecture decision record would carry.

**Class H: the operator's hands, not judgement.** Setting a secret, a login, a paid account, a
permission only the operator holds. Conductor keeps one numbered list, with the exact command or
page for each item and the controllers waiting on it (`conductor decision hands-todo`).

`record-conductor-decision` refuses a class-O or class-H decision as `ReservedForOperator`, and
records nothing. The operator records what was decided or done with `record-operator-decision`.
It names the session that records it (`--recorded-by <session>`) and quotes the operator's words
the decision rests on (`--operator-words`); both are required, and `decision show` prints them.

The guard enforces one class-H line in code: a forge write of a credential (`gh secret`, `gh
auth`, `gh ssh-key` or `gh gpg-key`; `glab variable`, `glab auth`, `glab ssh-key`, `glab
gpg-key`, `glab deploy-key`, `glab token`, `glab config` or `glab securefile`) is denied to every guarded session, whoever
authorized it, and the denial hands the operator the exact command to run themselves. Every other
`gh` or `glab` write from a guarded session is denied too, as not a read
([the guard](/docs/security/the-guard)).

When a class is unclear, conductor treats it as class C, takes the option that can be undone, and
records why. That record is how the
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

A controller sends a message with `conductor message send --to conductor --from <session name>
--text -`, the message on standard input. The command reads the first line; one that does not
parse is answered with the format and exit 1, and nothing is written. Otherwise it leaves the whole
text as `<state>/mailbox/conductor/<ulid>.md` and records it received, transport `Mailbox`.
`conductor run` reads the mailbox each watch tick, and puts its own messages in front of a
controller with `conductor session deliver`, which types them into the session.

| first line | from → to | meaning |
|---|---|---|
| `[DISPATCH <repo> <dispatch-id>] <goal id> <what to do>` | conductor → controller | a brief |
| `[REPORT <repo> <dispatch-id>] <started\|progress\|blocked\|unblocked\|done\|failed> <one line>` | controller → conductor | progress on a dispatch |
| `[DECISION-REQUEST <repo> <request-id>] <question>` | controller → conductor | options A/B/C and a recommendation below |
| `[DECISION <request-id>] <option> <one-line reason> (<decision-id>)` | conductor → controller | the answer |
| `[NEED <repo> <request-id>] <other-repo> <what is needed>` | controller → conductor | a cross-repository request, delivered to the other repository's running controller or kept for its next start |
| `[HANDOVER <repo>] context <n>k` | conductor → controller | the controller's context is over `thresholds.context_handover`: it finishes its step, writes `<checkout>/.git/handoff/<UTC stamp>.md`, reports `[REPORT <repo> <dispatch-id>] blocked handover <path>` and stops; `conductor run` then starts it again from its charter and that file |
| `[RESOURCE <repo>] <build-slot\|disk\|usage> <amount>` | controller → conductor | answered "use conductor build run": a build reserves its own disk, `conductor build run --reserve <size> -- <command>` ([Disk](./disk.md)), and `conductor run` resumes the sessions a usage limit stopped |
| `[ESCALATION <request-id>] <O\|H> <question>` | conductor → operator | class O or H |

Conductor records every message it receives (`conductor message receive-message`; `message send`
records through it). It rejects one whose first line does not parse (`message reject-message`) by
answering with the format, and does not act on it.

Each `REPORT` verb but `progress` moves the dispatch through its `dispatch report-*` command:
`started`, `blocked`, `unblocked`, `done`, `failed`. A `progress` report is a message only.

A decision request also lands in the controller's own planning store as a decision blocker, where
the repository has one. The controller clears the blocker stating what was decided, with no
reference to conductor. Conductor records each request it receives (`decision
raise-decision-request`, with the blocker reference); the blockers collector is the cross-check,
because an open blocker with no recorded request is a controller that did not report.

Disk is reserved by the binary itself from the host ledger (`conductor build run`, `conductor host
grants`), with no message. The resource commands (`conductor resource grant-resource`,
`refuse-resource`, `release-resource`) remain; `conductor run` calls none of them.

:::note[Retired]

`[RESTART conductor] <reason>` is in the design but not in the specification, and does not parse.
`conductor run` keeps no context, so it is restarted with `conductor run --uninstall` and
`conductor run --install`, with no hand-over.

:::
