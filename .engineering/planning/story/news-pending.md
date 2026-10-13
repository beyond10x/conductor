---
format: aep.planning-md/3
id: story:news-pending
kind: story
status: draft
title: Release news is printed verbatim since a recorded mark, on a configured cadence
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/repository.rs
- confidence: cited
  path: crates/conductor/src/shipped.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/shipped.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/direction.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

On 2026-10-13 conductor's release-news job was written again from the hand-over in each new
session, four different prompts in one day, so its behaviour changed at every restart. The
receiving session corrected or trimmed conductor's summaries four times, the last after
`story:claims-cite-source` shipped; conductor then switched to sending release bodies verbatim by
hand. The cutoff of what was already announced travelled in every hand-over.

## Acceptance

- Spec first: a shipped mark (name, instant) with `MarkShipped` and the view's `--since-mark`
  filter is declared in `spec/domains/direction.yaml`, and `cadence.news` in
  `spec/domains/config.yaml`; `ess specify validate --path spec` exits 0.
- `conductor repository shipped --since-mark news --format announcement` prints each release since
  the mark with its link and its body verbatim, and `repository mark-shipped news` moves the mark to
  the newest printed release; a test in `crates/conductor/tests/shipped.rs` reads a body byte for
  byte and, after the mark moves, prints nothing.
- `task conductor:restart`'s prompt arms the news job at `cadence.news` with a fixed instruction
  naming that command and "send the body verbatim"; a test in
  `crates/conductor/tests/taskfile.rs` reads both. `.agents/conductor.md` says the cutoff is the
  mark and never travels in a hand-over; `crates/conductor/tests/profiles.rs` holds it.
- `docs/config.md` documents `cadence.news`.

## Notes

- Shares `Taskfile.yml` and `crates/conductor/tests/taskfile.rs` with `story:watch-waits` and
  `story:conductor-start-reading`; one at a time.

## Scope

`spec/domains/direction.yaml`, `spec/domains/config.yaml`, `crates/conductor-model`,
`crates/conductor/src/shipped.rs`, `crates/conductor/src/repository.rs`,
`crates/conductor/src/cli.rs`, `crates/conductor/cli.yaml`, `crates/conductor/tests/shipped.rs`,
`Taskfile.yml`, `crates/conductor/tests/taskfile.rs`, `.agents/conductor.md`,
`crates/conductor/tests/profiles.rs`, `docs/config.md`, `website/docs/reference/cli.md`.
