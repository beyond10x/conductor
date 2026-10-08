---
format: aep.planning-md/3
id: story:codex-controller-profile
kind: story
status: draft
title: Codex controller profile
revision: 1
---
# Codex controller profile

## What

A `codex-controller.md` profile, next to the Claude controller profile `.agents/repo-controller.md`,
carries that profile's rules for a Codex session. Messages go through `conductor mail`
(`story:mailbox`) instead of SendMessage. The profile has these sections:

| section | content |
|---|---|
| `## Spec first` | the paragraph between the `> Spec first:` line and the next blank line of `.agents/repo-controller.md`, word for word |
| `## Talking` | the § 5.5 first lines: `[REPORT`, `[DECISION-REQUEST`, `[NEED`, `[RESOURCE` |
| `## Boundaries` | own repository and managed worktrees only, GitHub writes through the bot route, no force-push |
| `## Builds` | the tree's own `target/`, never `CARGO_TARGET_DIR`, `worktree finish --discard-cache --archive` |
| `## Enforced by Codex` | each setting in `codex-controller.toml` and the Codex source it is read from |
| `## Instruction only` | each rule Codex cannot enforce |

`codex-controller.toml` holds the sandbox (repository checkout and its managed worktrees writable)
and the approval policy. Every key is read from the installed Codex's own documentation or
`--help`, never invented.

Depends on `story:pilot` and `story:mailbox`.

## Acceptance

1. Extracting the `> Spec first:` paragraph from both profiles with
   `awk '/^> Spec first:/,/^$/'` gives identical output.
2. `grep -c` finds each of `[REPORT`, `[DECISION-REQUEST`, `[NEED`, `[RESOURCE`, `CARGO_TARGET_DIR`,
   `force-push`, `conductor mail`, `managed worktrees`, `bot route`,
   `worktree finish --discard-cache --archive` and `## Instruction only` in `codex-controller.md`,
   at least once each.
3. Every key in `codex-controller.toml` appears in the `## Enforced by Codex` table with its
   source.
4. A Codex session started with the profile in a scratch repository under the checkouts root tries
   to write a file in the conductor checkout, and the sandbox refuses it.
5. That session runs one dispatch received through `conductor mail` to a `[REPORT … done]` that
   conductor records (`epic:rollout`'s Codex acceptance).
