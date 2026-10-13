---
format: aep.planning-md/3
id: story:retro-spec
kind: story
status: draft
title: The retro report and its submission are specified first
relations:
- decomposes: epic:measured-self-improvement
revision: 2
---
## Why

`docs/design/user-retro.md` (approved 2026-10-13): the retro report and its submission are new nouns of the CLI.

## Acceptance

- `spec/` declares the report `conductor-retro/1` (outcome, KPIs by id, friction signatures, versions, the two free-text answers, the author's login), the `retro` command and its outcomes (written, refused: nothing to report, sent, not sent: not confirmed, refused: public target); `ess specify validate --path spec` exits 0.

## Scope

`spec/`, `crates/conductor/cli.yaml`.
