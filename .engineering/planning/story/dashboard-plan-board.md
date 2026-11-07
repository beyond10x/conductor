---
format: aep.planning-md/3
id: story:dashboard-plan-board
kind: story
status: active
title: The dashboard shows the plan as a board and lets the operator approve, reject, start and prioritise stories
summary: the operator asked for a Kanban plan board with approve and priority
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:39:22Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:39:22Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-11-07: "one section being just a Kanban-like planning board ... overview of what's happening next, what's currently being worked on, and a way to actually look into the stories and read them, and also a way to basically approve stories and change them from ... proposed to accepted ... and I can basically tell or decide as a user that this is now sort of important." Carries his ask (rules item 136). A mock-up exists and waits for his approval; nothing is built before it.

## Acceptance

- The dashboard has a Plan page: columns Draft, Proposed, Accepted (next), In progress, Done, grouped by wave, read from the product repository's planning store.
- Opening a story shows its Why, its Acceptance with a tick per line that has evidence, its Scope and its History, and a comment box.
- Approve, Reject and Start move the story through the store's lifecycle (`aep plan artifact move`); a move the lifecycle refuses is shown refused, with the store's reason, and changes nothing.
- "Make it important (P1)" puts the story at the front of the next wave.
- Each move and priority change is recorded as the operator's decision; writes are accepted only from the local origin with the per-process token, as on the Decisions page.

## Notes

- Blocked on the operator's approval of the mock-up (columns, drawer, buttons as above).
