---
format: aep.planning-md/3
id: story:ess-pin-lifted
kind: story
status: draft
title: The ess pin at 0.93 and the ActorKind order workaround are lifted
tags:
- from-finding
revision: 1
---
## Outcome

The specification is checked with the newest ess release again, and the order of
`conductor.operator.ActorKind` carries no workaround.

## Why

Two ess defects, both reported to the ess maintainers on 2026-11-11 and 2026-11-12 and filed there
as drafts, hold two workarounds in this repository:

- `.github/workflows/check.yml:18` pins `ESS_VERSION: "0.93.0"`. ess 0.94.0 refuses suite
  synthesis for a struct with a `distinct:` invariant, which the specification uses in 9 places.
- `spec/domains/operator.yaml:53`: `ControlPlane` is the first variant of `ActorKind` because, with
  it last, ess 0.93 put the two runs of one synthesized scenario on different instance names and
  the one-row check failed. The comment above the enum says so.

## Acceptance

- Given an ess release that synthesizes a suite for a `distinct:` struct invariant and binds the
  one-row check to its scenario's instance, when `ESS_VERSION` names it, then `task check` passes
  with every conformance scenario that passes at 0.93.0, and the generated model is byte-identical
  to a fresh synthesis.
- The variants of `ActorKind` are in the order `Human, Agent, Daemon, ControlPlane` and the
  comment about the workaround is gone.
