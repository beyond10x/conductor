---
format: aep.planning-md/3
id: story:doctor-refusal-wording
kind: story
status: draft
title: The missing-program refusal reads a snapshot's collectors start it
relations:
- decomposes: epic:operating-loop-hardening
scope:
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/snapshot.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: website/docs/getting-started.md
- confidence: cited
  path: website/docs/guides/troubleshoot.md
revision: 2
---
## Why

When a program a snapshot's collectors need is missing, `start-snapshot` refuses with "`aep` is
not on PATH, and a snapshot's collectors starts it". The verb does not agree with its plural
subject: the message is built in `crates/conductor/src/doctor.rs:77` ("... and {what} starts
...") from the subject "a snapshot's collectors" passed at `crates/conductor/src/snapshot.rs:599`.
The website quotes the wrong text in `website/docs/getting-started.md:144` and
`website/docs/guides/troubleshoot.md:53`. Found by the docs overhaul, 2026-10-12; split out of
story:session-tasks-follow-prefix after the plan critics' round 1.

## Design

The verb follows the subject: `doctor::require` takes the verb form with the subject (or the
caller passes a subject that reads with "start"), so the refusal reads "a snapshot's collectors
start it". The two website quotes are updated to the same text.

## Acceptance

- The refusal test in `crates/conductor/tests/doctor.rs`
  (`start_snapshot_refuses_at_start_naming_a_missing_program`) asserts the refusal text holds "a
  snapshot's collectors start it".

## Scope

`crates/conductor/src/doctor.rs`, `crates/conductor/src/snapshot.rs`,
`crates/conductor/tests/doctor.rs`, `website/docs/getting-started.md`,
`website/docs/guides/troubleshoot.md`.
