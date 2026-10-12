---
format: aep.planning-md/3
id: story:restart-current-profile
kind: story
status: active
title: A conductor start or restart refuses a profile checkout behind its upstream
relations:
- decomposes: epic:operating-loop-hardening
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T17:03:49Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-12T17:03:49Z", actor: "human:timo", revision: 4}
---
## Why

A conductor restart reads the role's `profile` from the product checkout named in the config. On
2026-10-12 `task conductor:restart` ran before that checkout was fast-forwarded to the merged
`main`, and conductor started on the previous profile; it had to be restarted again at once. The
restart takes four manual steps and nothing checks the order.

## Acceptance

- `task conductor:start` and `task conductor:restart` refuse, before stopping anything, when the
  git checkout holding the role's `profile` has an upstream and is behind it after a `git fetch`,
  naming the checkout and the command that fast-forwards it; with no upstream, or not a git
  checkout, they start as today. Tests in `crates/conductor/tests/taskfile.rs` (behind: refused,
  nothing stopped; even: started; no upstream: started).

## Scope

`Taskfile.yml`, `crates/conductor/tests/taskfile.rs`.
