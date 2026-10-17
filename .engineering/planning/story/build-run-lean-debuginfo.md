---
format: aep.planning-md/3
id: story:build-run-lean-debuginfo
kind: story
status: draft
title: Build run keeps line tables only, so each target/ is half the size
relations:
- decomposes: epic:disk-protocol
revision: 2
---
# Story: build-run-lean-debuginfo

## Outcome

Builds that `conductor build run` admits keep line tables, not full debug information, so each worktree's `target/` is about half the size or less. The host can carry the concurrent units it runs.

## Context

On 2026-10-17 at 18:2xZ, `/` had 26G free. About ten active worktrees each held 9–19G of `target/`; one ess tree had 19G of `target/` for under 80M of source. The trees were building, not idle, so no cleanup had anything to reclaim. Full debug information makes up most of a debug or test `target/`, and builds and tests do not need it. Interim: rules item 151 has sessions set the variables by hand.

## Acceptance

1. For a cargo-based command, `build run` sets `CARGO_PROFILE_DEV_DEBUG=line-tables-only` and `CARGO_PROFILE_TEST_DEBUG=line-tables-only` unless the environment already sets them, controlled by `thresholds.lean_debuginfo` (default on). Spec first, in `spec/domains/config.yaml`, validated with ess. Tests.
2. A repository whose Cargo profile already sets `debug` keeps its own setting. The env variable overrides that profile, so `build run` skips setting it when the workspace manifest sets `profile.dev.debug` or `profile.test.debug`. Test with a fixture manifest.
3. Evidence: the same crate's `cargo test` `target/` size with and without, recorded on the story.
4. The profiles and rules item 151 stop asking sessions to pass the variables. The profiles test pins this.

## Out of Scope

Release-profile settings.

## Ambiguities

None.
