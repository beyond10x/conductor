---
format: aep.planning-md/3
id: story:release-verify
kind: story
status: draft
title: '`conductor release verify` checks a release with no model call'
relations:
- decomposes: epic:deterministic-steps
revision: 1
---
## Why

Releases are checked by reading the release page; one release shipped a crashing binary.

## Acceptance

- `conductor release verify --repository <repo> --tag <tag>` checks: the tag exists and was made by the bot, the release has every target archive and a `SHA256SUMS` that matches them, the notes are not empty, and (for conductor) the archive's binary runs `--version`; exit 0 or a list of failures; tests with a fake forge.

## Scope

`crates/conductor/src/release.rs` (new), spec (the command), tests.
