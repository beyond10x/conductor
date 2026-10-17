---
format: aep.planning-md/3
id: approval-record:held-agents-back-for-usage
kind: approval-record
status: rejected
title: conductor-dev held agents back for usage (reversed by the operator)
revision: 2
transitions:
- {from: "draft", to: "rejected", at: "2026-10-17T19:20:16Z", actor: "human:timo", revision: 2}
---
## What

At 14:42Z on 2026-10-17, with one account at 16%, conductor-dev resumed only the throughput agent and held two wave 40 agents until the next reset. The same happened again at about 18:00Z. These decisions were recorded only in conductor-dev's private hand-over, not here.

## Outcome

Reversed by the operator: "dont worry about usage". Agents are no longer rationed for usage.
