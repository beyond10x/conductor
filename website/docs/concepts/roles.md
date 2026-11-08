---
title: Roles
sidebar_position: 1
description: The operator, conductor, conductor-dev, one controller per repository, and their workers; who owns what and who talks to whom.
lede: Five roles split the work, and every message runs through conductor, so each session reads only what its role needs.
source: docs/design/conductor.md sections 2, 3, 11 and 12; docs/design/conductor-run.md; crates/conductor/roles/maintainer.md, crates/conductor/roles/controller.md; crates/conductor/src/controller.rs
---

## The roles

| role | who | owns | never |
|---|---|---|---|
| operator | the person running the instance | direction, architecture, new or retired repositories, credentials and spending, public-facing risk | routine approvals |
| conductor | `conductor run`, one process per instance and no session ([start conductor](../guides/start-conductor.md)) | the cross-repository plan, decisions within its grant, dispatch, the loop's records, the operator brief | edits outside its records; anything that defines or runs conductor |
| conductor-dev | one session, `<session_prefix>-conductor-dev`, in the conductor repository | conductor's code, specification, plan, design, profiles and tasks; the instance's `rules.md`; installing the binary; restarting `conductor run` | dispatching to controllers; writing conductor's records |
| repository controller | one session per repository, `<session_prefix>-<repo>`, its working directory the checkout | its repository's plan, waves, reviews, releases and workers | messages to anyone but conductor, each a `conductor message send`; edits outside its repository |
| worker | a sub-agent of a controller | one unit of work | anything outside the unit |

Each role runs on the harness and model the instance's `roles` name, and starts with the profile
and settings file the role names ([instances and config](./instances-and-config.md)).

**A repository needs no planning store to be dispatched to.** In a repository without an AEP
planning store (no `.engineering/project.yaml`), the controller delivers its dispatch the same way,
with the same gates and one pull request, and the dispatch's brief, written into the pull
request's description, stands for the plan. It does not create a store itself: its `started`
report offers one once.

**Who merges.** A charter's `merge:` line decides it: `merge: controller`, the default, lets the
controller merge once its full gate is green; with `merge: human` it opens the pull request,
reports it for review and never merges it.

**Session names.** A session's name is derived from the instance's `session_prefix`, never chosen:
`<session_prefix>-conductor`, `<session_prefix>-conductor-dev` and `<session_prefix>-<repo>` for a
controller. An instance without a prefix keeps the bare names `conductor`, `conductor-dev` and
`<repo>`; at most one instance of a config file may do so. The guard and the sessions collector read
the name to tell which instance a session belongs to.

## A star

Controllers send to `conductor` and receive only from it. A controller that needs something from
another repository raises a `[NEED]`; conductor routes it to that repository's controller as a
dispatch (`conductor message route-need`), or queues it. A session outside the instance that needs
something from one of its repositories talks to conductor, not to a controller.

conductor-dev sits outside the star. It talks to the operator and to conductor, never to a
controller.

**One controller per repository.** `conductor controller start-controller` refuses a second
controller for a repository that already has a running or paused one (`ControllerAlreadyRunning`).

`controller start-controller` records the controller only and starts no session: `conductor
spawn` starts it, with the controller role's model, profile and settings. `controller
pause-controller` and `stop-controller` stop the session `conductor spawn` started through the
instance's confinement provider, and `resume-controller` resumes it from its transcript.

## Profiles and rules

Every session follows a generic profile the `conductor` binary carries (compiled in from
`crates/conductor/roles/`; `conductor role show --role <role>` prints one), then the instance's
`rules.md` in its records directory:

| file | read by |
|---|---|
| `crates/conductor/roles/maintainer.md` | the conductor-dev session |
| `crates/conductor/roles/controller.md` | every repository controller |
| `crates/conductor/roles/supervisor.md`, `teacher.md` | the host's supervisor and teacher |
| `rules.md`, in the instance's records | every session, after its profile: the organization's own rules, one section per role, each rule with its source |
| `charters/<repo>.md`, in the records | one controller: its goal, boundaries, current dispatch and standing decisions |

The profiles name no organization. A value that differs per instance is written `section.key`,
such as `thresholds.disk_low`, and read from `conductor config show --format json`. What holds for
one organization goes in its `rules.md`, which the repository does not ship.

**Profile, not agent.** When a session starts, its agent kind writes what the harness reads from
the binary: the `claude-code` kind writes the role's profile to
`<state>/sessions/<session name>.profile.md` and appends it with `--append-system-prompt-file`, and
writes the settings the binary carries for conductor and a controller, which wire the guard,
beside it. A role's `profile` or `settings` in the config takes the place of the binary's and is
read at start. Nothing is copied to the host: no profile directory, no adapter under
`~/.claude/agents/`, and the repository commits none. A session the `systemd` provider starts
passes `--setting-sources project,local`, so Claude Code loads no user-level agent; that is why
`conductor config validate` refuses a controller role that names an `agent` and no `profile`.

## Developing conductor

Conductor runs the organization; it does not build itself. conductor-dev owns conductor's
development: it reads `conductor status`, the dispatch log and `conductor metric kpi-readings`,
turns each finding into planned work in the repository, and after a merged change with a green
gate runs `task install`. The CLI keeps no state outside its state directory, so a new binary
needs no restart.

`conductor run` keeps no context either. After a release, conductord restarts each instance's
runtime on the new binary at its next pass; nothing is handed over.
