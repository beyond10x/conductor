---
format: aep.planning-md/3
id: story:profile-zsh-shell
kind: story
status: implemented
title: The profiles say the shell may be zsh and how to quote for it
relations:
- decomposes: epic:loop-without-handwork
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T04:34:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T04:34:27Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-14T04:34:28Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

The sessions' shell is zsh. Conductor's one-liners failed twelve times between 2026-10-13 19:50Z and
2026-10-14 00:56Z: an unquoted `===` separator expands as `=cmd` (`=== not found`, exit 1, once on a
decision record), and an unmatched glob aborts the command (`no matches found`).

## Acceptance

- `.agents/conductor.md` and `.agents/repo-controller.md` say the shell may be zsh: quote a word
  that starts with `=`, and guard a glob that may match nothing (`(N)` or an `ls` test);
  `crates/conductor/tests/profiles.rs` holds the sentence in both.

## Scope

`.agents/conductor.md`, `.agents/repo-controller.md`, `crates/conductor/tests/profiles.rs`.
