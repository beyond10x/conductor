---
title: Roles
sidebar_position: 1
description: The operator, conductor, the maintainer, one controller per repository with its workers, and the host roles supervisor and teacher; who owns what and who talks to whom.
lede: Each role owns one part of the work, and every message between an instance's sessions runs through conductor.
source: docs/design/conductor.md sections 2, 3, 11 and 12; docs/design/architecture-next.md section 10; crates/conductor/roles/maintainer.md, controller.md, supervisor.md, teacher.md; crates/conductor/src/role.rs, config.rs, controller.rs, daemon/host_roles.rs
---

## The roles

| role | who | owns | does not |
|---|---|---|---|
| operator | the person running the instance | direction, architecture, new or retired repositories, credentials and spending, public-facing risk | routine approvals |
| conductor | `conductor run`, one process per instance and no session ([the host](./the-host.md#the-loop)) | the cross-repository plan, decisions within its grant, dispatch, the loop's records, the operator brief | edit outside its records, or anything that defines or runs conductor |
| maintainer (conductor-dev) | one session, `<session_prefix>-conductor-dev`, in the conductor repository | conductor's code, specification, plan, design, profiles and tasks; the instance's `rules.md`; installing the binary | dispatch to controllers, or write conductor's records |
| controller | one session per repository, `<session_prefix>-<repo>`, in its checkout | its repository's plan, waves, reviews, releases and workers | message anyone but conductor, or edit outside its repository |
| worker | a sub-agent of a controller | one unit of work | act outside the unit |
| supervisor | one session per host, `supervisor` | every instance running and shipping; defects found and sent to the maintainer | act as an instance's conductor or a controller |
| teacher | one session per host, `teacher` | the supervisor's profile and the host's file for it, improved with evidence | run the supervisor's routine |

In the config file the roles are `director`, `maintainer` and `lead`. Their old names `conductor`,
`conductor-dev` and `controller` are read for this release. Each role runs on the harness, model,
profile and settings file its config entry names ([config](./config.md)).

A repository needs no planning store to be dispatched to. In a repository without an AEP planning
store, that is without `.engineering/project.yaml`, the controller delivers its dispatch with the same gates
and one pull request. The dispatch's brief, written into the pull request's description, stands
for the plan. The controller creates no store: its `started` report offers one once.

A charter's `merge:` line decides who merges. With `merge: controller`, the default, the
controller merges once its full gate is green. With `merge: human` it opens the pull request,
reports it for review and does not merge it.

## A star

Controllers send to conductor and receive only from it. A controller that needs something from
another repository raises a `[NEED]`. Conductor routes it to that repository's controller as a
dispatch (`conductor message route-need`), or queues it. A session outside the instance that needs
something from one of its repositories talks to conductor.

The maintainer sits outside the star: it talks to the operator and to conductor, not to a
controller. `conductor controller start-controller` refuses a second controller for a repository
that has a running or paused one (`ControllerAlreadyRunning`). It records the controller only, and
`conductor spawn` starts the session ([sessions](/docs/security/sessions)).

## Host roles

The supervisor and the teacher belong to the host, not to an instance. conductord starts each
while `globals.roles` lists it, through `conductor session start --role <role>` in the directory
of the config file, and starts it again when it stops.
`conductor daemon start-role --role <role>` asks for one now (`POST /v1/roles/{role}/start`). A role the file does not list is
refused, naming `globals.roles`.

- The supervisor keeps every instance's loop, controllers and maintainer running and shipping. It
  acts without asking on anything reversible and reports blockers and decisions that need the
  operator. `POST /v1/ask` on the daemon delivers a question to its session.
- The teacher observes the supervisor, the maintainer and every instance. It improves the
  supervisor through its profile and `host/supervisor.md` beside the config file, with rules backed
  by evidence.

The operator works in an ordinary harness session that is no role and talks to the supervisor.
When the operator hands an instance to a session the supervisor did not start, the supervisor
watches that session's merge requests and pipelines, messages it, and starts nothing in that
instance.

## Profiles and rules

Every session follows a generic profile the binary carries, compiled in from
`crates/conductor/roles/` (`conductor role show --role <role>` prints one), then the instance's
`rules.md` in its records directory:

| file | read by |
|---|---|
| `maintainer.md` | the maintainer |
| `controller.md` | every controller |
| `supervisor.md`, `teacher.md` | the host's supervisor and teacher |
| `rules.md`, in the records | every session, after its profile: the organization's rules, one section per role, each with its source |
| `charters/<repo>.md`, in the records | one controller: its goal, boundaries, current dispatch and standing decisions |

The binary carries no profile for `director`, which starts with the agent `conductor`. A role's
`profile` in the config takes the place of the binary's text. The profiles name no organization: a
value that differs per instance is written `section.key`, such as `thresholds.disk_low`, and read
from `conductor config show --format json`.

The maintainer turns findings about conductor into planned work in the conductor repository and
installs the new binary after a merged change with a green gate. A running instance keeps the binary
conductord copied for it until `conductor instance upgrade` moves it
([how we work](/docs/develop/how-we-work)).
