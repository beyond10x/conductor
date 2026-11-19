---
format: aep.planning-md/3
id: story:box-registry-egress
kind: story
status: draft
title: An instance declares extra egress apertures for its box, within substrate's bound
summary: Package registries and release-download hosts reach a box through operator-declared apertures, and conductor says when the four-aperture bound leaves no room
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#198
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-forge-work
scope:
- confidence: inferred
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: inferred
  path: crates/conductor/tests/config.rs
- confidence: inferred
  path: crates/conductor/tests/placement.rs
- confidence: cited
  path: spec/domains/config.yaml
revision: 4
---
## Why

Issue #198 (agent box friction F33 registry part, F34). In a box crates.io, proxy.golang.org, the npm
and yarn registries and github.com do not resolve. A controller cannot add a git dependency; private
Go modules served by the forge under a second hostname are not reached; a Node workspace whose SDK is
in the forge's private npm registry cannot install. Each was worked around on the host.

Read on the wave 56 branch:

- A box's egress apertures are fixed: the model host's, conductord's dial-in while
  `daemon.dial_port` is set, and one `forge_<host>` per forge host of the sources
  (`crates/conductor/src/placement.rs:179-200`, `forge_apertures`; `story:box-forge-work`).
- Substrate takes at most 4 per start (`APERTURES_BOUND`, `placement.rs:113`; refused
  `placement.apertures-exceeded` at `placement.rs:205-240`). One GitLab forge with the dial-in
  already takes 3.
- crates.io alone needs `index.crates.io` and `static.crates.io`; a github.com download needs
  `github.com` and its object host.

## Outcome

- An instance names extra apertures the substrate daemon declares (registries, release-download
  hosts, a forge's alias hostname): `box.egress: [<aperture name>...]`; its box start names them
  after the forge apertures.
- `conductor config validate` refuses with `placement.apertures-exceeded` when the total passes the
  bound, naming each aperture and how many are left; doctor names each declared aperture the daemon
  does not declare, with its `substrate-daemon` flag, as for forges (`forges_unreached`,
  `placement.rs:247`), and one whose host does not answer from the box.
- Spec first (AGENTS.md rule 1): `box.egress` is modelled in `spec/domains/config.yaml`.

Decision (class C, controller, 2026-11-18): the instance names substrate aperture names, not
hostnames, so the daemon stays where egress is declared. Undone by accepting hostnames that map to
names as `forge_aperture` does.

## Acceptance (tests that fail on the current code)

- `tests/placement.rs` `a_declared_egress_aperture_is_named_after_the_forge_apertures`.
- `tests/config.rs` `egress_past_the_aperture_bound_is_refused_at_validate`.

## Boundaries

- Raising the bound of 4, or one proxy aperture that serves several registries, is substrate's
  (beyond10x/substrate); without one a GitLab-and-crates.io box cannot be served. This story should
  carry an upstream blocker on substrate before it moves.
- A forge's second hostname for git (`insteadOf`) is host git configuration.
- `story:box-tool-roots` shares the `box` map of the config; whichever lands second extends it.
