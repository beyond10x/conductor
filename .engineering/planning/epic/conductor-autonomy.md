---
format: aep.planning-md/3
id: epic:conductor-autonomy
kind: epic
status: draft
title: Conductor keeps itself and its controllers moving without outside help
revision: 1
---
## Why

Between 2026-10-08 and 2026-10-13 the instance's loop depended on the operator or on outside help
more than it should: 33 conductor hand-overs, each re-arming cycles and the watch by hand; a long
gate restarted from its first step four times; a controller idle for hours beside its own gate; no
quick answer to "what is everyone doing"; builds paused for disk until help came from outside;
operator items ageing for days; a news item that had to be corrected; and a 1.5G source build for
every conductor install (a review by another session, 2026-10-13, and the operator's request for
pre-built binaries).

## Outcome

Conductor keeps itself running across restarts, keeps controllers busy beside long gates, answers
status in seconds, reclaims disk before it pauses, never lets an operator item stall, checks what it
announces, and installs from a pre-built binary.

## Stories

`story:self-arming-restart`, `story:status-board`, `story:prebuilt-binaries`,
`story:resumable-gates`, `story:work-beside-long-gate`, `story:claims-cite-source`,
`story:gc-before-pause`, `story:operator-items-age`.
