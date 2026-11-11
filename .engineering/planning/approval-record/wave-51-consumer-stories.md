---
format: aep.planning-md/3
id: approval-record:wave-51-consumer-stories
kind: approval-record
status: draft
title: The operator asked that wave 51 serve a consumer control plane's asks
relations:
- decides: story:release-container-image
- decides: story:daemon-probes-and-metrics
- decides: story:container-start-without-prompt
revision: 1
---
## The question

Which stories wave 51 holds. The plan's next wave (`aep plan artifact waves --kind story --status
active`, wave 1) holds no story a consumer control plane asked for; the three stories it asked
for first on 2026-11-10 (`epic:deployable-in-kubernetes`: a published image, probes and metrics,
a start with no prompt) were drafts.

## The decision

The operator, 2026-11-11, to the coordinating session, after it reported that `main` was 42
commits ahead of v1.1.0: "cut a release now, and then dispatch the next wave. And make sure that
wave somewhat serves the [consumer control plane's] calls". The bracket replaces the system's
name, which this store does not hold.

The coordinating session read that as naming, for this wave, the three stories that system asked
for first: `story:release-container-image`, `story:daemon-probes-and-metrics` and
`story:container-start-without-prompt`. The reading is the coordinator's; the operator named no
story id.

## What follows

The three stories move `draft -> proposed -> active` on this record and join wave 51 beside the
four stories of the plan's wave 1 that can run. `story:control-plane-work-handler` stays a
draft: the consumer's session answered on 2026-11-11 that its contract is not frozen and that a
handler cannot sign in on its side yet. `story:cluster-install` stays a draft.
