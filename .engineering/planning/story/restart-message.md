---
format: aep.planning-md/3
id: story:restart-message
kind: story
status: draft
title: Model the restart message, the ready report and the conductor-dev actor
revision: 1
---
# Model the restart message, the ready report and the conductor-dev actor

## What

Design § 5.5 lists `[RESTART conductor] <reason>` (conductor-dev → conductor) and § 5.11 step 2
has conductor answer `[REPORT conductor -] ready <handoff path>`. § 5.5 says: "`[RESTART …]` and
the `conductor-dev` actor are not in `spec/` yet. Modelling them is conductor-dev's first story;
until it lands they are design only." (`docs/design/conductor.md:364`.)

`spec/domains/dispatch.yaml:33` declares `MessageKind` as `[Dispatch, Report, DecisionRequest,
Decision, Need, Resource, Escalation, Unknown]`; no variant carries a restart. The domains declare
the actors `RepositoryController`, `Conductor` and `Operator` (`spec/domains/decision.yaml:121`,
`:128`, `:137`; `spec/domains/direction.yaml:140`, `:150`; `spec/domains/dispatch.yaml:240`, `:263`)
and no conductor-dev.

Spec first (`ess:specifying`, ess 0.54.0):
1. a `conductor-dev` actor, with the commands it may run and none of conductor's decision or
   dispatch commands;
2. a message kind for `[RESTART conductor] <reason>`, received by conductor;
3. the `ready` report with dispatch id `-`, which moves no dispatch (like `progress`, § 5.5).

Then `task regen`, and § 5.5's "not in `spec/` yet" paragraph is replaced by a pointer to the
declarations.

## Scope

- `crates/conductor-model`
- `docs/design/conductor.md`
- `spec/domains/dispatch.yaml`
- `spec/ess-inputs.yaml`
- `spec/scenarios/`

## Acceptance

- `ess specify validate --path spec` is valid with the three declarations.
- A scenario in `spec/scenarios/` receives `[RESTART conductor] profile changed` as the restart
  kind, and `[REPORT conductor -] ready docs/handoff/<handoff>.md` as a report that moves no
  dispatch; `ess verify conform run --target interpreted` passes it.
- A scenario where the `conductor-dev` actor runs `decision record-conductor-decision` is refused.
- `task check` passes on the integration branch, and `crates/conductor-model` is the output of
  `task regen`.
- `rg -n 'not in .spec/. yet' docs/design/conductor.md` prints nothing.
