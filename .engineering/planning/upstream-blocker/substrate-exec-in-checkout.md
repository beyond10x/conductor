---
format: aep.planning-md/3
id: upstream-blocker:substrate-exec-in-checkout
kind: upstream-blocker
status: open
title: Substrate cannot run an exec in an existing checkout, and caps an exec at one core
relations:
- blocks: story:substrate-gate-provider
revision: 1
---
## What would clear it

Substrate runs an exec in an adopted existing directory (`STATUS.md:144-150` of substrate:
"half done", `exec.start` requires a `ws_` id) and allows more than one core per exec
(`crates/substrate-host/src/process.rs:3058-3074`). Source: docs/analysis/2026-10-09-substrate-benefits.md § 1, § 3. Owner: the substrate
repository (conductor dispatches it).
