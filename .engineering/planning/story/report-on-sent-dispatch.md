---
format: aep.planning-md/3
id: story:report-on-sent-dispatch
kind: story
status: active
title: A blocked or done report on a sent dispatch is not dropped
summary: a blocked report on a Sent dispatch is refused and the sender never learns it
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:11:39Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T13:11:39Z", actor: "human:timo", revision: 4}
---
## Why

Observed 2026-11-07 on an instance: controllers given a dispatch while already working reported
`blocked` first, on a dispatch still `Sent`; the loop refused it with `DispatchStateConflict: …
only a started dispatch blocks` (two dispatches, 12:05:23Z) and wrote it to its journal only, so
the controllers were not told and the dispatches stayed `Sent`.

## Acceptance

- A `blocked` or `done` report on a `Sent` dispatch moves it through `Started` first and then to
  the state reported, with both lines in the dispatch log.
- A report the loop still refuses reaches its sender with the reason and the expected form
  (story:mailbox-format-refusal-reaches-sender).

## Started on a Blocked dispatch (added 2026-11-07)

From the host supervisor, 2026-11-07: a controller restarted after a `blocked handover` report sent `started`. The loop refused it (run journal 13:49:39Z: `dispatch report-started: DispatchStateConflict: dispatch <id> did not move`), and the dispatch stayed Blocked from 13:36Z to 15:33Z while the controller worked, until `dispatch report-unblocked` was run by hand.

- A `started` report on a Blocked dispatch moves it to Started (recorded as unblocked), and a test covers it; a `started` report on a Done dispatch is still refused, and its sender is told.
