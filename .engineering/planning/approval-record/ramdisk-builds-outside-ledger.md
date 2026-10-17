---
format: aep.planning-md/3
id: approval-record:ramdisk-builds-outside-ledger
kind: approval-record
status: accepted
title: conductor-dev builds three fixes on /dev/shm outside the build ledger (one-off)
relations:
- decides: story:session-attach-command
revision: 2
transitions:
- {from: "draft", to: "accepted", at: "2026-10-17T19:20:17Z", actor: "human:timo", revision: 2}
---
## What

On 2026-10-17 at 19:2xZ, / was at 18G, below the 20G floor, and the ledger refused every build, even 1G. conductor-dev therefore builds the attach, sessions-page and rules-into-code fixes with `CARGO_TARGET_DIR=/dev/shm/...`, `CARGO_INCREMENTAL=0`, nice/ionice and 6 jobs, without `conductor build run`, because these builds write nothing to /.

## Limits

This is a one-off exception for the operator's top priority, attach. It ends when / is back above the floor. The ledger does not count /dev/shm; story build-run-disk-aware-target is to make it target-aware.
