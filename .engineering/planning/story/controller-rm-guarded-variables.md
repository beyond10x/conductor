---
format: aep.planning-md/3
id: story:controller-rm-guarded-variables
kind: story
status: draft
title: Controllers write a variable in an rm path as "${VAR:?}", so no harness prompt stops them
summary: Two controllers stood at an rm prompt in one day
relations:
- decomposes: epic:loop-without-handwork
revision: 1
---
# Story: controller-rm-guarded-variables

## Why

Host supervisor, 2026-11-08: twice in one day a controller ran `rm -rf` on a path built from a
shell variable (`$D/$m`, `$S/$v/scenarios`), the harness asked "Dangerous rm operation on
possibly-empty variable path", and the controller stood at that Yes/No prompt until someone
answered (11:00 and 11:5x local time). `.agents/repo-controller.md:108` names `rm -rf -- <paths>`
and says nothing about variables.

## Acceptance

1. `.agents/repo-controller.md` says a variable in an `rm` path is written `"${VAR:?}"`, because
   a bare `$VAR` stops the session at a prompt nobody answers; the profile stays within its
   120-line cap.
2. `crates/conductor/tests/profiles.rs` pins the sentence.

## Out of Scope

Detecting the prompt; that is story:watch-session-waiting.

## Ambiguities

None.
