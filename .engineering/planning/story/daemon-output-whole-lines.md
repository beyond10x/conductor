---
format: aep.planning-md/3
id: story:daemon-output-whole-lines
kind: story
status: draft
title: A line of the daemon's own output is written whole
scope:
- confidence: cited
  path: crates/conductor/src/daemon
- confidence: cited
  path: crates/conductor/tests/container_start.rs
revision: 2
---
## Why

`container_start::a_seeding_that_fails_keeps_the_daemon_serving_and_a_later_start_commits_the_records`
(`crates/conductor/tests/container_start.rs:942`) failed once in the hosted check of a push to
`main` whose commit added only planning records; the pull-request run of the same commit passed
it. Whether a rerun of the failed job passed is not recorded.

The case counts the lines of the daemon's output that start with
`conductor: daemon serve: unattended: alpha:` and expects one. In the failed run two lines of
the daemon's output were written into each other:

```
conductor: daemon serve: unattendedtrusted: <dir>/alpha/records
: alpha: the records skeleton in <dir>/alpha/records is not committed; the next start tries again: `git commit` failed: fatal: empty ident name (for <conductor@example-org.example>) not allowed
```

So the line was there and the count read 0. Two writers of the daemon's standard error write
without one lock or without whole lines.

## What

- A line of the daemon's own output is written whole: one write per line, or one lock for the
  writers of that stream.
- The case reads what it reads today.

## Acceptance

- The two writers are named in this story with their `file:line`.
- A case starts the daemon with both writers active and reads every line whole, many times in one
  run.
- The case above is not loosened to accept a broken line.
