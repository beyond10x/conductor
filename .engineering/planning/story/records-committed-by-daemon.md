---
format: aep.planning-md/3
id: story:records-committed-by-daemon
kind: story
status: draft
title: The daily tick commits and pushes the records directory through the bot route; a refusal is one log line
relations:
- decomposes: epic:conductor-run
revision: 1
---
## Why

`conductor run` appends dispatch and decision log lines, writes charters' resume points and
`REPORT.md` in the records directory, and commits nothing (`story:run-daemon`, "Not done"). The
conductor session used to commit its records through the bot route at every hand-over. Records that
are not committed are not records the next host or a reviewer can read.

## Outcome

The daily tick, and `conductor run --once --commit-records`, commit the records directory through
the bot route (`b10x-gates bot -- commit -F <file> -- decisions dispatches charters docs/handoff
REPORT.md STATUS.md`) when it has changes, with a generated message naming the day's counts, and
push when the records repository has an `origin`; a refusal is one `records-commit-refused` line,
never a retry loop.

## Acceptance

1. Spec first: `DispatchLogEvent` gains `RecordsCommitted` and `RecordsCommitRefused`;
   `ess specify validate` exits 0; `task regen` is committed with the spec.
2. `run::commit_records(records, route)` runs `conductor records check` first (a failing check
   refuses the commit with its output in the log line), then the bot-route commit with a message
   `Conductor: records of <UTC day>: <n> dispatch lines, <m> decision lines`, then `push origin
   main` when `origin` exists; one `records-committed` line with the commit id, or one
   `records-commit-refused` line with the route's last line. A fake `b10x-gates` on a temporary
   PATH in the test (`crates/conductor/tests/run_records.rs`): committed, refused, nothing to commit.
3. The daily tick calls it after `metric compute`; `conductor run --once --commit-records` calls
   it alone.

## Scope

`spec/domains/dispatch.yaml`, `crates/conductor-model` (regenerated), `crates/conductor/src/run.rs`,
new `crates/conductor/src/run/records.rs`, `crates/conductor/tests/run_records.rs`,
`website/docs/reference/`.
