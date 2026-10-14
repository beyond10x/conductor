---
format: aep.planning-md/3
id: epic:usage-continuity
kind: epic
status: draft
title: The instance forecasts, conserves and resumes across usage limits by itself
relations:
- decomposes: initiative:self-improvement
revision: 1
---
## Why

On 2026-10-14 every session of the instance stopped on the subscription usage limit until the
operator noticed and said "usage is fixed": both accounts had spent their seven-day window, so the
account rotator (`ais watch`, rotating at 5 % remaining) had nothing to rotate to, and nothing told
any session the limit had lifted. The operator: "think about how you can detect this automatically
in advance and prepare for that and recover from that".

## Outcome

The instance sees its usage headroom across every account, forecasts when the pool runs out,
spends less before it does, stops at safe points instead of mid-unit, and resumes every session by
itself when a window resets, with no operator message. Conductor stays generic: the probe and the
rotation are commands named in config (here `ais`), not built in.
