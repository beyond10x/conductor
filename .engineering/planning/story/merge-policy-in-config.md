---
format: aep.planning-md/3
id: story:merge-policy-in-config
kind: story
status: draft
title: Who merges is an instance config key, and unwatched open pull requests are listed
revision: 2
---
## Why

The operator, 2026-11-07: "shouldnt we be able to configure the system to configure auto-merge or merge manually - I do not want to be involved in merging stuff (given it was reviewed by agents anyway)". Who merges is read today from a plain `merge:` line in a repository's charter (`crates/conductor/src/run/dispatch.rs:177`, `merge_human`), which no config page or command shows or sets. A second instance had six open pull requests older than 24 h, the oldest 10 days, all in repositories with no running controller, and nothing listed them.

## Acceptance

- The instance config has a key `merge` (`controller` | `human`, default `controller`) and each `repositories[]` entry may override it; `conductor config show` prints both, and dispatch reads the effective value instead of the charter's `merge:` line.
- Under `controller`, the controller profile tells the controller to set auto-merge on its pull request once its pipeline is green, and to merge it itself where the forge has no auto-merge.
- The watch tick writes one line per open pull request or merge request older than 24 h in a repository of the instance's sources that has no running controller, naming repository, number and age; a test reads that line from a fixture of forge answers.
