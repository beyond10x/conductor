---
format: aep.planning-md/3
id: story:public-text-check
kind: story
status: implemented
title: A test keeps instance ids, home paths and hand-overs out of the public repository
relations:
- decomposes: epic:operating-loop-hardening
scope:
- confidence: cited
  path: crates/conductor/tests/no_organization.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T17:03:48Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-12T17:03:49Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-12T17:27:23Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Why

On 2026-10-12 three conductor-dev hand-overs with session ids, decision ids and host details were
committed under `docs/waves/` of this public repository, and review caught them only after merge;
the history was rewritten. `tests/no_organization.rs` already keeps organization names out of the
product; nothing keeps an instance's internal ids out.

## Design

A test in `crates/conductor/tests/no_organization.rs` (or a sibling file) reads every tracked file
under `docs/`, `website/docs/` (generated pages excluded), `README.md`, `AGENTS.md` and `.agents/`,
and fails naming file and line for each forbidden pattern:

- a decision, dispatch, need or hands id of an instance: `DEC-`, `DSP-`, `N-` or `H-` followed by
  an eight-digit date;
- an absolute home path: a path token that starts with `/home/<name>/`. The match is anchored at
  the start of a path token (start of line, or after whitespace, a quote, a backtick or `(`), so a
  path such as `/workspace/home/.claude` (`docs/analysis/2026-10-09-substrate-sessions.md:57`) does
  not match;
- a file named `handoff-*` under `docs/`.

A documented example id in the product's own format is allowed only inside a fenced code block of
`website/docs/`.

## Acceptance

- The test passes on the tree as it is and fails on a fixture holding one of each forbidden
  pattern (an instance id, a `/home/<name>/` path, a `handoff-*` file).

## Scope

`crates/conductor/tests/no_organization.rs`.
