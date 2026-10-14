---
format: aep.planning-md/3
id: story:rules-section-names
kind: story
status: implemented
title: A missing rules section is refused with the sections the file has
relations:
- decomposes: epic:loop-without-handwork
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T07:14:22Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T07:14:22Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T07:14:22Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

In an outside user's trial `conductor rules show --section "all sessions"` failed with
`RuleSectionNotFound` on a rules.md whose shared section is named otherwise; the profile tells
conductor to read that section at start, so the start failed on a heading the product assumed.

## Acceptance

- A `--section` the file does not hold is refused naming every section heading the file has, on one
  line of standard error; a test in `crates/conductor/tests/rules.rs` holds it.
- `.agents/conductor.md` reads the shared section only when the file has it: `conductor rules show
  --section "all sessions"`, skipped when it is refused; `crates/conductor/tests/profiles.rs` holds
  the sentence.

## Scope

`crates/conductor/src/rules.rs`, `crates/conductor/tests/rules.rs`, `.agents/conductor.md`,
`crates/conductor/tests/profiles.rs`.
