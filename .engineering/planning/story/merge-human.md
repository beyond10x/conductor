---
format: aep.planning-md/3
id: story:merge-human
kind: story
status: active
title: 'A charter''s merge: human makes the controller open the pull request and never merge'
relations:
- decomposes: epic:review-first
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T16:33:56Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-13T16:33:57Z", actor: "human:timo", revision: 5}
---
## Why

See `epic:review-first`: some work cannot be handed fully to agents and needs stricter gates. Today
a controller merges its integration branch once its full gate is green; no charter can say a person
merges.

## What

A charter may carry the line `merge: human` (default `merge: controller`). With it, the controller
runs its gate as before, opens one pull request, never merges it and never enables auto-merge, and
reports `[REPORT <repo> <dispatch>] review <url>`; the dispatch stays open until a snapshot shows the
pull request merged. Conductor's charter section names the line and its two values, and conductor
answers a controller's request to merge in such a repository with "a person merges here".

## Acceptance

- `.agents/repo-controller.md`'s delivery section holds `merge: human`, "never merges", "never
  enables auto-merge", the `review <url>` report and "the dispatch stays open until"; a test in
  `crates/conductor/tests/profiles.rs` holds each phrase.
- `.agents/conductor.md`'s charter section holds `merge: human`, `merge: controller` and "a person
  merges here"; the same test holds them.
- `website/docs/guides/start-conductor.md` names `merge: human`; the same test reads the guide for
  it.

## Not covered

The guard does not enforce the line: it reads no charter today. `story:merge-human-guard` (draft)
adds a charter reader and denies a merge call.

## Scope

`.agents/repo-controller.md`, `.agents/conductor.md` (charter section only),
`crates/conductor/tests/profiles.rs`, `website/docs/guides/start-conductor.md`.
