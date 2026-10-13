---
format: aep.planning-md/3
id: story:ssh-command-kept
kind: story
status: implemented
title: A snapshot keeps the user's core.sshCommand
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T13:47:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T13:47:19Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T14:20:53Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`story:fetch-without-prompts` gives every git command of a snapshot a `GIT_SSH_COMMAND` with batch
mode and a connect timeout. It kept a `GIT_SSH_COMMAND` the caller had exported, but when the
caller had none it set `ssh -o BatchMode=yes …`, which overrides a `core.sshCommand` in the
repository's or the user's git config: a key named there (`ssh -i <key>`) no longer reached ssh,
and the fetch failed as a member failure.

## What

With no `GIT_SSH_COMMAND` of the caller's, the collectors take the `core.sshCommand` git's config
gives in the directory the command runs in, and put batch mode and the connect timeout after it;
`ssh` only when neither names one. The caller's variable still wins, as it does in git.

## Acceptance

- `conductor_cli::collect::ssh_command` answers a repository's `core.sshCommand`, the caller's
  command over it, and treats a blank caller's command as none
  (`tests/collect_stores.rs`, `a_repositorys_core_ssh_command_is_kept_and_the_callers_wins`).
- The two cases of `story:fetch-without-prompts` stay green: every collector git command runs with
  `GIT_TERMINAL_PROMPT=0` and an ssh command in batch mode.

## Scope

`crates/conductor/src/collect/mod.rs` (`git_environment`, `ssh_command`),
`crates/conductor/src/collect/blockers.rs`, `crates/conductor/src/collect/repositories.rs`,
`crates/conductor/tests/collect_stores.rs`.
