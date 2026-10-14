---
format: aep.planning-md/3
id: story:reserve-from-peak
kind: story
status: draft
title: A build reserves its command's measured peak, not a guess
relations:
- decomposes: epic:disk-protocol
revision: 1
---
## Why

On 2026-10-14 most host reservations were 3-8G while released runs of the same commands peaked at
0.4-1.8G, so a 38G disk with a 25G floor admitted one build at a time and 28 waited. How much to
reserve was left to each brief (rules item 103).

## Acceptance

- `conductor build run` without `--reserve`, or with `--reserve auto`, reserves the largest
  recorded peak of the same repository and command (its first two words and the package flags) in
  the last 14 days plus 50 %, at least 1G, and the given size or the built-in default when none is
  recorded; it prints the size and its source. A `--reserve` over twice that peak prints a warning
  naming the peak. Tests over a ledger fixture with recorded peaks hold both.
- `.agents/repo-controller.md` says builds use `--reserve auto` unless a size is known to be larger;
  a profile test holds the sentence.

## Scope

`crates/conductor/src/build.rs`, `crates/conductor/src/host.rs`, `crates/conductor/src/cli.rs`,
`crates/conductor/tests/build_run.rs`, `.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`.
