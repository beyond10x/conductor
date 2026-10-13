---
format: aep.planning-md/3
id: story:spec-first-wording
kind: story
status: draft
title: 'Every profile says: check for ESS specifications and get them green first'
scope:
- confidence: cited
  path: .agents/conductor-dev.md
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 3
---
## Why

The operator, 2026-10-13 (DEC-20261013-49): "any development usually is "spec first" (ess) - so
that means any change we apply, we usually want to check if there are ess specs, and if so, those
need to be modified first and become green first" and "make this clear in the dev profile of
conductor". The profiles' spec-first paragraphs say model, validate, regenerate, implement; they do
not say "check whether specifications exist" nor "green first". The instance's `rules.md` item 44
says it since records 5426281.

## Acceptance

- `.agents/conductor-dev.md`, `.agents/repo-controller.md` (its spec-first paragraph and the brief
  text conductor copies) and `.agents/conductor.md` (the paragraph every `[DISPATCH …]` brief
  carries) each contain the sentence: "Before any change, check whether the repository has ESS
  specifications; if it does, modify them first and get them green (`ess specify validate` and the
  conformance suite they drive) before implementing."; a test in
  `crates/conductor/tests/profiles.rs` finds it once in `.agents/conductor-dev.md`, once in
  `.agents/conductor.md`, and twice in `.agents/repo-controller.md` (its paragraph and its brief
  text).

## Scope

`.agents/conductor-dev.md`, `.agents/repo-controller.md`, `.agents/conductor.md`,
`crates/conductor/tests/profiles.rs`.
