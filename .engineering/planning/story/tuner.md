---
format: aep.planning-md/3
id: story:tuner
kind: story
status: draft
title: a tuner host role proposes one settings change at a time inside a bounded grant and measures it
summary: settings-proposal records; globals.tuning ranges; keep or revert after the window
relations:
- decomposes: epic:architecture-next
- depends_on: story:config-globals
- depends_on: story:federated-views
revision: 1
---
## Why

Operator, 2026-11-07: self-improvement that "would try to tweak the conductor settings, the instance settings". The public page's "Self-improvement over settings"; design §7.

## Acceptance

- After the daily tick, the `tuner` role reads the KPI readings and open findings and writes one `settings-proposal` record (key, old, new, the reading it should move, the window).
- When the key is in `globals.tuning` and the value inside its range, it applies the change through `PUT /v1/config` and, after the window, keeps it on an improved reading or reverts it, recording both; outside the grant it files a finding.
- The daily brief lists every proposal, kept or reverted, with the readings; with an empty grant nothing is applied (a test).
