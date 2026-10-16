---
format: aep.planning-md/3
id: story:role-names-in-files
kind: story
status: draft
title: Profiles, adapters, settings files, tasks and docs use the new role names, with the old ones kept as links and task aliases for one release
relations:
- decomposes: epic:role-names
- depends_on: story:role-name-aliases
scope:
- confidence: cited
  path: .agents
- confidence: cited
  path: .claude/agents
- confidence: cited
  path: .claude/conductor-settings.json
- confidence: cited
  path: .claude/controller-settings.json
- confidence: cited
  path: .claude/director-settings.json
- confidence: cited
  path: .claude/lead-settings.json
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: website/docs
revision: 4
---
# Story: role-names-in-files

## Outcome

Profiles, adapters, settings files, Taskfile tasks and the hand-written docs use the names
director, maintainer and lead; the old file and task names still work for one release.

## Context

Design: `docs/design/roles-and-surfaces.md` § 2 and § 7 step 1; `docs/design/conductor.md` § 11
(Profiles) and § 12. Today: `.agents/conductor.md`, `.agents/conductor-dev.md`,
`.agents/repo-controller.md`; adapters of the same names in `.claude/agents/`;
`.claude/conductor-settings.json` and `.claude/controller-settings.json`; tasks
`conductor:start|restart|attach`, `dev:start|attach`, `sessions`, `agents:link`, `profiles` in
`Taskfile.yml`. A running instance's config names the old profile, agent and settings paths, so the
old names must keep resolving.

## Domain relations

- Role → profile file, one-to-one, the role names the file through `roles[].profile` — inferable
  from `spec/domains/config.yaml`, `conductor.config.Role.profile` (a field, not a `relations:`
  entry).

## Acceptance

1. The profiles are `.agents/director.md`, `.agents/maintainer.md` and `.agents/lead.md`, the
   adapters `.claude/agents/director.md`, `maintainer.md` and `lead.md`, and the settings files
   `.claude/director-settings.json` and `.claude/lead-settings.json`; each old path stays for one
   release as a symbolic link to its new file. Test: `crates/conductor/tests/profiles.rs` reads the
   new paths and checks each old path resolves to its new file.
2. `task profiles` installs the new names into `~/.b10x/conductor/profiles/` and the old names as
   links to them, so a config naming `profiles/repo-controller.md` still starts a lead.
3. `Taskfile.yml` has `director:start`, `director:restart`, `director:attach`, `maintainer:start`
   and `maintainer:attach`, reading the roles `director` and `maintainer` from
   `conductor config show`; `conductor:*` and `dev:*` are `aliases:` of them. Test:
   `crates/conductor/tests/taskfile.rs` checks the tasks and the aliases.
4. The guard protects the new settings file names as it does the old ones. Test:
   `crates/conductor/tests/guard_rules.rs`.
5. `AGENTS.md`, `README.md`, `docs/design/conductor.md`, `docs/config.md` and the hand-written
   pages under `website/docs/` use the new role names, with one line in each saying the old names
   stay for one release; `task docs-check` and `crates/conductor/tests/no_organization.rs` pass.

## Scope

`.agents/`, `.claude/agents/`, `.claude/*-settings.json`, `Taskfile.yml`, `AGENTS.md`, `README.md`,
`docs/design/conductor.md`, `docs/config.md`, `website/docs/` (hand-written pages),
`crates/conductor/src/guard/rules.rs` (protected settings names),
`crates/conductor/tests/profiles.rs`, `crates/conductor/tests/taskfile.rs`,
`crates/conductor/tests/guard_rules.rs`.

## Out of Scope

- The role names in the config and binary (story `role-name-aliases`).
- Shortening the profiles (story `profile-slimming`); this story renames, it does not rewrite.
- `docs/design/roles-and-surfaces.md`, which already uses the new names.
