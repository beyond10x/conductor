---
format: aep.planning-md/3
id: story:worktree-create-own-instance-trees
kind: story
status: active
title: A controller's worktree create lands under its own instance's trees
scope:
- confidence: inferred
  path: SETUP.md
- confidence: inferred
  path: crates/conductor/roles/controller.md
- confidence: inferred
  path: crates/conductor/src/doctor.rs
- confidence: inferred
  path: crates/conductor/src/session.rs
- confidence: inferred
  path: crates/conductor/src/setup.rs
- confidence: inferred
  path: crates/conductor/src/spawn.rs
- confidence: inferred
  path: crates/conductor/tests/docs_setup.rs
- confidence: inferred
  path: crates/conductor/tests/doctor.rs
- confidence: inferred
  path: crates/conductor/tests/profiles.rs
- confidence: inferred
  path: crates/conductor/tests/setup.rs
- confidence: inferred
  path: crates/conductor/tests/spawn.rs
- confidence: inferred
  path: website/docs/guides/first-task-existing-repo.md
- confidence: inferred
  path: website/docs/guides/troubleshoot.md
revision: 22
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:09Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:09Z", actor: "human:timo", revision: 6}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from conductor 1.0.1 (the reporter's defect list item 7). A lead's `worktree create` landed under
another instance's trees (`trees/<other instance>/<repo>`, the other instance's worktree profile), which the guard
then refused; `conductor doctor` reported the workspace activated. Workaround: plain `git worktree
add` under `trees/<instance>/<repo>/`.

## Acceptance

- A controller's `worktree create` lands under its own instance's trees; the session carries the
  instance's worktree profile, and doctor reports a workspace whose profile is another
  instance's; tests hold both.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor` worktree-workspace activation and its doctor row (`doctor.rs`, `setup.rs`) — inferred, the story names no path; the tree has one activation surface
- **Files:** `crates/conductor/src/doctor.rs`, `crates/conductor/src/setup.rs`, `crates/conductor/src/spawn.rs`, `crates/conductor/tests/doctor.rs`, `crates/conductor/tests/setup.rs` — inferred
- **Symbols:** `doctor::Activation`, `doctor::activation_of`, `doctor::activation`, `doctor::Workspace`, `doctor::workspace_line`, `doctor::read_config` (`roots`), `doctor::profile_name`, `doctor::profile_file`, `doctor::profile_text`, `setup::activate`, the `env` vector of `spawn.rs:450-466` — inferred
- **Also likely:** `crates/conductor/src/session.rs`, `crates/conductor/tests/spawn.rs`, `crates/conductor/roles/controller.md`, `crates/conductor/tests/profiles.rs`, `crates/conductor/tests/docs_setup.rs` — inferred; `session.rs:1171` sets the same instance environment for non-controller sessions, `controller.md` holds the `workspace-not-activated` fallback sentence that `profiles.rs` pins, `docs_setup.rs` pins the SETUP.md wording
- **Documents:** `SETUP.md`, `website/docs/guides/troubleshoot.md`, `website/docs/guides/first-task-existing-repo.md` — inferred, each quotes the `worktree workspace: <root> activated / not activated` row a new doctor state would add to
- **Confidence:** medium — the doctor half is read from the tree (`doctor.rs:1030`, `doctor.rs:1081`, `setup.rs:580`); the "session carries the profile" half has no mechanism in `worktree` 0.19.4, so its landing file is a guess
- **Would collide with:** any unit touching `doctor.rs`'s workspace rows or `Doctor::lines`, `setup.rs`'s `apply`/`activate`, the controller launch environment in `spawn.rs`, or the doctor-row wording in `SETUP.md` and `website/docs/guides/troubleshoot.md` — inferred
- **Safety fact:** `worktree` picks the profile whose `workspace_root` is the deepest ancestor of the repository (`worktree/crates/worktree-state/src/lib.rs:191-192`) and reads no per-session profile selector, so a second profile on the same root cannot be selected by the session; the change is safe only if each instance's root resolves to its own profile — step 2, unproven — inferred
