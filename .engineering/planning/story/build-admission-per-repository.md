---
format: aep.planning-md/3
id: story:build-admission-per-repository
kind: story
status: draft
title: Build admission estimates per repository and reclaims regenerable caches before refusing
summary: An sh -c gate of a small repository was sized from the largest sh -c run on the host
revision: 2
---
## Why

The host supervisor, 2026-11-07: `conductor build run` sizes an `sh -c` command given no `--repository` from the largest peak of any project's `sh -c` run. A small repository's gate (a 16 MB tree) was refused with "57G asked, 27G short", and the refusal raised a class-H disk request whose options listed about 90 worktrees and archives, while a regenerable compiler cache held 32G.

## Acceptance

- `build run` keys its peak estimate on the repository of the working directory the command runs in (also through `env -C <tree>` or `cd <tree> &&` at the head of an `sh -c`), else on the command line itself; it never pools every `sh -c` run of the host into one estimate.
- With no history for that key, the estimate is the repository's tree size times a stated factor, named on standard error with its source.
- Before a refusal, reclaim frees regenerable caches (the compiler caches and `go-build` that `thresholds` or the host config name) without a person; only what remains short is reported.
- A class-H disk request lists at most the ten largest reclaimable paths, not every tree and archive.
- Tests over a fixture ledger: two repositories' `sh -c` peaks stay apart; a regenerable cache is reclaimed before a refusal.
