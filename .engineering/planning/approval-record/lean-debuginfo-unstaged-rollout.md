---
format: aep.planning-md/3
id: approval-record:lean-debuginfo-unstaged-rollout
kind: approval-record
status: accepted
title: conductor-dev rolled out line-tables-only debuginfo host-wide without staging it
relations:
- decides: story:build-run-lean-debuginfo
revision: 2
transitions:
- {from: "draft", to: "accepted", at: "2026-10-17T19:20:16Z", actor: "human:timo", revision: 2}
---
## What

On 2026-10-17 at 18:2xZ, conductor-dev made every repository's builds use line-tables-only debug information at once, through the instance rules. New artifact hashes doubled `target/` in the trees that rebuilt before being cleaned: one went from 9 to 38G, another from 3 to 29G. Every repository also recompiled at the same time, and load reached 79.

## Correction

Each tree is cleaned once before its first lean build. Host-wide build changes are staged: one repository first, measured, then the rest. The binary sets the variables itself (story build-run-lean-debuginfo).
