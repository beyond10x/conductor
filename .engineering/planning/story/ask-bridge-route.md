---
format: aep.planning-md/3
id: story:ask-bridge-route
kind: story
status: draft
title: conductor ask reaches the bridge, which names its route or asks one question
relations:
- decomposes: epic:host-bridge
- depends_on: story:bridge-session
- depends_on: story:ask-direct
revision: 2
---
# Story: ask-bridge-route

## Outcome

`conductor ask "<text>"` without `--to` reaches the bridge, whose first line names its route or asks one question with options, and whose answers come back labelled with their instance.

## Context

docs/design/roles-and-surfaces.md § 4: "The bridge names its route in its first line: `→ <instance>/<repository>: <one line>`. A repository name or an instance's goal decides the route. An ambiguous request is not guessed: the bridge asks one question with options." A request needing two instances becomes one request to each, joined by the bridge. The bridge session and its profile come from story:bridge-session; the channel and the labels from story:ask-direct.

## Domain relations

- Ask → Route, one-to-many (one per instance a request needs), the ask owns its routes; a route never outlives its ask — inferable (docs/design/roles-and-surfaces.md § 4, operator-approved 2026-10-16; no ess/1 document declares it: this story's spec step adds `conductor.bridge.Route` to `spec/domains/bridge.yaml`).
- Route → Instance and Route → Repository, many-to-one each, owning neither — inferable (same section and spec step).

## Acceptance

1. Spec first: `spec/domains/bridge.yaml` gains `conductor.bridge.Route` (instance, repository, line) and the `Ambiguous` answer (one question and its options); `ess specify validate --path spec` exits 0 and the model crate is regenerated.
2. `conductor ask "<text>"` delivers to the bridge's inbox in the host directory and to no director; `crates/conductor/tests/ask.rs` holds it.
3. A bridge reply whose first line is neither `→ <instance>/<repository>: …` nor one question with options is printed as a route error with exit 1; the same test holds both valid forms and the invalid one with a fixture bridge.
4. A reply routed to `alpha` and `beta` prints each answer as `[alpha] …` and `[beta] …`; the same test holds it.
5. End to end with the fake harness (`crates/conductor/tests/fake_provider`), a request naming a repository only `alpha` holds is routed to `alpha`, and one naming a repository both hold gets one question with both as options.

## Scope

`spec/domains/bridge.yaml`, `crates/conductor-model`, `crates/conductor/src/ask.rs`, `crates/conductor/tests/ask.rs`, `crates/conductor/tests/fake_provider`, `.agents/bridge.md` (the route line's exact form).

## Out of Scope

The dashboard input (story:dashboard-ask-input). A history of asks kept on the host: each request lives in the instance's store it was delivered to and in the bridge's transcript.

## Ambiguities

None.
