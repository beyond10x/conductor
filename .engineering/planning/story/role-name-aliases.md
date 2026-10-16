---
format: aep.planning-md/3
id: story:role-name-aliases
kind: story
status: draft
title: The config and the binary know director, maintainer and lead, and read the old role names as aliases for one release
relations:
- decomposes: epic:role-names
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/dashboard/series.rs
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/init.rs
- confidence: cited
  path: crates/conductor/src/rules.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: crates/conductor/tests/init.rs
- confidence: cited
  path: crates/conductor/tests/rules.rs
- confidence: cited
  path: crates/conductor/tests/session.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/reference
revision: 3
---
# Story: role-name-aliases

## Outcome

The config and the binary know the roles by their new names, `director`, `maintainer` and
`lead`, and still read `conductor`, `conductor-dev` and `controller` for one release, each noted
by `conductor config validate`. This is the expand step: nothing that names an old role breaks.

## Context

Design: `docs/design/roles-and-surfaces.md` § 2 (levels and names) and § 7 step 1, approved by the
operator on 2026-10-16; `docs/design/conductor.md` §§ 2, 9 and 11. Today `conductor.config.Role`
(`spec/domains/config.yaml`) documents the roles `conductor`, `conductor-dev` and `controller`,
the default `roles` list holds those three, and `session_name` is derived from the role name.
The old names are read in `crates/conductor/src/config.rs`, `session.rs`, `watch.rs`,
`status.rs`, `init.rs`, `cli.rs`, `dashboard/series.rs` and `guard/rules.rs`.

## Domain relations

- Old role name → new role name, one-to-one, the new name owns the meaning; the alias lives one
  release and is removed after it — inferable from `docs/design/roles-and-surfaces.md` § 7 step 1
  (operator-approved design; no `ess/1` document declares the alias yet, and this story declares it
  first in `spec/domains/config.yaml`).
- Instance → Role, one-to-many, the instance owns its roles — inferable from
  `spec/domains/config.yaml`, `conductor.config.Instance.roles: List<conductor.config.Role>` (a
  field, not a `relations:` entry).

## Acceptance

1. Spec first: `spec/domains/config.yaml` documents the role names `director`, `maintainer` and
   `lead`, the alias table (`conductor` → `director`, `conductor-dev` → `maintainer`, `controller`
   → `lead`), and the default `roles` list under the new names; `ess specify validate --path spec`
   and `task regen` pass, and `task docs-generate` refreshes `website/docs/reference/**`.
2. `conductor config validate` on a file whose role is `controller` exits 0 and prints a note
   naming its YAML path, such as `instances[0].roles[2].role: controller is the old name of lead`;
   `conductor config show --format json` prints that role as `lead`, and its `session_name` as
   `<session_prefix>-lead` for `director` and `maintainer` alike (`<prefix>-director`,
   `<prefix>-maintainer`). Test: `crates/conductor/tests/config.rs`.
3. `conductor session start --role conductor` and `--role director` start the same role, the old
   one with a note on standard error; `session stop` and `spawn` find a live session still named
   `<prefix>-conductor` as the director's for this release. Test: `crates/conductor/tests/session.rs`.
4. The guard treats a live `<prefix>-conductor` and `<prefix>-director` alike as the instance's
   director: a lead's SendMessage to either is allowed, to another instance's is denied as today.
   Test: `crates/conductor/tests/guard_rules.rs`.
5. `conductor rules show --section director` reads a `rules.md` section headed `conductor`, with a
   note, and `conductor init` writes the sections under the new names. Tests:
   `crates/conductor/tests/rules.rs`, `crates/conductor/tests/init.rs`.

## Scope

`spec/domains/config.yaml`, `crates/conductor-model` (generated), `crates/conductor/src/config.rs`,
`crates/conductor/src/session.rs`, `crates/conductor/src/watch.rs`,
`crates/conductor/src/status.rs`, `crates/conductor/src/init.rs`, `crates/conductor/src/cli.rs`,
`crates/conductor/src/dashboard/series.rs`, `crates/conductor/src/guard/rules.rs`, the tests named
above, `website/docs/reference/**` (generated).

## Out of Scope

- Renaming the profile, adapter and settings files, the Taskfile tasks and the hand-written docs
  (story `role-names-in-files`).
- Moving the maintainer out of `instances[].roles` (decision-blocker `maintainer-host-config`).
- Renaming the product, the binary or CLI groups such as `controller start-controller`.
- Removing the aliases (story `role-alias-removal`).
