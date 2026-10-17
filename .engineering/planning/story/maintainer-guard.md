---
format: aep.planning-md/3
id: story:maintainer-guard
kind: story
status: draft
title: 'A maintainer rule set in the guard: builds only through build run, no EnterWorktree, no kill by pattern, writes only in the product and its trees'
relations:
- decomposes: epic:conductor-run
revision: 1
---
## Why

The guard has two rule sets, controller and conductor, chosen by the session's working directory
(`crates/conductor/src/guard/rules.rs:917-922`, `guard.rs:381-386`); a maintainer session in the
product checkout runs with settings `{}` and no hook (`crates/conductor/src/session.rs:625-668`),
and the conductor set does not deny `cargo` outside `build run`. On 2026-10-17 the maintainer
session built on `/dev/shm` outside the ledger, entered a worktree with `EnterWorktree` and ran
86 tool calls into the harness's isolation refusals.

## Outcome

A maintainer rule set: builds only through `conductor build run`, no `EnterWorktree`, no kill by
pattern, writes only in the product checkout, its managed trees, the instance's `rules` files and
`<state>/conductor-dev/`, no forge writes outside the bot route; wired by the maintainer's settings
file.

## Acceptance

1. `Rules::Maintainer` in `guard/rules.rs`, chosen when the cwd is the product repository or one
   of its managed trees under the instance's `checkouts.trees`; the table in the module doc lists
   its rows.
2. Denied, each with a test in the guard suite: `cargo build|test|check|clippy|nextest|install|run`,
   `task check|gate|install|ui|regen|docs-generate`, `npm run build|test`, `npm ci` outside
   `conductor build run` (the denial names the form); `EnterWorktree`; `pkill`, `killall`, `kill` by
   pattern or negative group; Edit/Write outside the allowed roots; `gh`/`glab` writes.
3. `conductor session start --role conductor-dev` writes a settings file that wires the hook, as
   the controller's does; `crates/conductor/tests/session_settings.rs` reads it back.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/src/guard.rs`,
`crates/conductor/src/session.rs`, `crates/conductor/src/config.rs`, `crates/conductor/tests/guard*.rs`,
`crates/conductor/tests/session_settings.rs`, `docs/design/conductor.md` § 7.
