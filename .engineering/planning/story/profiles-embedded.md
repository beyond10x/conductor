---
format: aep.planning-md/3
id: story:profiles-embedded
kind: story
status: draft
title: The binary embeds its version's profiles and conductor profiles install writes them
relations:
- decomposes: epic:binary-only-operator
- informed_by: story:installed-profiles
scope:
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: inferred
  path: crates/conductor/src/cli.rs
- confidence: inferred
  path: crates/conductor/src/profiles.rs
- confidence: cited
  path: crates/conductor/src/upgrade.rs
- confidence: inferred
  path: crates/conductor/tests
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/guides/start-conductor.md
- confidence: cited
  path: website/docs/reference/cli.md
revision: 14
---
# Story: profiles-embedded

## Outcome

The `conductor` binary carries the session profiles of its own version, and `conductor profiles install` writes them to the profiles directory beside the config file. An operator with only the binary has the profiles the roles name, and `conductor upgrade` leaves them matching the new binary.

## Context

- `task profiles` copies `.agents/conductor.md`, `conductor-dev.md` and `repo-controller.md` from a checkout into `profiles/` beside the config file, `~/.b10x/conductor/profiles` unless `CONDUCTOR_CONFIG` names another (Taskfile.yml:329-332); `task install` runs it after `cargo install` (:323-327). Without a checkout nothing writes them.
- `story:installed-profiles` made that copy the way a restart reads a profile, so it matches the installed binary; this story moves the source of the copy from the checkout to the binary.
- `conductor upgrade` replaces the binary and keeps `conductor.previous` (crates/conductor/src/upgrade.rs:69; cli.yaml:336-343) and touches no profile, so a new binary starts on the previous version's profile.
- The profile files are renamed by `story:role-names-in-files` (`director.md`, `maintainer.md`, `lead.md`) once it lands; the embedded list is the files present in `.agents/` at build time, so this story does not depend on the names.
- The role's `profile` config key names a file path (Taskfile.yml:46-50); a path the operator wrote is read as today and never replaced.
- Shared surface: `crates/conductor/cli.yaml`, `crates/conductor-model` and `website/docs/reference/cli.md` with the other stories of the epic and with `story:demo-mode`; the merge cost is accepted and generated files are regenerated after the later merge.

## Acceptance

1. Spec first: model `conductor.config.InstallProfiles` (field `check`) with its result (each profile `written`, `unchanged` or `differs`) and refusal (`profiles-dir-unwritable`), and `conductor.config.DiffProfiles`, in `spec/domains/config.yaml`, validated with `ess specify validate --path spec`; add `[profiles, install]` and `[profiles, diff]` to `crates/conductor/cli.yaml`; `crates/conductor-model` is regenerated.
2. Every file of `.agents/*.md` is embedded with `include_str!`; a test fails when a file of `.agents/` is missing from the embedded list or the list names a file that is not there.
3. In a home with no checkout, `conductor profiles install` writes each embedded profile, mode 0644, into `profiles/` beside the config file (the directory of `CONDUCTOR_CONFIG` when set), prints `profiles installed in <dir>` and each file's state; the files are byte-identical to the embedded text.
4. A second run writes nothing and exits 0 with every file `unchanged`; a file edited by hand is overwritten by `conductor profiles install` without `--check`, and the output names it as `replaced`.
5. `conductor profiles install --check` writes nothing and exits 1 naming each profile that is missing or differs, and 0 when all match.
6. `conductor profiles diff` prints a unified diff of each differing installed profile against the embedded one, prints nothing when they match, and exits 0 in both cases.
7. `conductor upgrade` (not `--check`), after the new binary's `--version` is verified, runs the new binary's `profiles install` and prints its result; a failure of that step is reported and leaves the replaced binary in place.
8. `task profiles` runs `conductor profiles install` and nothing else; `task install` still ends in it.
9. `website/docs/reference/cli.md` is regenerated and `task docs-check` passes; `website/docs/guides/start-conductor.md` says the binary installs its profiles.

## Out of Scope

Changing a profile's contents; per-instance profile directories; merging a hand edit with a new version (the edit is replaced, or the install is not run); a role's `profile` path outside the profiles directory.

## Ambiguities

- Decided: `profiles install` overwrites a differing file because a profile must match the binary (`story:installed-profiles`); `--check` and `diff` are how an operator sees a hand edit first. The operator can ask for a refusal on a differing file instead.
- Open: whether `conductor upgrade --check` also reports profile drift; as written it does not, because `--check` writes nothing and reads only the release list.

## Scope

## Scope

Derived 2026-10-17 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor` (the CLI crate: new `profiles` command, `upgrade` hook) — cited
- **Files:**
  - `crates/conductor/src/upgrade.rs:232` (`replace(...)` ends `install`; the new binary's `--version` check is at :205-229, so the profiles step goes between them) — cited
  - `crates/conductor/src/upgrade.rs:69` (`pub fn upgrade`) — cited
  - `crates/conductor/cli.yaml:336-343` (`[upgrade]` binding; add `[profiles, install]` and `[profiles, diff]`) — cited
  - `spec/domains/config.yaml` (add `conductor.config.InstallProfiles`, `DiffProfiles`, result and `profiles-dir-unwritable` refusal; `UpgradeRefusal` sits at :734 as the pattern) — cited
  - `Taskfile.yml:323-332` (`install`, `profiles` tasks) — cited
  - `crates/conductor/src/cli.rs` (new `Group::Profiles`; dispatch near :428, `Access` table near :750, `upgrade` args near :1601) — inferred, from how `upgrade` and `config-workspace` are wired
  - new module `crates/conductor/src/profiles.rs` (`include_str!` of the 3 files in `.agents/`: `conductor.md`, `conductor-dev.md`, `repo-controller.md`; plus the completeness test) — inferred, new file
- **Symbols:** `upgrade::upgrade`, `upgrade::install`, `upgrade::replace`, `UpgradeRefusal`, `CONDUCTOR_CONFIG` — cited; `Group::Upgrade` in `cli.rs` — inferred
- **Also likely:** `crates/conductor-model/` (regenerated, never hand-edited) — cited, by AGENTS.md and the story; `crates/conductor/tests/` (new test for install, check, diff and upgrade) — inferred
- **Documents:** `website/docs/reference/cli.md` (generated, regenerated after the later merge) — cited; `website/docs/guides/start-conductor.md` (hand-written, add that the binary installs its profiles) — cited
- **Confidence:** high — the story cites Taskfile, upgrade.rs and cli.yaml lines, and I confirmed them. Medium for `cli.rs` and the new module's exact placement.
- **Would collide with:** any unit touching `crates/conductor/cli.yaml`, `spec/domains/config.yaml`, `crates/conductor-model` or the generated `website/docs/reference/cli.md` (the other `epic:binary-only-operator` stories and `story:demo-mode`, per the story); any unit touching `crates/conductor/src/upgrade.rs` or the `upgrade` wiring in `cli.rs`; any unit renaming or adding files in `.agents/` (`story:role-names-in-files`). A rename changes the embedded list, so the test must read the directory.
- **Safety fact:** the `upgrade` hook runs after the `--version` check and before the file swap, or after `replace` and with the old binary kept as `conductor.previous` (`upgrade.rs:232`, `replace` at :236+). Failure handling therefore cannot lose the working binary. The hook may only report a profile failure, not roll back, and the story says the replaced binary stays. Pointed at a `file:line` (level 2), unproven. The edit also reaches no config key: a role's `profile` path is read as before (Taskfile.yml:46-50), unproven.
