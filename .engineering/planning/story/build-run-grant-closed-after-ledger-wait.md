---
format: aep.planning-md/3
id: story:build-run-grant-closed-after-ledger-wait
kind: story
status: draft
title: A grant is closed when the ledger frees after a run
summary: grants stayed Running after the ledger wait timed out
revision: 1
---
## Why

Observed 2026-11-07 on a second instance: after normal runs `conductor build run` printed "ledger was not free within 30s … grant stays Running" (grants 01a1b615-c235-744f-afab-8c65a35535c9 and 01a1b61a-c5d9-7376-9f25-f5f988979c3e). A grant left Running holds a build slot for nobody until a later sweep.

## Acceptance

- A test holds the ledger lock for 31 s while a run ends, and the grant is closed by the run once the lock frees, or by the next `build run` within one call.
- No grant stays Running after its run's process is gone for longer than one `build run` call.
