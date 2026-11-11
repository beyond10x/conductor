---
format: aep.planning-md/3
id: story:need-repository-from-tag
kind: story
status: active
title: A NEED's repository comes from its tag, without trailing punctuation
scope:
- confidence: inferred
  path: crates/conductor/roles/controller.md
- confidence: inferred
  path: crates/conductor/src/message.rs
- confidence: cited
  path: crates/conductor/src/run/mailbox.rs
- confidence: inferred
  path: crates/conductor/tests/dispatch.rs
- confidence: inferred
  path: crates/conductor/tests/followup_w06.rs
- confidence: inferred
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: inferred
  path: docs/design/conductor.md
- confidence: inferred
  path: website/docs/concepts/decisions-and-messages.md
revision: 21
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:05Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:06Z", actor: "human:timo", revision: 7}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from conductor 1.0.1, a trial instance. A NEED whose
text reads `[NEED <repo> <id>] docs: …` or `… specs, …` is stored with the repository `docs:`,
`e2e-tests:` or `specs,`, punctuation attached; it matches no controller and stays in
`<state>/run/pending.json` (4 of 7 items there). That the parser causes it is inferred from the
stored names; the source was not read. Detail: the reporter's defect list item 11.

## Acceptance

- A NEED's repository is read from its `[NEED <repo> <id>]` tag, never from the free text after
  it, and a repository name never keeps trailing punctuation; a NEED naming no known repository is
  refused in one line instead of pending; tests hold each case.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor/src/run/mailbox.rs`, the run loop's mailbox tick — cited
- **Files:** `crates/conductor/src/run/mailbox.rs`, `crates/conductor/tests/run.rs` — cited, the defect site is `mailbox.rs:331-334` (`rest.split_whitespace().next()`, the first word of the free text taken as the repository), and the existing tick test of a mailbox NEED is `run.rs:1954-1978`
- **Symbols:** `need`, `act`, `answer_refused`, `FORMS` in `run/mailbox.rs`; `deliver_or_keep` and `PENDING` in `run/handlers.rs` — cited
- **Also likely:** `crates/conductor/src/message.rs`, `crates/conductor/roles/controller.md`, `crates/conductor/tests/profiles.rs`, `crates/conductor/tests/dispatch.rs`, `crates/conductor/tests/followup_w06.rs` — inferred, touched only if the first-line form `[NEED <repo> <request-id>] <other-repo> <what>` changes: `message.rs:76` and `:148-153` hold the form and its rule, `controller.md:68` teaches it, and the three tests assert its text
- **Documents:** `docs/design/conductor.md`, `website/docs/concepts/decisions-and-messages.md` — inferred, both print the NEED form (`conductor.md:200`, `decisions-and-messages.md:91`) and change only if the form does
- **Confidence:** medium — the tree shows the defect site exactly, but the acceptance ("read from its tag, never from the free text") contradicts the documented form, where the tag's `<repo>` is the sender and the target is the first word after `]`; which way the implementor resolves that decides whether the Also likely files are touched
- **Would collide with:** any unit touching the mailbox tick in `crates/conductor/src/run/mailbox.rs`, or adding tests to `crates/conductor/tests/run.rs`; if the form changes, any unit touching the first-line grammar in `crates/conductor/src/message.rs` or the controller role text
- **Safety fact:** a refusal needs no new channel: an `Err` from `need()` is already answered to the sender as one `[REPORT <repo> -] refused: <reason>` message and the file is marked handled, so nothing reaches `pending.json` — step 2 (`crates/conductor/src/run/mailbox.rs:86-91` and `:150-173`), unproven

## Decision (wave 51)

The first-line form stays `[NEED <repo> <request-id>] <other-repo> <what is needed>`
(`crates/conductor/src/message.rs`): the tag's `<repo>` is the sender, and the repository the
NEED is for is the `<other-repo>` field. The acceptance's "read from its tag, never from the free
text" is met by reading `<other-repo>` as a field of that form: its leading run of name
characters is the repository, so trailing punctuation is never part of it. A NEED whose
`<other-repo>` is no repository the instance knows is refused in one line to its sender. The
coordinator of wave 51 took this reading; the form's text does not change.
