---
format: aep.planning-md/3
id: story:box-tool-roots
kind: story
status: active
title: An instance declares read-only tool directories and environment for its box
summary: Toolchains, tool caches and pinned tools on the host reach a boxed runtime's sessions through declared read-only roots and environment, not through copies in the cache root
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#197
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-runtime-starts
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
  path: crates/conductor/tests/doctor.rs
- confidence: inferred
  path: crates/conductor/tests/placement.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: inferred
  path: website/docs/guides/run-isolated.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T06:10:46Z", actor: "human:unknown", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-20T06:10:46Z", actor: "human:unknown", revision: 6}
---
## Why

Issue #197 (agent box friction F27, F30, F33 tool part, F35). Toolchains, tool caches and pinned tools
on the host do not reach a box's sessions. Each NEED was worked around by copying into the instance's
cache root (the one writable root that is no checkout) and setting `CARGO_HOME`, `AEP_CACHE_DIR`,
`GOMODCACHE`, `PATH` in `<state>/claude/settings.json`; every copy goes stale when the host updates,
and repeats per instance:

- `~/.cargo` and `~/.rustup` are not in the box: no `cargo`.
- `~/.cache/aep` is not in the box (its home is a tmpfs), so a prefetch on the host never reaches it.
- A repository's pinned tool, installed by a script from github.com, cannot be downloaded; the box
  ships conductor's pin.
- Go modules were prefetched into the cache root with `GOPROXY=off`.

Read on the wave 56 branch:

- The box's mounts are one fixed list (`crates/conductor/src/placement.rs:1476-1510`, `mounts`).
- No config key adds a directory to read: `spec/domains/config.yaml` has only the guard's
  `read_only_tools` (`spec/domains/config.yaml:235`, `:303`).
- Substrate takes at most 4 read-only roots per start (`READ_ONLY_ROOTS_BOUND`, `placement.rs:1310`,
  refused `placement.mounts-exceeded` at `placement.rs:1324`). The binary's directory, the
  harness's, and since `story:box-harness-versions-dir` the harness's versions directory may take 3.

## Outcome

- An instance declares read-only host directories and environment for its box:

  ```yaml
  box:
    tools: [~/.cargo, ~/.rustup, ~/.cache/aep]
    env: {CARGO_HOME: ~/.cargo, RUSTUP_HOME: ~/.rustup, AEP_CACHE_DIR: ~/.cache/aep}
  ```

  Conductor mounts each directory read-only and puts `env` into the environment of the box's
  sessions, `~/` read as the box's `HOME`.
- `conductor config validate` refuses with `placement.mounts-exceeded` when the declared directories
  and conductor's own would pass substrate's bound, naming each; `conductor doctor` names a declared
  directory that is missing.
- Spec first (AGENTS.md rule 1): the instance's `box` map with `tools` and `env` is modelled in
  `spec/domains/config.yaml`, then `task regen`.

Decision (class C, controller, 2026-11-18): the config takes the issue's shape, one `tools` list and
one `env` map per instance, not an environment per directory; a value of `env` is set for every
session of the box. Undone by a per-repository form later.

## Acceptance (tests that fail on the current code)

- `tests/placement.rs` `a_declared_tool_directory_is_mounted_read_only_and_its_env_reaches_the_session`.
- `tests/config.rs` `box_tools_past_the_read_only_bound_are_refused_at_validate`.
- `tests/doctor.rs` `doctor_names_a_missing_box_tool_directory`.

## Boundaries

- With up to 3 of 4 read-only roots taken, one tool directory fits, or several under one parent; more
  needs a higher bound in substrate (beyond10x/substrate), which this story should carry as an
  upstream blocker before it moves.
- Network access to registries is `story:box-registry-egress`.
- A repository's own pinned tool version, installed per repository, is not in this story.
