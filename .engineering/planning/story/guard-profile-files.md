---
format: aep.planning-md/3
id: story:guard-profile-files
kind: story
status: implemented
title: The guard protects a role's profile file like its settings file
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T20:03:12Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T20:03:12Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T21:00:46Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

A role's `profile` is the system prompt its sessions start with (`--append-system-prompt-file`).
The guard protects every `.claude/*settings*.json` and each role's `settings` file, but not a role's
`profile`: a session whose file rules otherwise allow the path could rewrite another role's system
prompt. Found while implementing the `profile` key.

## Acceptance

- The guard denies a file-tool write and a Bash write form that names a configured role's
  `profile` file, for every session, as it does for `settings` files.
- Tests in `crates/conductor/tests/guard_rules.rs` beside the settings-file cases.

## Scope

`crates/conductor/src/guard/rules.rs`, `crates/conductor/tests/guard_rules.rs`.
