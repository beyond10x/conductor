---
format: aep.planning-md/3
id: story:quickstart-existing-repo
kind: story
status: active
title: 'A guide: one task in an existing repository, start to merged'
relations:
- decomposes: epic:onboarding-from-url
- depends_on: story:merge-human
- depends_on: story:controller-without-store
- depends_on: story:worktree-activation
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T16:33:56Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-13T16:33:57Z", actor: "human:timo", revision: 5}
---
## Why

Users new to conductor found it hard to start, with many tools and concepts at once, and want to try
it on an existing repository; the getting-started page starts from an empty instance. An evaluation
run on macOS (finding V8-3) also found that `ess generate synthesize` refuses files carrying
`com.apple.provenance`, with no word on it in the docs.

## What

- A guide, `website/docs/guides/first-task-existing-repo.md`: one task in an existing repository,
  start to merged — `conductor setup --apply` in the checkout, one goal, one dispatch, what the
  controller does, reviewing the pull request (with `merge: human` for stricter gates), and what to
  do when it stops. Every command exists; each step says what it prints.
- README's first screen and `SETUP.md`'s last step link it.
- The troubleshooting table gains the macOS row: `ess generate synthesize` refusing
  `com.apple.provenance`, its cause (files written by a process Claude Code started carry the
  attribute), and the workaround (run that generation from a plain terminal; the fix belongs to
  ess).

## Acceptance

- A new test, `crates/conductor/tests/docs_quickstart.rs` (as `docs_forge.rs` does for the GitLab
  guide), reads the guide, resolves every `conductor <group> <leaf>` it names against the CLI tree
  and fails on one that does not exist, and checks that `website/sidebars.ts` lists the guide
  first under guides.
- The same test checks that the guide names `merge: human`, the no-store path and the activation
  `to run:` line; that `README.md`'s first 12 lines link the guide while keeping the `SETUP.md`
  pointer `crates/conductor/tests/docs_setup.rs` holds; that `SETUP.md`'s last step links it; and
  that `website/docs/guides/troubleshoot.md` holds a row naming `com.apple.provenance`, its cause
  (files written by a process Claude Code started) and the workaround (a plain terminal).

## Notes

- Depends on `story:merge-human`, `story:controller-without-store` and `story:worktree-activation`,
  whose behaviour the guide describes.
- Shares `README.md` with `story:guard-write-targets`, which edits only its security section.

## Scope

`website/docs/guides/first-task-existing-repo.md`, `website/sidebars.ts`, `README.md`, `SETUP.md`,
`website/docs/guides/troubleshoot.md`, `crates/conductor/tests/docs_quickstart.rs`.
