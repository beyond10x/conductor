---
format: aep.planning-md/3
id: story:goal-docs-config
kind: story
status: draft
title: The direction and documents keys are parsed, validated and shown
relations:
- decomposes: epic:goal-documents
- depends_on: story:goal-docs-spec
revision: 1
---
## Why

`docs/design/goal-documents.md` § D2, D5, approved 2026-10-13.

## Acceptance

- `conductor config validate` refuses an unknown source, both `source` and `url`, a bad kind, by YAML path; `config show` prints them; existing configs load unchanged; tests per refusal.

## Scope

`crates/conductor/src/config.rs`, `crates/conductor-docs` sample, tests, `docs/config.md`.
