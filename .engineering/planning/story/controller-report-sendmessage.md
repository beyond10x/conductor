---
format: aep.planning-md/3
id: story:controller-report-sendmessage
kind: story
status: implemented
title: A controller's report is a SendMessage call, not printed text
tags:
- session-review
scope:
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-09T01:36:47Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-09T01:36:47Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-09T03:29:30Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Controllers print their `[REPORT …]` as assistant text instead of sending it, so conductor never
sees it. Reported by conductor on 2026-10-08: three times in one evening, each on a short one-item
dispatch where the session ended its turn on the report; conductor found the merged PRs about 40
minutes later through the forge. Verified on 2026-10-09 in one of those controllers' transcripts:
the `[REPORT <repo> <dispatch-id>] done …` line is assistant text, and the transcript holds 0
`SendMessage` tool calls.

`.agents/repo-controller.md` § Talking says "Messages go only to `conductor`" and gives the first
lines, but never says that a message is a `SendMessage` call.

## Acceptance

- § Talking of `.agents/repo-controller.md` says: every `[REPORT]`, `[DECISION-REQUEST]`, `[NEED]`
  and `[RESOURCE]` is a `SendMessage` call to conductor; text printed in the session reaches nobody;
  a turn that ends on a report ends with that call.
- A test in `crates/conductor/tests/profiles.rs` finds that sentence in the profile.
- The profile diff goes to the operator; it merges after an `approval-record` from the operator
  `decides` this story.

## Scope

`.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`.
