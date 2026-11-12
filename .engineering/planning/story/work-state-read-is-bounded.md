---
format: aep.planning-md/3
id: story:work-state-read-is-bounded
kind: story
status: draft
title: Reading a unit of work's state does not read the whole dispatch log
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

The security review of the work call (`review-result:work-call-security-round-1`, its last
note) read `dispatch_lines` in `crates/conductor/src/dispatch.rs`: each `GET
/v1/work/{reference}`, and each `POST` that answers a row, reads every month file of the
dispatch log from the month of the dispatch's `sent_at` on, whole, into memory, parses every
line and keeps those of one dispatch. It takes no lock and has no cap. Over HTTP `sent_at` is
the time of the call, so a dispatch sent this month costs one read of this month's file; a
dispatch sent months ago and still open costs every file since. A control plane that reads the
state of each open unit on a timer pays that per unit and per read.

Not measured: the size of a month file on an instance in use, and what one read costs there. A
development host's file of one month read 2 MB on 2026-11-12.

The same whole-file read is in the other log readers (`records.rs`, `run/records.rs`); this
story is about the one a remote caller can repeat at will.

## Acceptance

- A test with a dispatch log of a stated size (lines of many dispatches over several months)
  reads one dispatch's state and states what it read: the read is bounded by a number the test
  names (bytes read, or files opened), not by the size of the log.
- A row's `started_at`, `ended_at`, `detail` and `links` are what they were before, for a
  dispatch whose lines sit in more than one month file.
- A log line appended while a read runs is either in the answer or not; no partial line is
  parsed as one.

## Notes

- Pre-existing pattern, reached over HTTP by the work call: it waits in the backlog.
- Candidates, not decided: keep the row's times and detail in the dispatch record at each
  transition, so the read opens no log file; or an index of the dispatch's lines by file and
  offset.
