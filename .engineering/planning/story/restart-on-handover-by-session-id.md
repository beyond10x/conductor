---
format: aep.planning-md/3
id: story:restart-on-handover-by-session-id
kind: story
status: draft
title: The loop restarts at the hand-over report and acts on session ids, not names
summary: A 15-minute gap after a hand-over, and a respawned successor at risk of being stopped by name
revision: 3
---
## Why

The host supervisor, 2026-11-07: after a controller reports `blocked handover`, the loop waits up to 15 minutes before it restarts the controller (`story:loop-restarts-controller-at-handover`, whose acceptance waits for an idle session or `thresholds.silent_session` and does not restart on the report itself); and when a session was respawned after a gone one, a restart keyed by the session name can stop the live successor (`story:loop-restarts-gone-conductor-dev`).

Evidence: one controller instance, 2026-11-07 23:12 to 23:21 CET: two sessions held one dispatch after a respawn (host supervisor's report, 23:24 CET; host teacher, 23:2x CET).

Evidence, 2026-11-08 (host supervisor): after the loop's hand-over restart (journal 06:52:15 `stopped beyond10x-atlas (da034acc)`), the new session 54300b8d read the hand-off and sat idle at 06:53 "waiting for the next dispatch"; its open dispatch stayed `Blocked`. The supervisor's hand-run restart script delivers the open dispatch id with the hand-off path and unblocks it; the loop's `restart_with` does neither.

## Acceptance

- The loop restarts a controller on its `blocked handover` report at the next watch tick, with no further wait, once `test -s <path>` holds for the named hand-over.
- A restart or stop acts on the session id the loop recorded for the gone session, never on a name: a live session with the same name and another id (started by a later spawn) is left running and named in the journal.
- A restart delivers to the new session the hand-over path and the id of the repository's open dispatch (`Started` or `Blocked`), and moves a `Blocked` one back to `Started` (`report-unblocked`), so the successor resumes that dispatch instead of waiting for the next.
- Tests over fixtures: a hand-over report is followed by one restart within one tick; a successor with the same name and a new id is not stopped; a restart with a `Blocked` dispatch delivers its id and leaves it `Started`.
