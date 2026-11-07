---
format: aep.planning-md/3
id: story:loop-restarts-controller-at-handover
kind: story
status: active
title: The loop asks a controller for its hand-over at the context threshold and restarts it fresh
summary: 11 controllers ran past the context threshold in one day
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:11:40Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:11:40Z", actor: "human:timo", revision: 3}
---
## Why

Observed 2026-11-07 on the host's instances, reported by the host supervisor: 11 controllers ran
past `thresholds.context_handover` that day. Nothing in the loop acts on the threshold: a
controller is expected to notice its own context, write its hand-over and stop, and when it does
not, each later turn re-reads the whole oversized context.

## Acceptance

- At a watch tick the loop reads each live controller's context (the transcript's newest usage
  line); at or above `thresholds.context_handover` it sends the controller one message asking for
  its hand-over, logged once per crossing.
- When the controller has written a hand-over newer than the request and gone idle, or after
  `thresholds.silent_session` with no hand-over, the loop stops it and starts it fresh from its
  charter and the newest hand-over, as it starts a gone controller; one log line names the restart
  and the context it was at.
- A controller under the threshold is never asked; a request already answered is not sent again
  for the same session.
- Tests over the watch with a fake transcript: one over the threshold gets one request, then one
  restart once its hand-over exists; one under it gets neither.
