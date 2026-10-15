---
format: aep.planning-md/3
id: story:controller-onboarding-check
kind: story
status: draft
title: A session conductor did not spawn is never treated as a controller
revision: 1
---
# Story: controller-onboarding-check

## Outcome

Every controller runs with its charter, the repo-controller profile and the guard, because conductor accepts only controllers it spawned. A hand-started session that announces itself is told how to hand over instead.

## Context

On 2026-10-15 an interactive session created a repository and registered itself with a free-form `[REGISTER]` message. It ran for a day with no charter, without the repo-controller profile and with the guard off. As a result it ran a full gate and a push to main per unit, against rules items 110 and 21. It kept no hand-over and was resumed into its old context after each usage limit.

## Acceptance

- `conductor message receive-message` records a REGISTER or REPORT whose sender is not a Running controller's session as `unmanaged <session> <repository>`, and the watch prints that line once.
- The conductor profile says that on `unmanaged`, conductor writes the repository's charter and asks the session to hand over to `.git/handoff/`. It then spawns the controller with `conductor spawn --repository <repo>` and stops the old session after the new one reports ready.
- Tests: a REPORT from an unknown session gives one `unmanaged` line; one from a spawned controller gives none.

## Out of Scope

Refusing messages from unmanaged sessions outright.

## Ambiguities

None.
