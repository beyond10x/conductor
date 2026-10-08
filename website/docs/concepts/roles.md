---
title: Roles
sidebar_position: 1
description: The operator, conductor, conductor-dev, one controller per repository, and their workers; who owns what and who talks to whom.
lede: Five roles split the work, and every message runs through conductor, so each session reads only what its role needs.
source: docs/design/conductor.md sections 2, 3, 11 and 12; .agents/conductor.md, .agents/conductor-dev.md, .agents/repo-controller.md; crates/conductor/src/controller.rs
---

## The roles

| role | who | owns | never |
|---|---|---|---|
| operator | the person running the instance | direction, architecture, new or retired repositories, credentials and spending, public-facing risk | routine approvals |
| conductor | one session, `conductor` | the cross-repository plan, decisions within its grant, dispatch, the loop's records, the operator brief | edits outside its records; anything that defines or runs conductor |
| conductor-dev | one session, `conductor-dev`, in the conductor repository | conductor's code, specification, plan, design, profiles, adapters and tasks; the instance's `rules.md`; installing the binary; restarting conductor | dispatching to controllers; writing conductor's records |
| repository controller | one session per repository, its working directory the checkout | its repository's plan, waves, reviews, releases and workers | messages to anyone but conductor; edits outside its repository |
| worker | a sub-agent of a controller | one unit of work | anything outside the unit |

Each role runs on the harness and model the instance's `roles` name, and starts with the harness
agent and settings file the role names, if any ([instances and config](./instances-and-config.md)).

## A star

Controllers send to `conductor` and receive only from it. A controller that needs something from
another repository raises a `[NEED]`; conductor routes it to that repository's controller as a
dispatch (`conductor message route-need`), or queues it. A session outside the instance that needs
something from one of its repositories talks to conductor, not to a controller.

conductor-dev sits outside the star. It talks to the operator and to conductor, never to a
controller.

**One controller per repository.** `conductor controller start-controller` refuses a second
controller for a repository that already has a running or paused one (`ControllerAlreadyRunning`).

:::caution[Planned]

`controller start-controller` and `stop-controller` record the controller only. They do not start
or end its session: the conductor session starts the harness session itself, with the controller
role's model, agent and settings.

:::

## Profiles and rules

Every session follows a generic profile from the repository's `.agents/` directory, then the
instance's `rules.md` in its records directory:

| file | read by |
|---|---|
| `.agents/conductor.md` | the conductor session |
| `.agents/conductor-dev.md` | the conductor-dev session |
| `.agents/repo-controller.md` | every repository controller |
| `rules.md`, in the instance's records | every session, after its profile: the organization's own rules, one section per role, each rule with its source |
| `charters/<repo>.md`, in the records | one controller: its goal, boundaries, current dispatch and standing decisions |

The profiles name no organization. A value that differs per instance is written `section.key`,
such as `thresholds.disk_low`, and read from `conductor config show --format json`. What holds for
one organization goes in its `rules.md`, which the repository does not ship.

For Claude Code, `.claude/agents/` holds one adapter per profile (`conductor.md`,
`conductor-dev.md`, `repo-controller.md`), each the agent front matter and a pointer at its
`.agents/` file.

## Developing conductor

Conductor runs the organization; it does not build itself. conductor-dev owns conductor's
development: it reads conductor's reversed decisions, failed tool calls and requests, turns each
finding into planned work in the repository, and after a merged change with a green gate runs
`task install`. The CLI keeps no state outside its state directory, so a new binary needs no
restart.

Conductor is restarted only when something it read at session start changed: its adapter, its
profile, the controller profile, the instance's `rules.md` or the design. conductor-dev sends
`[RESTART conductor]`; conductor writes a hand-over and answers ready; conductor-dev runs
`task conductor:restart`, which starts a new session that reads the hand-over first.
