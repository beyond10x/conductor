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
revision: 7
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

Read on main at release 1.19.2:

- The box's mounts are one fixed list (`crates/conductor/src/placement.rs:1558-1747`, `layout`;
  its table `placement.rs:1506-1537`).
- No config key adds a directory: `spec/domains/config.yaml` has only the guard's
  `read_only_tools` (`spec/domains/config.yaml:240`, `:308`); `conductor.config.Instance`
  (`spec/domains/config.yaml:1031`) has no `box`.
- Substrate takes at most 4 read-only roots per start (`READ_ONLY_ROOTS_BOUND`, `placement.rs:1340`;
  `placement.mounts-exceeded`, `placement.rs:1354`, raised `placement.rs:1716-1746`), and the
  binary's directory, the harness's and the harness's versions directory (`placement.rs:1633-1658`)
  usually take 3.

## Outcome

- An instance declares read-only host directories and environment for its box:

  ```yaml
  box:
    tools: [~/.cargo, ~/.rustup]
    env: {CARGO_HOME: ~/.cargo, RUSTUP_HOME: ~/.rustup}
  ```

  Conductor mounts each directory read-only and puts `env` into the box's start (`BoxRequest.env`,
  the runtime's whole environment, which its sessions inherit), `~/` read as the box's `HOME`.
- Declared directories are read before the binary's and the harness's, so one that holds them
  (`~/.cargo` over `~/.cargo/bin`) takes their place.
- A declared directory that does not exist is left out of the start, and `conductor doctor` names
  it in a row `box <name> tools:` (reported, not a failure).
- `env` may not name `HOME`, `PATH`, `CLAUDE_CONFIG_DIR`, `XDG_STATE_HOME` or a `CONDUCTOR_*`
  variable: `conductor config validate` refuses one at `instances[i].box.env.<name>`.
- `conductor config validate` refuses with `placement.mounts-exceeded` when the declared directories
  and conductor's own would pass substrate's bound, naming each. A declared directory that holds the
  config file or conductord's directory is refused, as `shares` does (`placement.rs:1497`).
- Spec first (AGENTS.md rule 1): a new struct `conductor.config.Box` (`tools: List<String>`,
  `env: Map<String,String>`, `egress: List<String>` reserved for #198, no behaviour) and
  `box: conductor.config.Box` on `Instance` in `spec/domains/config.yaml`; `task regen`, which
  re-pins `source_digest` in `crates/conductor/http.yaml:2`; `task docs-generate`
  (`website/docs/reference/config.md`). Design: amend `docs/design/architecture-next.md:346`.

Decision (class C, controller, 2026-11-18): the config takes the issue's shape, one `tools` list and
one `env` map per instance, not an environment per directory; a value of `env` is set for every
session of the box. Undone by a per-repository form later, or a later `box.path`.

## Acceptance (tests that fail on the current code)

- `tests/placement.rs` `a_declared_tool_directory_is_mounted_read_only_and_its_env_is_in_the_box_start`.
- `tests/placement.rs` `a_tool_directory_holding_the_binary_directory_takes_its_place`.
- `tests/placement.rs` `a_missing_tool_directory_is_left_out_of_the_start`.
- `tests/config.rs` `box_tools_past_the_read_only_bound_are_refused_at_validate`.
- `tests/config.rs` `box_env_naming_a_variable_conductor_sets_is_refused`.
- `tests/doctor.rs` `doctor_names_a_missing_box_tool_directory`.

## Boundaries

- Within the bound, Rust (`~/.cargo`, `~/.rustup`) fits on a host whose binary is under `~/.cargo`;
  a third directory needs a higher `MAX_READ_ONLY_ROOTS` in substrate, an upstream blocker for that
  case only, not for this story.
- The box reads everything under a declared directory, `~/.cargo/credentials.toml` included.
- Network access to registries is `story:box-registry-egress` (`box.egress` is reserved for it).
- A repository's own pinned tool version, installed per repository, is not in this story.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model` (regenerated), `crates/conductor/http.yaml`,
`crates/conductor/src/config.rs` (instance key list :490, parse ~:4684, show ~:6505/6546),
`crates/conductor/src/placement.rs` (`layout`, `request_on`), `crates/conductor/src/doctor.rs`
(`box_lines`, :1696-1830), `tests/placement.rs`, `tests/config.rs`, `tests/doctor.rs`,
`website/docs/reference/config.md`, `website/docs/guides/run-isolated.md`,
`docs/design/architecture-next.md`.
