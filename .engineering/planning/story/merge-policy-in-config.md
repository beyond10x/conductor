---
format: aep.planning-md/3
id: story:merge-policy-in-config
kind: story
status: active
title: Who merges is an instance config key, and unwatched open pull requests are listed
scope:
- confidence: inferred
  path: crates/conductor-model/src/config.rs
- confidence: cited
  path: crates/conductor/roles/controller.md
- confidence: inferred
  path: crates/conductor/roles/supervisor.md
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: inferred
  path: crates/conductor/src/shipped.rs
- confidence: inferred
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: website/docs/guides/start-conductor.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T16:46:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T16:46:11Z", actor: "human:timo", revision: 4}
---
## Why

The operator, 2026-11-07: "shouldnt we be able to configure the system to configure auto-merge or merge manually - I do not want to be involved in merging stuff (given it was reviewed by agents anyway)". Who merges is read today from a plain `merge:` line in a repository's charter (`crates/conductor/src/run/dispatch.rs:177`, `merge_human`), which no config page or command shows or sets. A second instance had six open pull requests older than 24 h, the oldest 10 days, all in repositories with no running controller, and nothing listed them.

## Acceptance

- The instance config has a key `merge` (`controller` | `human`, default `controller`) and each `repositories[]` entry may override it; `conductor config show` prints both, and dispatch reads the effective value instead of the charter's `merge:` line.
- Under `controller`, the controller profile tells the controller to set auto-merge on a pull request it opened itself once its pipeline is green, and to merge it itself where the forge has no auto-merge.
- The watch tick writes one line per open pull request or merge request that the instance shipped (opened by one of its controller sessions, as recorded in a dispatch report or the instance's merge-request body record), older than 24 h, whose repository has no running controller, naming repository, number and age; a test reads that line from a fixture of forge answers.
- Scope (the operator, 2026-11-07: "the rules must only relate to the things shipped by conductor"): neither the merge policy nor the watch line ever applies to a pull or merge request the instance did not ship. Nothing scans an organization or group of the instance's sources for open requests; a test holds that a request in the fixture with no record of the instance is neither listed nor merged.
