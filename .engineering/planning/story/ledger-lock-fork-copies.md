---
format: aep.planning-md/3
id: story:ledger-lock-fork-copies
kind: story
status: draft
title: The ledger lock does not loop on descriptor copies held by forked children
revision: 2
---
# Story: ledger-lock-fork-copies

## Outcome

`Ledger::lock` does not loop when a forked child briefly holds copies of the ledger's lock descriptors.

## Context

Found while fixing story:build-run-load-races (a6588d4f), 2026-10-16. When a lock wait ends, `Ledger::lock` checks the store lock again at once and backs off if it looks held. Between fork and exec, a child of the same process holds copies of every open descriptor. Under injected fork noise (children that wait 100 ms before exec), a run looped there for over 20 s in 3 of 15 runs, and then in 10 of 15.

Rust opens files with `O_CLOEXEC`, so a real child holds the copies only for the microseconds between fork and exec. Production `build run` seldom forks from two threads at once. Decided (conductor-dev, principle 3): no change to the lock protocol now. This story is kept for the time a measured loop happens outside a test.

## Acceptance

1. A loop of `Ledger::lock` longer than 5 s is logged with the holder's pid, so a real occurrence is measured.
2. If one is measured, the lock uses OFD locks (`F_OFD_SETLK`), which fork does not share, and a test with fork noise holds it.

## Out of Scope

The test-side fixes (a6588d4f).

## Ambiguities

None.
