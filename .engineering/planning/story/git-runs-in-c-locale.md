---
format: aep.planning-md/3
id: story:git-runs-in-c-locale
kind: story
status: draft
title: Git runs in the C locale, so recorded error lines read the same on every host
revision: 2
---
## Why

`crates/conductor/tests/collect_stores.rs:1848` (`a_member_whose_fetch_is_refused_is_read_as_last_fetched`, unchanged since 2026-10-17) expects the recorded error line to start with `` `git fetch --quiet origin` exited 128: fatal: ``. On a host whose locale is German, git prints `Schwerwiegend:`, so the test fails there (2026-11-07, `cargo test --no-fail-fast` on the 0.23.0 integration head: 465 passed, this 1 failed) and passes in CI. The member-failure record carries git's localised text, so a record from such a host reads differently from one written in CI.

## Acceptance

- Every git command conductor runs gets `LC_ALL=C` (or `LANG=C`) in its environment, so its error text is the same on every host; the test passes with `LANG=de_DE.UTF-8` set in its environment.
