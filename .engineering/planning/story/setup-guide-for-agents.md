---
format: aep.planning-md/3
id: story:setup-guide-for-agents
kind: story
status: implemented
title: SETUP.md is the step list an agent follows to set conductor up
relations:
- decomposes: epic:onboarding-from-url
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: SETUP.md
- confidence: cited
  path: crates/conductor/tests/docs_setup.rs
- confidence: cited
  path: crates/conductor/tests/no_organization.rs
- confidence: cited
  path: website/docs/getting-started.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T13:13:49Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-13T13:14:01Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-13T13:19:36Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Why

An agent handed the conductor URL lands on `README.md`, which explains the product but gives no
step list an agent can follow in another repository (`epic:onboarding-from-url`).

## Acceptance

- `SETUP.md` at the repository root is a numbered list of steps; each step has a `Check:` line (a
  command whose output says whether the step is already done, so the list is safe to repeat), a
  `Fix:` line (the exact command), and a `Hands:` line (`none`, or what only the person can do: a
  login, an install that needs root, approving a store migration). The steps: detect the forge
  from `git remote get-url origin` (GitHub, GitLab with its host, or none); install `gh` or `glab`
  and log it in to that host; install conductor from the release archive with its `SHA256SUMS`
  check (or `cargo install` without a matching archive); install `aep`, `ess` and `worktree` at the
  pins conductor was built with (`conductor doctor` names them); check the repository's planning
  store version and, when it is not current, show `aep plan store migrate git --dry-run` and ask
  the person before `--verify`; write the instance config for the repository; `conductor init`;
  `conductor trust`; `conductor doctor` until it ends `ready`.
- The first lines of `README.md` say: an agent asked to set conductor up in a repository follows
  `SETUP.md`, with its raw URL.
- A test in `crates/conductor/tests/docs_setup.rs` reads `SETUP.md` and finds, for every step, a
  `Check:`, a `Fix:` and a `Hands:` line; finds the README pointer; and checks that every
  `conductor` command named exists in the CLI tree and every other tool named is in `b10x.toml` or
  is one of `git`, `gh`, `glab`, `cargo`, `curl`, `tar`, `sha256sum` and `shasum`.
- `crates/conductor/tests/no_organization.rs` also reads `SETUP.md`.

## Notes

- `story:doctor-identity` and later stories of the wave also land on
  `website/docs/getting-started.md`, `SETUP.md` and `docs_setup.rs`; they depend on this story.

## Scope

`SETUP.md` (new), `README.md`, `website/docs/getting-started.md`,
`crates/conductor/tests/docs_setup.rs` (new), `crates/conductor/tests/no_organization.rs`.
