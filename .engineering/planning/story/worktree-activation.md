---
format: aep.planning-md/3
id: story:worktree-activation
kind: story
status: active
title: Doctor and setup check the worktree workspace activation; controllers have a fallback
relations:
- decomposes: epic:onboarding-from-url
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T16:33:56Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T16:33:57Z", actor: "human:timo", revision: 4}
---
## Why

An evaluation run on v0.8.0 (2026-10-13, finding V8-1) found that `worktree create` refused with
`workspace-not-activated` for the instance's checkouts root. Neither setup nor the controller
profile gave a way on, so conductor used plain `git worktree add` under the trees root to avoid
`worktree activate` writing the user's global config.

## What

- `conductor doctor` prints, for each instance of the config file, whether its checkouts root is
  inside an activated worktree workspace (`worktree doctor --json` or `worktree status --json`,
  whichever answers it), as `worktree workspace: <root> activated` or `… not activated`, with an
  `  install:`-style line naming the command that activates it; not activated makes doctor `not
  ready`.
- `conductor setup --apply` prints that command as a `to run:` line; it does not run it (it writes
  the user's config).
- `.agents/repo-controller.md`: when `worktree create` refuses with `workspace-not-activated`, the
  controller reports it as `[NEED conductor] worktree workspace not activated: <root>` and, until
  answered, works in a plain `git worktree add <trees>/<repo>/<name>` tree it removes when done.

## Acceptance

- `crates/conductor/tests/doctor.rs`: with a fake `worktree` that answers "not activated" for the
  checkouts root, doctor prints the line, the activation command, and `not ready`; answering
  "activated", the line and `ready`.
- `crates/conductor/tests/setup.rs`: `--apply` prints the `to run:` line and does not call the
  activation.
- A profile test holds the fallback sentence.

## Scope

`crates/conductor/src/doctor.rs`, `crates/conductor/src/setup.rs`,
`crates/conductor/tests/doctor.rs`, `crates/conductor/tests/setup.rs`,
`.agents/repo-controller.md` (one paragraph), `crates/conductor/tests/profiles.rs`.
