---
format: aep.planning-md/3
id: story:daemon-stderr-line-in-one-write
kind: story
status: draft
title: A daemon stderr line of an unattended start is one write
tags:
- from-finding
revision: 1
---
## Outcome

A line the daemon writes to standard error at an unattended start arrives as one line in a capture
that also holds its standard output.

## Why

`container_start::a_seeding_that_fails_keeps_the_daemon_serving_and_a_later_start_commits_the_records`
(`crates/conductor/tests/container_start.rs:897`) failed once in CI on 2026-11-11, on a pull request
that changed only planning files; the same commit's code had passed on the pull request before it
and on `main`. In the captured output the line `conductor: daemon serve: unattended: alpha: …` was
split by a line of standard output (`trusted: …`), so the case's filter on whole lines found none.
Read from that capture: the unattended start writes the line in more than one write, and a write
to the other stream can land between two of them where both streams share one pipe.

## Acceptance

- Each line `daemon serve` writes to standard error during an unattended start is written with one
  write call; a case captures both streams into one pipe while the start prints to both and finds
  every such line whole.
- The case above no longer depends on the two streams not interleaving.
