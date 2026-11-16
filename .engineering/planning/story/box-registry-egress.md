---
format: aep.planning-md/3
id: story:box-registry-egress
kind: story
status: draft
title: An instance declares extra egress apertures for its box, within substrate's bound
summary: Package registries and release-download hosts reach a box through operator-declared apertures, and conductor says when the four-aperture bound leaves no room
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-forge-work
scope:
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: inferred
  path: crates/conductor/src/doctor.rs
revision: 1
---
## Why

Agentbox friction logs F33, F34 and F35 (2026-11-15). In the box, crates.io, the npm registry,
`proxy.golang.org` and github.com release downloads do not resolve. The operator prefetched
279M of Go modules and a cargo home into the instance cache root, and set `GOPROXY=off`. A
repository's pinned tool could not be downloaded.

Read at v1.13.0:

- A box's apertures are fixed: `conductord`, `conductor`, and one `forge_<host>` per forge
  (`crates/conductor/src/placement.rs:145-235`).
- Substrate takes at most 4 per start (`APERTURES_BOUND`, `placement.rs:111-121`; substrate-wire
  `MAX_EGRESS_APERTURES` = 4). One GitLab forge already uses 3.
- crates.io alone needs `index.crates.io` and `static.crates.io`. A github.com download needs
  `github.com` and its object host.

## Outcome

- An instance names extra aperture names the substrate daemon declares (for example
  `registry_crates_index`), and its box start names them after the forge apertures.
- Conductor refuses at validate when the total passes the bound (`placement.apertures-exceeded`),
  and doctor names each undeclared one with its `substrate-daemon` flag, as for forges.

## Acceptance

- An instance that names one extra aperture the daemon declares gets a box request that names
  it after the forge apertures. One that takes the total past 4 is refused at
  `conductor config validate` with `placement.apertures-exceeded`. Both are held in
  `tests/placement.rs`.

## Boundaries

- Raising the bound of 4, or a proxy aperture that serves several registry hosts, is
  substrate's (beyond10x/substrate). Without one of them a GitLab-and-crates.io box cannot be
  served. This story should get an upstream blocker on substrate before it moves.
- The `gitlab.stack.babelforce.com` alias of F34 is host git configuration (`insteadOf`), not
  conductor's.
