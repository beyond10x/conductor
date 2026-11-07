---
format: aep.planning-md/3
id: story:restart-keeps-the-cycle
kind: story
status: draft
title: A loop restart neither cuts a cycle short nor loses it
summary: a restart during a cycle lost that cycle
revision: 2
---
## Why

Observed 2026-11-07: restarting three instances' loops for a release (`conductor run --uninstall`,
then `--install`) during a cycle lost one instance's cycle; it ran again only at the next
`cadence.cycle`.

## Acceptance

- `conductor run --uninstall` while a tick runs waits for that tick to end (bounded, the wait
  printed), or refuses naming the tick; `--force` stops it at once.
- After a restart that cut a cycle short, the loop runs the cycle at start instead of at the next
  `cadence.cycle`.
