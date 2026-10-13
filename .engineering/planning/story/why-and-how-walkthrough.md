---
format: aep.planning-md/3
id: story:why-and-how-walkthrough
kind: story
status: active
title: Why conductor, and one day with it
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T15:22:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T15:22:37Z", actor: "human:timo", revision: 3}
---
## Why

An IVR engineer asked on 2026-10-09 for a demo and the motivation behind the workflow ("best way is
you to demo stuff … folks would probably like to see and understand motivations"), and after setup
trouble on 2026-10-12 said they would "research more and see can I adapt to my current workflows"
(brain's case list). The docs say what each command does, not why the loop is worth adopting or
what a day with it looks like.

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
