---
format: aep.planning-md/3
id: story:gc-archive-measure
kind: story
status: draft
title: gc measures archive build output within one cycle
revision: 1
---
## Why

Conductor's NEED N-20261013-15: under load (40-65), `conductor gc --dry-run` gave no answer for the
worktree archives, "build output in worktree archives ~/.local/state/worktree/archives: not measured
(timed out after 1800 s)", and the dry run took about 39 minutes. Under a disk floor gc could not
say what the archives would free.

## What

gc measures archive build output within one cycle:
- each archive is measured on its own and its row printed as soon as it is known;
- a size is cached by archive id and the archive's modification time under the state directory, so
  an unchanged archive is not walked again;
- when the bound is reached, gc prints a partial row (the archives measured, their bytes, and
  `partial: <n> of <m> archives`) instead of no row.

## Acceptance

- On a fixture of three archives with the bound set so that only two are walked, the dry run prints
  two archive rows and `partial: 2 of 3 archives`, and exits 0.
- A second dry run with nothing changed walks no archive (the cache answers) and prints all three
  rows.
- An archive whose modification time changed is walked again.

## Scope

`crates/conductor/src/gc.rs` (the archives row), its cache file under the state directory,
`crates/conductor/tests/gc.rs`.
