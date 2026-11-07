---
format: aep.planning-md/3
id: story:loop-restarts-gone-conductor-dev
kind: story
status: active
title: The loop restarts a gone conductor-dev while the plan has active stories
summary: a gone conductor-dev stayed gone 1.5 h with four stories active
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T12:41:39Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T12:41:39Z", actor: "human:timo", revision: 3}
---
## Why

Observed 2026-11-07 on one instance: the loop's journal logged `session gone` for the instance's
conductor-dev session at 07:34Z, and no conductor-dev ran until about 09:10Z, while four stories of
the conductor plan were `active`. The loop restarts a gone repository controller; it does not
restart a gone conductor-dev, so nothing delivered the active stories for about 1.5 hours.

## Acceptance

- When the loop's watch finds the instance's conductor-dev session gone and the product
  repository's plan holds an `active` story, it starts conductor-dev again as it starts a gone
  controller (`conductor session start --role maintainer`, the role's model and settings), from
  its newest hand-over, and logs one line naming the restart.
- With no `active` story it does not start one, and logs nothing new.
- A test over the watch with a gone conductor-dev and one active story reads one start; with none
  active, no start.
