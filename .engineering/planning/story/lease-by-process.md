---
format: aep.planning-md/3
id: story:lease-by-process
kind: story
status: implemented
title: The guard tells the lease holder from another process carrying its session id
relations:
- decomposes: epic:operating-loop-hardening
- depends_on: story:lease-guard
scope:
- confidence: cited
  path: .claude/conductor-settings.json
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/guard.rs
- confidence: cited
  path: crates/conductor/tests/cli_tree.rs
- confidence: cited
  path: crates/conductor/tests/guard_hook.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T01:15:10Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T01:15:11Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T02:04:38Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

The security review of `story:lease-guard` (wave 09) found that the guard identifies the lease holder
by the PreToolUse payload's `session_id` alone (`crates/conductor/src/guard.rs`, the lease check).
A second live process carrying the holder's session id, such as the holder's session resumed by
hand with `claude --resume <id>` while the original runs, has its claim refused yet writes freely.
Resuming by hand is the incident `story:single-conductor-lease` was filed for.

## Acceptance

- `a_second_process_resuming_the_holder_s_session_is_denied_its_writes` in
  `crates/conductor/tests/guard_hook.rs`, committed in wave 09 with
  `#[ignore = "story:lease-by-process: …"]`, runs without the ignore and passes: the guard learns the
  calling session's process (the shipped settings pass the hook shell's parent pid to
  `conductor guard`) and denies a conductor-role write whose process is not the holder's while the
  holder lives.

## Scope

`.claude/conductor-settings.json`, `crates/conductor/src/guard.rs`, `crates/conductor/src/cli.rs`,
`crates/conductor/tests/guard_hook.rs`, `crates/conductor/tests/cli_tree.rs`.
