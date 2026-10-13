---
format: aep.planning-md/3
id: story:tool-smoke-install
kind: story
status: draft
title: A tool is installed machine-wide only after a smoke run, and the previous kept
relations:
- decomposes: epic:measured-self-improvement
revision: 1
---
## Why

On 2026-10-13 a gates release crashed every bot API call on the machine one hour after a fix release. (`docs/design/self-improvement.md`.)

## Acceptance

- `task tools:install <tool> <version>` (or a conductor command) downloads, verifies the checksum, runs the tool's smoke commands (for gates: `api GET` of a repository; for aep and ess: their validate commands), installs only when all pass, keeps the previous binary, and `tools:rollback <tool>` restores it; tests with fake tools.

## Scope

`Taskfile.yml` or `crates/conductor/src/tools.rs`, tests, `.agents/conductor-dev.md`.
