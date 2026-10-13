---
format: aep.planning-md/3
id: story:gh-optional
kind: story
status: implemented
title: conductor runs without gh for a workspace with no GitHub repository
relations:
- decomposes: epic:forge-neutral
- depends_on: story:gitlab-source-host
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: crates/conductor/tests/grouped_instance.rs
- confidence: cited
  path: crates/conductor/tests/instance_sources.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/reference/config.md
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:42:54Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "proposed", to: "active", at: "2026-10-13T10:42:54Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "active", to: "implemented", at: "2026-10-13T10:42:55Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"test_result":1,"review_outcome":6}}}
---
## Why

Engineers of a team trying conductor v0.4.0 (2026-10-13) ran `conductor doctor`; every posted run
exited 1, and `gh` was missing on all four that posted output. One asked to drop the dependency:
the team keeps no repository on GitHub. `gh` is one of the eight `PROGRAMS` that `doctor` always
requires (`crates/conductor/src/doctor.rs:24`), while the snapshot already asks for `gh` only with
a `github` owner (`collect::programs`, `crates/conductor/src/collect/mod.rs:109`) and for nothing
on a `gitlab` source.

## Acceptance

- With a gitlab-only config and no `gh` on `PATH`, `conductor doctor` prints `gh: not needed` and
  exits 0 when every other program is present; with a github-only config it prints `glab: not
  needed`; with a `gitlab` source and no `glab` on `PATH` it exits 1 naming `glab`. A test in
  `crates/conductor/tests/doctor.rs` holds the three cases.
- `collect::programs` asks for `glab` when the instance has a `gitlab` source, so a snapshot of a
  gitlab instance without `glab` is refused before it starts; a case in
  `crates/conductor/tests/instance_sources.rs` holds it.

## Notes

- `not needed` is a new variant of `conductor.config.PrerequisiteState` in
  `spec/domains/config.yaml`; this story lands after `story:gitlab-source-host`, which also changes
  that file and the generated model and reference page.
- `story:doctor-verdict` and `story:doctor-minimum-versions` also land on `doctor.rs` and
  `tests/doctor.rs`; both depend on this story. The GitLab collector stories land on
  `collect/mod.rs` after it.
- The README and website wording is `story:gitlab-guide`'s.
- The existing gitlab case of `crates/conductor/tests/grouped_instance.rs` starts a snapshot on a
  `PATH` without `glab`; this story puts a fake `glab` on that `PATH` (the file is shared with
  `story:gitlab-source-host` before it and `story:gitlab-repositories` after it).

## Scope

`spec/domains/config.yaml`, `crates/conductor-model`, `website/docs/reference/config.md`,
`crates/conductor/src/doctor.rs`, `crates/conductor/src/collect/mod.rs`,
`crates/conductor/tests/doctor.rs`, `crates/conductor/tests/instance_sources.rs`,
`crates/conductor/tests/grouped_instance.rs`.
