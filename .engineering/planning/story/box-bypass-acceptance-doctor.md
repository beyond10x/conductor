---
format: aep.planning-md/3
id: story:box-bypass-acceptance-doctor
kind: story
status: draft
title: doctor checks that a box's harness state accepts bypassPermissions, and setup says how
summary: For a boxed instance, conductor doctor reads <state>/claude for the bypass-permissions acceptance its controllers need, and the docs name the one setup step
tags:
- from-finding
relations:
- decomposes: epic:architecture-next
- informed_by: story:bg-launch-reads-user-settings
- informed_by: story:first-run-bypass-disclaimer
scope:
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: inferred
  path: website/docs/guides/run-isolated.md
revision: 1
---
## Why

Agentbox friction logs F22 and F23 (2026-11-15). With 1.12.0 every controller spawn in the box
failed `LaunchFailed: --bg with bypassPermissions requires accepting the disclaimer first` and
8 controllers stopped. The box's `CLAUDE_CONFIG_DIR` (`<state>/claude`, from
`story:box-forge-work`) had never seen the acceptance, and doctor said nothing. The operator
put `skipDangerousModePermissionPrompt: true` into `<state>/claude/settings.json`, the box's
user settings. That works on 1.13.0.

Read at v1.13.0:

- Conductor knows the acceptance lives only in user or managed settings
  (`crates/conductor/src/confine/mod.rs:17-20`, 91-130). `story:first-run-bypass-disclaimer`
  made the host start refuse with instructions.
- No code checks the box's harness state: `skipDangerousModePermissionPrompt` and
  `bypassPermissions…Accepted` appear nowhere in `crates/conductor/src`.
- `doctor`'s box rows are forge, git and trust (`doctor.rs:1661-1700`).

## Outcome

- For a boxed instance, `conductor doctor` reads `<state>/claude/settings.json` (and the
  managed settings the box reads) and reports a failure when no source accepts
  bypassPermissions. The row names the file and the key.
- `run-isolated.md` names the step.

## Acceptance

- A boxed instance whose `<state>/claude/settings.json` lacks the acceptance gets a doctor
  failure that names that file and key. With the key set, the row passes. Doctor tests hold
  both.

## Boundaries

- Conductor does not write the acceptance itself: accepting the disclaimer stays the
  operator's act.
- F23 attributed the lost key to conductord's trust write. At v1.13.0 that write is an
  order-preserving merge that keeps every other key (`crates/conductor/src/trust.rs:72-82`,
  `:283-290`), and it was the same at 1.12.0. The key belongs in settings, not in
  `.claude.json`, which Claude Code itself rewrites. Nothing about the trust write is in this
  story.
