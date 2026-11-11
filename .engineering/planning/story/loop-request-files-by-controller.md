---
format: aep.planning-md/3
id: story:loop-request-files-by-controller
kind: story
status: draft
title: The loop's stalled, limited and resumed files are keyed by controller, and its own resume is never taken for a newer session
tags:
- from-finding
revision: 1
---
## Why

Found while unit U2 of wave 51 moved the run loop's hand-over requests from the session name to
the controller's id (the implementor's report, and `review-result:w51-u2-adversary-2`, note N1).

- The loop's other request files, `stalled.json`, `limited.json` and `resumed.json`, are still
  keyed by session name. For hand-over requests that keying let a session replaced under the
  same name inherit the entry of the one before it, which U2 removed; whether the three files
  have the same fault is read from the keying, not measured. This was there before the wave.
- The resume stamp the loop writes is the instant `spawn::resume` returned
  (`crates/conductor/src/run/handlers.rs` about 707). A request is dropped when the listed
  session started after the stamp. Nothing in the tree shows what the harness lists as the
  start of a session it resumed in the background: if that instant lies after the command
  returns, the session the loop itself resumed counts as a newer one, the request is dropped
  and the watch does not ask that session again. No case dates a start after the stamp.

## Acceptance

- `stalled.json`, `limited.json` and `resumed.json` are keyed as the hand-over requests are, and
  an entry of a replaced session is not applied to its successor; a test holds each file.
- A case shows what the harness lists as the start of a resumed session, measured once and
  written down, and the stamp is compared so that the loop's own resume is never taken for a
  newer session.
