---
format: aep.planning-md/3
id: story:mailbox
kind: story
status: draft
title: The harness-neutral mailbox
revision: 1
---
# The harness-neutral mailbox

## What

Codex sessions cannot receive Claude's SendMessage, and the operator relayed hand-offs between
them by hand 8 times in one week (design § 5.2, P6). Messages of record therefore travel as files.

**Spec first.** `spec/domains/dispatch.yaml` gains:
- `MailItem`, the record of one **outbound** message: `recipient`, `kind`, `first_line`, `path`,
  lifecycle `Queued → Delivered`. Inbound messages stay `Message` records created by
  `ReceiveMessage`, so the two never describe the same message.
- `PostMail`, which writes one file under `state/mail/<repo>/out/` and records it `Queued`.
- `MarkDelivered`, which moves an item to `Delivered` when the recipient has collected it.
- `CollectMail`, which reads `state/mail/<repo>/in/` and records each file through `ReceiveMessage`,
  with `parsed` as `story:spec-review-closure` defines it.
- `MailItems`, a view.

`spec/components.yaml` places them in a new `mail` group, the tenth.

The specification is validated, the model crate regenerated, and only then implemented in
`crates/conductor/src/mail.rs`. This is the one story after `story:cli-from-spec` that edits
`cli.rs` and `tests/cli_tree.rs`, to add the `mail` group; it runs alone in its wave on those files.
It also updates the group count in design § 5.9 (currently "It has 9"), which
`story:spec-review-closure` edits first; the `depends_on` edge orders the two.

SendMessage becomes a wake-up for Claude controllers. A Codex controller collects its inbox at wave
boundaries.

Depends on `story:pilot` and `story:spec-review-closure`.

## Scope

- `crates/conductor-model`
- `crates/conductor/src/cli.rs`
- `crates/conductor/src/lib.rs`
- `crates/conductor/src/mail.rs`
- `crates/conductor/tests/cli_tree.rs`
- `crates/conductor/tests/mail.rs`
- `docs/design/conductor.md`
- `spec/components.yaml`
- `spec/domains/dispatch.yaml`

## Acceptance

1. `ess specify validate --path spec` exits 0.
2. `ess verify conform run --path spec --target interpreted --report-format 2 --report-out <scratch>/run.json`
   prints `passed`.
3. `task check` exits 0, including the regeneration drift step.
4. `conductor --help` lists `mail` among 10 groups, and `grep -n "It has 10" docs/design/conductor.md`
   prints the updated § 5.9 line.
5. In an integration test:
   1. `mail post-mail` for repository `x` writes exactly one file under `state/mail/x/out/`, whose
      first line is the § 5.5 line it was given, and `mail mail-items` lists it `Queued`.
   2. `mail mark-delivered` on it makes `mail mail-items` list it `Delivered`.
   3. A file `state/mail/conductor/in/1.md` whose first line is
      `[REPORT repo-a <dispatch id>] done gate green` is recorded by `mail collect-mail` as a
      `Message` with kind `Report`, sender `repo-a` and transport `Mailbox`.
   4. A second collect records nothing new.
