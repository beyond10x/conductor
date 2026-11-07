---
format: aep.planning-md/3
id: story:spawn-without-dispatch-reports
kind: story
status: active
title: A controller spawned with no dispatch reports without a made-up id
summary: made-up dispatch ids are dropped by the loop and the sender is not told
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:11:39Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T13:11:39Z", actor: "human:timo", revision: 4}
---
## Why

Observed 2026-11-07 on a running instance: controllers were started by hand with `conductor
spawn`, with no dispatch, and the loop was then restarted. The run unit's journal held 8 lines
`mailbox <file>: dispatch report-started|report-blocked: DispatchNotFound: no dispatch has the
id DSP-01a1b431` (also `DSP-1` and `DSP-01a1b431-4a48`): the controllers made up dispatch ids.

Verified in the code: `conductor spawn` records a controller and a `sent` line, and no dispatch
in the store (`crates/conductor/src/spawn.rs`, no dispatch id); the controller profile writes
`[REPORT <repo> <dispatch-id>]` and never names the form `-`
(`.agents/repo-controller.md:11`, `:66`), though the loop takes `-` as the repository's open
dispatch (`crates/conductor/src/run/mailbox.rs:120`). A refused report is only logged:
`tick.note(&format!("mailbox {name}"), &error)` (`run/mailbox.rs:81`), and the file is marked
handled; the sender is not told. Related: `story:mailbox-format-refusal-reaches-sender`, the same
silent drop for a request refused by format.

## Acceptance

- A controller started by `conductor spawn` with no dispatch is either given a real dispatch id in
  its charter, or its profile says to report with `-` when it holds none, and a report with `-`
  from it moves nothing and is not refused.
- A report the loop refuses with `DispatchNotFound` reaches its sender as one message naming the
  id and the open dispatch of its repository (or that there is none), within one cycle.
