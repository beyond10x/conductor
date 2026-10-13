---
format: aep.planning-md/3
id: story:why-and-how-walkthrough
kind: story
status: implemented
title: Why conductor, and one day with it
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T15:22:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T15:22:37Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T15:40:46Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Users asked for a demonstration of conductor and the motivation behind its workflow, and one stopped
after setup trouble to see whether it fits their own way of working. The docs say what each command
does, not why the loop is worth adopting or what a day with it looks like.

## What

A page, `website/docs/concepts/why-conductor.md`, and a recorded walkthrough script beside it:
- the problem in one paragraph: many repositories and agent sessions, nobody holding the whole
  picture, work stopping on questions nobody sees;
- one day with conductor, step by step, on a sample instance of two repositories (the snapshot and
  board, a dispatch, a controller's wave, an escalation answered, the dashboard), each step with the
  command and what it prints;
- what it costs to adopt (prerequisites, time to first snapshot) and what can be adopted alone
  (doctor and snapshot read-only, the dashboard) before any dispatch;
- a terminal recording script (`docs/demo/walkthrough.md`) that reproduces the day on the sample.

## Acceptance

- The page passes the documentation check, every command in it exists, and getting-started links it
  first.
- The walkthrough script runs on a fresh sample instance from `conductor setup` to a published board
  without a step that needs a person, and says where one is needed.

## Scope

`website/docs/concepts/why-conductor.md`, `docs/demo/walkthrough.md`,
`website/docs/getting-started.md`, the sidebar.
