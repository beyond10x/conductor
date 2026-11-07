---
format: aep.planning-md/3
id: story:report-on-sent-dispatch
kind: story
status: draft
title: A blocked or done report on a sent dispatch is not dropped
summary: a blocked report on a Sent dispatch is refused and the sender never learns it
revision: 2
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
