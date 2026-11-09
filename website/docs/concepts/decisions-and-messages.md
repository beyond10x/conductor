---
title: Decision classes and messages
sidebar_position: 4
description: Class C conductor decides, class O the operator decides, class H needs the operator's hands; who answers and reverses; the first lines every message between sessions carries.
lede: Every decision has a class that says who may take it, and every message has a first line a program can parse.
source: docs/design/conductor.md sections 5 and 6; docs/design/conductor-run.md section 3; crates/conductor/src/decide.rs, decide/handle.rs, message.rs, session.rs, run/mailbox.rs, run/handlers.rs; crates/conductor/tests/decision.rs, adv_decision_ids.rs
---

## Three classes

| class | who decides | examples | command |
|---|---|---|---|
| C | conductor, recorded, told to the operator by exception | wave approvals; commit, push, pull request and merge; releases through each repository's own process; dependency upgrades, CI repair, baseline moves of a repository's gates; disk and build scheduling; cleanup of merged worktrees; order inside an accepted epic; design choices inside an approved design; issue triage; which profile or model a dispatch uses | `decision record-conductor-decision` |
| O | the operator, from options and a recommendation conductor prepares | data loss (another session's or person's work, an archive that is the only copy, a repository); rewriting published history; a new, renamed or retired repository; an architecture decision | `decision escalate-request` |
| H | the operator's hands, not judgement | setting a secret, a login, a paid account, a permission only the operator holds | `decision hands-todo` |

Everything that is not class O or H is class C: conductor takes the recommended option that can be
undone, records it and reports it as taken. That record is how the boundary moves over time. A
request already answered is answered again from the record, and is not asked of the operator a
second time.

`decision hands-todo` is one numbered list of class-H items, each with the exact command or page
and the controllers waiting on it.

`record-conductor-decision` refuses a class-O or class-H decision (`ReservedForOperator`) and
records nothing. `record-operator-decision` records what the operator decided or did. It takes the
session that records it (`--recorded-by`) and the operator's words the decision rests on
(`--operator-words`), and `decision show` prints both. It refuses an agent actor
(`ActorNotHuman`). `decision answer-escalated-as-operator` records the operator's decision and
answers the escalated request in one command.

The guard denies every guarded session a forge write of a credential, whoever authorized it, and
hands the operator the command to run ([the guard](/docs/security/the-guard)).

### Handled in code

`conductor decision handle --request <file>` takes one `[DECISION-REQUEST]` whole. Rules over its
question and lettered options pick the class: `data-loss`, `history-rewrite`,
`repository-lifecycle` and `architecture` are class O. `credential` and `hands` are class H. No
match is class C. Class C is answered through the `Decider` adapter, `claude` by default or
`fake`, which answers the recommendation. The answer is recorded with the adapter's name and the
prompt's SHA-256, and the `[DECISION]` line is printed. An answer that names no option is not
recorded: the request is escalated as class O. Class O and H are escalated with the request's
options, its recommendation as the default, and a deadline 48 hours away.

## Answers and reversals

| who | answers | reverses |
|---|---|---|
| conductor | an open request (`decision answer-request`) | its own decisions only (`decision reverse-conductor-decision`) |
| the operator | an escalated request (`decision answer-escalated-request`) | any decision (`decision reverse-operator-decision`) |

A decision id is `DEC-YYYYMMDD-NN`: the UTC date of `--decided-at` and a number. Given no
`--decision-id`, the record commands allocate the next number for that date. Each command that
records an outcome appends one line to `decisions/YYYY-MM.jsonl` in the records
([records and store](./records-and-store.md)).

## Authority

Conductor decides class C on the operator's standing grant, `authority.class_c`. The instance's
`rules.md` says where the operator wrote that grant and how the operator decides, so conductor can
pick the option the operator would. Every class-C decision conductor sends carries its decision
id. One without an id is a recommendation.

The controller profile tells a controller to act on a `[DECISION …]` with an id as the operator's,
within conductor's grant. Where the operator's own harness instructions make a controller ask the
operator instead, they need the same grant, or the loop stalls on routine approvals.

## Messages

A message's first line is machine-readable. The rest is free text, at most 20 lines. Detail goes in
a file in the sender's own repository, and the message carries its path.

```console
conductor message send --to conductor --from <session name> --text -
```

`message send` reads the message on standard input. A first line that does not parse is answered
with the format and exit 1, and nothing is written. Otherwise the whole text is left as
`<state>/mailbox/conductor/<ulid>.md` and recorded received, transport `Mailbox`. `conductor run`
reads the mailbox every watch tick. It puts its own messages in front of a controller with
`conductor session deliver`, which types into the session's tmux server through
`<state>/sessions/<name>.tmux` and refuses a session without that socket.

| first line | from → to | meaning |
|---|---|---|
| `[DISPATCH <repo> <dispatch-id>] <goal id> <what to do>` | conductor → controller | a brief |
| `[REPORT <repo> <dispatch-id>] <started\|progress\|blocked\|unblocked\|done\|failed> <one line>` | controller → conductor | progress on a dispatch |
| `[DECISION-REQUEST <repo> <request-id>] <question>` | controller → conductor | options A/B/C and a recommendation below |
| `[DECISION <request-id>] <option> <one-line reason> (<decision-id>)` | conductor → controller | the answer |
| `[NEED <repo> <request-id>] <other-repo> <what is needed>` | controller → conductor | a cross-repository request, delivered to the other repository's running controller or kept for its next start |
| `[HANDOVER <repo>] context <n>k` | conductor → controller | the context is over `thresholds.context_handover`: the controller finishes its step, writes `<checkout>/.git/handoff/<UTC stamp>.md`, reports `blocked handover <path>` and stops; `conductor run` starts it again from its charter and that file |
| `[RESOURCE <repo>] <build-slot\|disk\|usage> <amount>` | controller → conductor | answered "use conductor build run" ([disk](./disk.md)); `conductor run` resumes the sessions a usage limit stopped |
| `[ESCALATION <request-id>] <O\|H> <question>` | conductor → operator | class O or H |

Conductor records every message it receives (`message receive-message`, which `message send`
records through). One whose first line does not parse is rejected (`message reject-message`),
answered with the format, and not acted on.

Each `REPORT` verb but `progress` moves the dispatch through its `dispatch report-*` command. A
`progress` report is a message only.

A decision request also lands in the controller's own planning store as a decision blocker, where
the repository has one. The controller clears the blocker stating what was decided, with no
reference to conductor. Conductor records each request (`decision raise-decision-request`, with
the blocker reference). The blockers collector is the cross-check: an open blocker with no recorded
request is a controller that did not report.

The resource commands (`resource grant-resource`, `refuse-resource`, `release-resource`) remain, and
`conductor run` calls none of them.

:::note[Retired]

`[RESTART conductor] <reason>` is in the design but not in the specification, and does not parse.
`conductor run` keeps no context. conductord restarts an instance's runtime with no hand-over: on a
changed placement or binary, and when it stops running ([the host](./the-host.md#conductord)).

:::
