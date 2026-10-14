---
format: aep.planning-md/3
id: story:retro-report
kind: story
status: draft
title: '`conductor retro` builds a short report of a user''s sessions from the KPI instruments'
relations:
- decomposes: epic:measured-self-improvement
- depends_on: story:retro-spec
revision: 2
---
## Why

`docs/design/user-retro.md` (approved 2026-10-13): `conductor retro` builds the report from the KPI instruments.

## Acceptance

- `conductor retro --since 2h` over fixture transcripts prints the five sections in order, each KPI with its id and only those that apply (T1–T4 per merged PR, K4/K5 for those PRs, wall-clock duration, K3, K6–K11, security);
- the YAML front matter parses back to the same numbers;
- no transcript text appears in the report: a test seeds a unique string into a fixture transcript and asserts it is absent.

## Scope

`crates/conductor/src/retro.rs` (new), `crates/conductor/tests/retro.rs`.
