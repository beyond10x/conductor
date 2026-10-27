---
format: aep.planning-md/3
id: story:tool-smoke-install
kind: story
status: draft
title: A tool is installed machine-wide only after a smoke run, and the previous kept
relations:
- decomposes: epic:measured-self-improvement
revision: 3
---
## Why

On 2026-10-13 a gates release crashed every bot API call on the machine one hour after a fix release. (`docs/design/self-improvement.md`.) On 2026-10-14 an aep release, installed machine-wide, failed `aep plan artifact validate` in 30 stores, every running controller's repository among them, and was reversed 38 minutes later.

## Acceptance

- `conductor tools check <tool> <binary>` runs the given binary read-only from a scratch directory against every consumer the instance knows (aep: `plan artifact validate` in every store `aep plan workspace list` names; ess: `specify validate` in every specification of the newest snapshot; gates: one `api GET` of a repository; worktree: `doctor`), runs the installed binary on the same consumers, and prints one row per consumer whose answer is a new refusal; it exits 1 when one is new and 0 when none is; tests with fake tools.
- The check runs each consumer's own gate form, as its `Taskfile.yml` calls the tool: for example `ess specify validate --strict-requires`. A spec whose `requires` names an older minor is a refusal under that form. Test: a fake consumer whose Taskfile passes `--strict-requires`, and a new binary that passes the bare command but fails that form, exits 1 (a conductor NEED of 2026-10-15: ess 0.60.0 passed the bare probe and eight gates refused it).
- `task tools:install <tool> <version>` (or a conductor command) downloads, verifies the checksum, runs the check above, installs only when it exits 0, keeps the previous binary, and `tools:rollback <tool>` restores it; tests with fake tools.

## Scope

`Taskfile.yml` or `crates/conductor/src/tools.rs`, tests, `.agents/conductor-dev.md`.
