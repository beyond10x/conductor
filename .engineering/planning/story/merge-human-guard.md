---
format: aep.planning-md/3
id: story:merge-human-guard
kind: story
status: draft
title: 'The guard enforces a charter''s merge: human'
relations:
- decomposes: epic:review-first
- depends_on: story:merge-human
revision: 1
---
## Why

`story:merge-human` makes `merge: human` a controller's profile rule. The guard does not enforce
it: it reads no charter (`charter` appears in `crates/conductor/src/guard/rules.rs` only as a
records directory).

## What

The guard reads the charter of the repository a controller session serves (the session's
repository, as the guard already resolves it, and `<records>/charters/<repo>.md`) and, when it says
`merge: human`, denies `gh pr merge`, `glab mr merge`, `gh api … /pulls/<n>/merge` and `glab api …
/merge_requests/<iid>/merge` (any method), and `--auto` merge flags, naming the charter line.

## Acceptance

- `crates/conductor/tests/guard_rules.rs`: with a charter saying `merge: human`, each of those
  commands is denied naming the charter; with `merge: controller` or no line, today's verdict; a
  charter that cannot be read leaves today's verdict and records why.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/src/guard.rs`,
`crates/conductor/tests/guard_rules.rs`.
