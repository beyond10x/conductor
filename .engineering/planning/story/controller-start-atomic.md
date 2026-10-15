---
format: aep.planning-md/3
id: story:controller-start-atomic
kind: story
status: draft
title: Concurrent controller starts leave exactly one Running controller per repository
revision: 1
---
# Story: controller-start-atomic

## Outcome

However many starts race, a repository has at most one Running controller.

## Context

This is P1 of the operator-requested external review of 0.14.0 (0e7ce756), at crates/conductor/src/controller.rs:324. The uniqueness check and the insert are not atomic: 16 concurrent starts made 11 Running controllers for one repository.

## Acceptance

- The check and the insert happen under the store's writer lock in one transaction, or as a conditional insert the store refuses on conflict.
- Every losing start exits non-zero naming the Running controller.
- A test runs 16 concurrent starts and finds exactly one Running controller, with the other 15 refused.

## Out of Scope

Session spawning itself.

## Ambiguities

None.
