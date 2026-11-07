---
format: aep.planning-md/3
id: story:forge-route
kind: story
status: active
title: An instance names its forge-write route; direct writes its own requests with gh or glab and git
relations:
- decomposes: epic:forge-neutral
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:11:41Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:11:41Z", actor: "human:timo", revision: 3}
---
## Why

Conductor wired the bot route `b10x-gates` in as the only way to write to the forge: the guard
denied every `gh` and `glab` write, and `conductor run` committed and pushed the records with
`b10x-gates bot`. An instance of another organization (a self-hosted GitLab group whose sessions
use `glab` on its keyring login, say) has no `b10x-gates` and must open, update and merge its
merge requests and push its branches with the forge CLI and git directly.

## What

- `forge.route` in the instance config (`conductor.config.Forge`, `conductor.config.ForgeRoute`):
  `gates` (the default, today's behaviour) or `direct`.
- Under `direct` the guard admits `gh pr create|edit|comment|merge|ready` and `glab mr
  create|update|note|merge` (auto-merge included) of the session's own repository, named with
  `-R`/`--repo`; it keeps denying another repository, a URL operand, an assignment of a forge
  host or repository variable, credential writes, API writes and every other write.
- Under `direct` `conductor run` commits and pushes the records with plain git, and the broker's
  `commit` uses `git commit` with no Gates policy.
- The board's `stale gates hooks` counts only under `gates` or with a `gates_policy`.
- The controller profile names the route `forge.route` gives, with no bot-route sentence that
  holds for every instance.

## Acceptance

- `config show` prints `forge.route: gates` for an instance that writes none; `direct` is read;
  any other word is refused by its YAML path (`tests/config.rs`).
- The guard admits the direct writes above and denies the rest, for a controller and for
  conductor; under `gates` and by default every write stays denied (`tests/guard_rules.rs`).
- Under `direct` the records are committed and pushed with git and the bot route never runs
  (`tests/run_records.rs`); the broker commits with git and needs no policy
  (`conductor-broker/tests/commit.rs`).
