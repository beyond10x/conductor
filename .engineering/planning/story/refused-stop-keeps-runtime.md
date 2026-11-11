---
format: aep.planning-md/3
id: story:refused-stop-keeps-runtime
kind: story
status: draft
title: A runtime whose stop was refused stays known to the daemon
tags:
- from-finding
revision: 1
---
## Why

Found while unit U5 of wave 51 corrected what a reconciliation pass keeps when the store
refuses one of its records (the implementor's report of the last correction;
`review-result:w51-u5-adversary-2`).

- A stop the service manager refuses drops the runtime from `runtimes.json` anyway. The daemon
  then does not know a runtime that may still run. The implementor read this and left it; no
  case measures what follows from it. This was there before the wave.
- Reconciliation records held in memory, because the store refused them, are lost when the
  daemon stops before it could write them. Unit U5 made the rows stay true and says so on the
  host page; the records themselves are still lost. Accepted for wave 51.

## Acceptance

- A runtime whose stop the service manager refused stays in `runtimes.json` and is asked to
  stop again at the next pass; a test with a stand-in manager holds it.
- Either the held records survive a stopped daemon, or the page that says they are lost names
  what an operator reads instead; decided and held by a test or the page.
