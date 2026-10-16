---
format: aep.planning-md/3
id: story:message-restart-kind
kind: story
status: draft
title: '[RESTART conductor] is a recorded message kind'
revision: 2
---
# Story: message-restart-kind

## Outcome

`[RESTART conductor] <reason>` is a message kind the store records like every other § 6 form, and so is its answer, `[REPORT conductor -] ready <handoff path>`. `receive-message` no longer rejects the restart line that conductor's profile defines.

## Context

Conductor's report of 2026-10-16 ~21:13Z: `conductor message receive-message` rejected conductor-dev's `[RESTART conductor] …` first line. The rejection is recorded (01a1468f-f4fd-77a5-b17a-507019061f81). `crates/conductor/src/message.rs:88` says why: `[RESTART …]` is design only (design § 6) and has no `MessageKind`, so it does not parse. Conductor-dev sends one at every conductor restart, six times on 2026-10-16.

## Acceptance

1. Spec first: `MessageKind` in spec/domains/dispatch.yaml gains `Restart`, with its fields (target role, reason), validated with ess 0.60. The model and docs are regenerated.
2. `message.rs` FORMS parses `[RESTART <role>] <reason>`, and `receive-message` records it. A test receives one and reads it back in `message messages`.
3. The dashboard's message list shows it like the other kinds.

## Out of Scope

Changing who may send it (conductor-dev only, as the profile says).

## Ambiguities

None.
