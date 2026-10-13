---
format: aep.planning-md/3
id: story:rules-show
kind: story
status: draft
title: conductor rules show prints rules.md items verbatim by number or section
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/rules.rs
- confidence: cited
  path: crates/conductor/tests/fixtures/rules
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/rules.rs
- confidence: cited
  path: spec/domains/direction.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

A rule obliges every worker brief to carry seven rules.md items in full. On 2026-10-13 controllers
copied them out with `awk` and line-number ranges (`NR>=338 && NR<=342`) from a file that changed
twice that day, so an excerpt could cut the wrong lines; after each rules.md change they re-briefed
running workers by hand, four messages in one controller. Conductor itself read the whole 650-line
rules.md at every start to find its own section.

## Acceptance

- Spec first: a `Rules` view (item number, title, section, source, text) over the instance's
  rules.md is declared in `spec/domains/direction.yaml`; `ess specify validate --path spec` exits
  0.
- `conductor rules show --item 43,49,51` prints those items verbatim, byte for byte as in rules.md,
  wherever they sit in the file; `--section <heading>` prints every item under that heading; an
  item number that does not exist exits 1 and names it. Tests in `crates/conductor/tests/rules.rs`
  over a fixture rules.md, including one whose items are out of numeric order.
- The guard allows `conductor rules show` to a controller (a view); a case in
  `crates/conductor/tests/guard_rules.rs` holds it.
- `.agents/repo-controller.md` says a worker brief cites the items with the output of `conductor
  rules show --item …`, never a hand excerpt; `crates/conductor/tests/profiles.rs` holds the
  sentence.

## Notes

- After `story:guard-write-targets` on the guard files; before `story:conductor-start-reading`,
  which names this command in conductor's start.
- Reads rules.md only; it writes nothing and opens no store.

## Scope

`spec/domains/direction.yaml`, `crates/conductor-model`, `crates/conductor/src/rules.rs` (new),
`crates/conductor/src/lib.rs`, `crates/conductor/src/cli.rs`, `crates/conductor/cli.yaml`,
`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/rules.rs` (new),
`crates/conductor/tests/fixtures/rules`, `crates/conductor/tests/guard_rules.rs`,
`.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`,
`website/docs/reference/cli.md`.
