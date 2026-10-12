---
format: aep.planning-md/3
id: story:guard-expanded-conductor-words
kind: story
status: draft
title: A conductor run whose group or program word the shell expands is denied to a controller
relations:
- decomposes: epic:operating-loop-hardening
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
revision: 3
---
## Why

The security review of `story:guard-store-migrate` (wave 06) found that the guard checks the leaf
word of a `conductor` run strictly but not the group word or the program word: when the shell
expands them (braces, `$"…"`, `${…}`, `$(…)`), or when the leaf reaches the program through `xargs`
or a function's `"$@"`, a controller's line that runs a writing leaf is allowed. Nine such lines
(`conductor {store,migrate}`, `$(command -v conductor) store migrate`, `echo store migrate | xargs
conductor`, …) were traced through `crates/conductor/src/guard/rules.rs` (the `conductor_write`
check around line 1379). The gap predates wave 06.

## Acceptance

- `a_controller_reaches_no_writing_leaf_through_a_word_the_shell_expands` in
  `crates/conductor/tests/guard_rules.rs`, committed in wave 06 with
  `#[ignore = "story:guard-expanded-conductor-words: …"]`, runs without the ignore and passes:
  each of its nine lines is denied to a controller.

## Notes

The review's suggested fix: deny when a word after a `conductor` program word holds a shell
expansion character (`$ { \` * ? [`), and treat a program word that is a substitution, and `xargs`
or `"$@"` handing words to `conductor`, as unknown. A guard change that tightens a rule; it gets a
security review again.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_rules.rs`.
