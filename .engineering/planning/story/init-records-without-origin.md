---
format: aep.planning-md/3
id: story:init-records-without-origin
kind: story
status: draft
title: A snapshot completes over freshly initialised records that have no origin yet
scope:
- confidence: cited
  path: crates/conductor/src/collect/blockers.rs
- confidence: cited
  path: crates/conductor/tests/workspace_snapshot.rs
- confidence: cited
  path: website/docs/guides/initialise-the-records.md
revision: 2
---
## Why

A fresh `conductor init` adds the records directory as a workspace member. While the records have
no `origin` remote, every snapshot fails: `blockers: repository records … git fetch --quiet origin
exited 128`. `config workspace` already leaves out a checkout with no origin (`4840a9d`), but the
records member is written regardless. Reproduced by the docs overhaul, 2026-10-12; the guide
`website/docs/guides/initialise-the-records.md` tells the reader to add an origin first.

## Acceptance

- A snapshot over a workspace whose records member has no `origin` completes; the blockers collector
  reads the records' store without fetching. Test in `tests/workspace_snapshot.rs`.
- The guide drops the "add an origin first" step.

## Scope

`crates/conductor/src/collect/blockers.rs`, `crates/conductor/tests/workspace_snapshot.rs`,
`website/docs/guides/initialise-the-records.md`.
