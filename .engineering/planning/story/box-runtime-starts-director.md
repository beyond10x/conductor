---
format: aep.planning-md/3
id: story:box-runtime-starts-director
kind: story
status: draft
title: A boxed instance's runtime starts its director in the box, or the instance refuses to run without one
summary: The larger part of issue 220, left for a decision on whether boxed instances run a director
refs:
- provider: github
  reference: beyond10x/conductor#220
relations:
- decomposes: epic:architecture-next
revision: 1
---
## Why

Issue #220, expected 1. A boxed instance runs without a director: conductord starts the runtime
and its controllers in the box, but nothing starts the director, and `session start --role
director` now refuses outside the box (`story:director-boxed-start`). The design retired the
conductor session in favour of `conductor run` (AGENTS.md, Sessions), so whether a boxed instance
runs a director at all is a product decision before it is code.

## Outcome

- Either the runtime (`conductor instance serve`) starts the director in its box from the director
  profile when the instance names a director role and none runs, as the loop starts controllers,
  through the `child` provider; or the instance refuses to run without one, as the issue offers.

## Acceptance

- A box test: a boxed instance with a director role and no director session gets one started by
  its runtime, in the box's registry, under `<session_prefix>-director`.

## Boundaries

- Decide first whether boxed instances run a director (the design retired it); then the start path.
