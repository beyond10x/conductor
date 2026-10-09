---
format: aep.planning-md/3
id: upstream-blocker:substrate-usage-without-io
kind: upstream-blocker
status: open
title: Substrate refuses an exec's usage when the cgroup lacks the io controller
relations:
- blocks: story:substrate-client
- blocks: story:substrate-tool-server
- blocks: story:substrate-gate-provider
revision: 2
---
## What would clear it

Substrate serves `exec.resource-usage` (CPU, memory, pids) in a delegated cgroup without the `io`
controller, with io reported absent, instead of withholding it; harness then publishes `run`. On
this host the user scope has `cpu memory pids` and no `io`, and a direct exec is refused
`exec.metrics-unserved` (docs/analysis/substrate-tool-spike.md, steps 1 and 2). When this clears, `decision-blocker:host-io-delegation`
is no longer needed and is cleared with this record cited. Owner: the substrate repository; asked 2026-10-09.
