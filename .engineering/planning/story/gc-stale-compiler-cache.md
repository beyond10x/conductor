---
format: aep.planning-md/3
id: story:gc-stale-compiler-cache
kind: story
status: draft
title: gc reclaims a compiler cache no configuration or process uses
revision: 3
---
# Story: gc-stale-compiler-cache

## Outcome

`conductor gc` and `host reclaim` mark a compiler cache reclaimable when no cargo configuration and no live process uses its wrapper, so a cache left behind by a retired setup no longer survives as "shared compiler cache".

## Context

A conductor NEED (2026-10-16): `conductor gc --dry-run` kept `~/.cache/sccache` (5.4 GB) as "shared compiler cache" while / was under disk_low, although rules.md item 145(1) had retired the machine-wide wrapper. The real users turned out to be 27 tree-root `.cargo/config.toml` files. They were moved aside, and the cache was removed by hand at 13:51Z.

## Acceptance

1. gc reads the cargo configurations that apply to the host and to each checkout and tree root: `~/.cargo/config.toml`, and `.cargo/config.toml` in each checkout root, trees root and their parents up to `$HOME`. It also reads the `RUSTC_WRAPPER` of live processes.
2. `~/.cache/sccache` is reclaimable only when none of those names sccache and no sccache process runs. Otherwise it is kept, with the path of the configuration or the pid that uses it.
3. A tree-root `.cargo/config.toml` that sets `rustc-wrapper` is reported by `conductor doctor` as a violation of item 145(1), with its path.
4. Tests cover a fixture home with and without a wrapper config.

## Out of Scope

`~/.cache/go-build`, which stays age-based.

## Ambiguities

None.
