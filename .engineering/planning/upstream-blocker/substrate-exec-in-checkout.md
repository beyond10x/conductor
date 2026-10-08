---
format: aep.planning-md/3
id: upstream-blocker:substrate-exec-in-checkout
kind: upstream-blocker
status: open
title: Substrate cannot run an exec in an existing checkout, and caps an exec at one core
relations:
- blocks: story:substrate-gate-provider
revision: 2
---
## What would clear it

Substrate's daemon runs an exec in an adopted existing directory (`STATUS.md:144-150` of substrate:
"half done", `exec.start` requires a `ws_` id). That alone clears it for
`story:substrate-gate-provider`. If `story:substrate-tool-spike` shows the in-process host adopts a
managed tree, the gate story can use that host instead, and this blocker is cleared with the spike
page cited.

Not a clearing condition: more than one core per exec
(`crates/substrate-host/src/process.rs:3058-3074`). It makes a gate slow, not impossible; the gate
story's recorded run reports the cost. Source: docs/analysis/2026-10-09-substrate-benefits.md § 1, § 3, § 5. Owner: the substrate repository
(conductor dispatches it).
