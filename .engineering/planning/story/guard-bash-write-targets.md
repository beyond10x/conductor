---
format: aep.planning-md/3
id: story:guard-bash-write-targets
kind: story
status: archived
title: The guard checks where a Bash write form writes, like a Write
tags:
- session-review
relations:
- depends_on: story:guard-write-forms-and-config-scope
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-10-08T22:26:55Z", actor: "human:timo", revision: 2}
---
## Why

Observed in a live instance: the guard denied conductor's Write of
`<records>/docs/analysis/<date>/reassessment.md` ("not one of conductor's records"), and a Bash
`cp` then wrote `<records>/docs/analysis/<date>/census.tsv` with no denial. The guard checks where a
file tool writes and does not check where a Bash write form writes (design § 7). The records'
`docs/analysis/` is where conductor keeps its own dated evidence, so the Write denial was wrong too.

## Acceptance

- For the Bash write forms whose target is a plain word on the command line — a redirect `>`/`>>`
  outside quotes, `tee <file>`, `cp <src>… <dst>`, `mv <src>… <dst>`, `install … <dst>`,
  `sed -i <file>` — the guard resolves the target like a Write path and applies the session's Write
  rule; a target it cannot resolve (a variable, a substitution) keeps today's behaviour.
- Conductor may write `<records>/docs/analysis/**` with file tools and these Bash forms.
- Tests in `crates/conductor/tests/guard_rules.rs`: each form denied for a controller writing into
  another checkout and into the records; allowed into its own checkout and scratch; conductor's
  `docs/analysis` write allowed; a target in a variable unchanged.
- Design § 7 and README Security model say which Bash forms are checked; the general statement
  that a command built to get past the guard does so stays.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/src/guard/shell.rs`,
`crates/conductor/tests/guard_rules.rs`, `docs/design/conductor.md`, `README.md`.
After `guard-write-forms-and-config-scope` (same files).
