---
format: aep.planning-md/3
id: epic:dispatch-runner
kind: epic
status: draft
title: A dispatch names its runner; claude-code-session is the first and today
summary: Spec and model a runner on dispatches and controllers with one value, claude-code-session, and no change in behaviour
revision: 1
---
# Epic: dispatch-runner

## Outcome

A dispatch and a controller name the runner that carries them. `claude-code-session` is the only
runner and is today's behaviour; the field exists so later runners (`loom-agent`, `agency`) are a
new value, not a new path. Spec first: the runner is modelled in `spec/` before any handler.

## Acceptance

`task check` is green; new conformance scenarios for the runner on a dispatch and a controller
pass; every existing dispatch and spawn test passes unchanged; no command is removed.

## Source

`docs/design/conductor-on-agency.md` § 4 step 1 (the first step of the migration), § 6 D8.
