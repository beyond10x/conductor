---
format: aep.planning-md/3
id: story:reserve-from-peak
kind: story
status: implemented
title: A build reserves its command's measured peak, not a guess
relations:
- decomposes: epic:disk-protocol
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T20:30:36Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-10-16T20:30:37Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-10-16T20:30:37Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
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
- The guard's denial of an unreserved build (`BUILD_RESERVE`, `crates/conductor/src/guard/rules.rs`)
  proposes `--reserve auto`, or the size `auto` would pick for that repository when the guard can
  read it from the host ledger without opening the store, never a fixed 8G; and a `build run`
  refused for disk names the repository's measured peak beside the shortfall. Source: conductor's
  NEED (2026-10-14): aep and connectors asked the guard's 8G and were refused "8G
  short" at 23-25G free while their ledger peaks were 0.49-0.69G and 0.2-2.1G.
- The recorded peak is a real measurement (a conductor NEED of 2026-10-16: `peak_bytes` 0 for connectors' journey builds). The size walk runs on its own thread with a deadline of at least 60 s, not one watchdog interval (`build.rs:1137`), and keeps its last complete reading when a walk times out. `--measure <dir>` (repeatable) names what a script builds outside `<cwd>/target`, and `CARGO_TARGET_DIR`, when set, is measured. A peak never measured is recorded as unknown, not 0, and `auto` ignores it. Tests cover a slow walk, a script building into `--measure`, and an unknown peak.

## Scope

`crates/conductor/src/build.rs`, `crates/conductor/src/host.rs`, `crates/conductor/src/cli.rs`,
`crates/conductor/tests/build_run.rs`, `.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`.
