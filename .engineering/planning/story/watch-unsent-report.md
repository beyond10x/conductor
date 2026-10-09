---
format: aep.planning-md/3
id: story:watch-unsent-report
kind: story
status: draft
title: The watch reports a controller that printed a report and never sent it
tags:
- session-review
relations:
- informed_by: story:controller-report-sendmessage
scope:
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/watch.rs
revision: 2
---
## Why

A controller that prints its report instead of sending it looks idle and finished to everyone but
itself (`story:controller-report-sendmessage` has the evidence). The profile rule lowers the rate;
the watch catches the rest.

## Acceptance

- The watch prints `report not sent: <session name> <id> <first line>` once per session and report,
  when a controller session's newest assistant text starts with `[REPORT`, `[DECISION-REQUEST`,
  `[NEED` or `[RESOURCE` and no `SendMessage` tool call follows it in the transcript.
- Tests in `crates/conductor/tests/watch.rs` with fixture transcripts: a printed report and no call
  prints the line once; a printed report followed by a `SendMessage` call prints nothing; a second
  run over the same transcript prints nothing.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`. Shares both files with
`story:watch-silent-controller`; the second of the two to land merges after the first.
