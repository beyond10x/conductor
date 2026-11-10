---
format: aep.planning-md/3
id: story:checkout-name-differs-from-project
kind: story
status: active
title: A checkout named differently from its forge project is one repository under one name
relations:
- decomposes: epic:forge-neutral
scope:
- confidence: cited
  path: crates/conductor/src/collect/gitlab.rs
- confidence: cited
  path: crates/conductor/src/collect/mod.rs
- confidence: inferred
  path: crates/conductor/src/collect/repositories.rs
- confidence: inferred
  path: crates/conductor/src/collect/sessions.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/decide/handle.rs
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: inferred
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: inferred
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: crates/conductor/tests/collect_gitlab.rs
- confidence: inferred
  path: crates/conductor/tests/config_workspace.rs
- confidence: inferred
  path: crates/conductor/tests/run.rs
- confidence: inferred
  path: crates/conductor/tests/spawn.rs
- confidence: inferred
  path: docs/design/conductor.md
- confidence: inferred
  path: spec/domains/config.yaml
- confidence: inferred
  path: website/docs/guides/gitlab.md
revision: 32
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:01Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:01Z", actor: "human:timo", revision: 9}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 from the first conductor 1.0.1 run of a trial instance on a self-hosted GitLab. The checkout directory `checks` holds GitLab project `example-checks`.
The workspace.yaml member is `checks`, while `conductor repository activity` names the repository
`example-checks`. `charters/checks.md` plus a dispatch to `checks` started nothing. Workaround used:
a second clone under the project's name.

## Acceptance

- A checkout whose directory name differs from its forge project's name is one repository under
  one name across workspace.yaml, snapshots, `repository activity`, charters and dispatch; a test
  with a checkout `checks` of project `example-checks` gets a controller started by a dispatch.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor` — cited, the GitLab checkout-to-repository naming: collector, workspace members, spawn
- **Files:** `crates/conductor/src/collect/gitlab.rs` `crates/conductor/src/config.rs` `crates/conductor/src/spawn.rs` `crates/conductor/src/run.rs` `crates/conductor/src/collect/mod.rs` — cited, read in the tree: `gitlab.rs:930` (`checkouts`, holds both `path` and `project`), `config.rs:6247` (member named by `checkout.path`), `spawn.rs:689` (`root.join(repository)` then `repository_of` must equal it), `run.rs:798` (`Tick::checkout`), `collect/mod.rs:235` (`repository_of` names a checkout by its directory)
- **Symbols:** `gitlab::checkouts` `gitlab::Checkout` `Forge::mapped` `config::each_checkout` `config::workspace_members` `spawn::checkout` `Tick::checkout` `Tick::charter` `collect::repository_of` `collect::repository_at` — cited
- **Also likely:** `crates/conductor/src/collect/repositories.rs` `crates/conductor/src/collect/sessions.rs` `crates/conductor/src/watch.rs` `crates/conductor/src/run/handlers.rs` `crates/conductor/src/decide/handle.rs` `crates/conductor/tests/spawn.rs` `crates/conductor/tests/run.rs` `crates/conductor/tests/config_workspace.rs` `crates/conductor/tests/collect_gitlab.rs` — inferred, `repositories.rs:496` sets the snapshot name from the project; `sessions.rs:570`, `watch.rs:2840`, `run/handlers.rs:302`, `decide/handle.rs:1080` each map a name to `<checkouts.root>/<name>` or back; the tests are where a `checks` / `example-checks` case would sit
- **Documents:** `spec/domains/config.yaml` `docs/design/conductor.md` `website/docs/guides/gitlab.md` — inferred, `config.yaml:87` and `:889` and `conductor.md:361-364` state the two naming rules that disagree
- **Confidence:** medium — the three defect sites are read in the tree, but the story does not say which name wins, and that decides how many directory-keyed consumers change
- **Would collide with:** any unit touching GitLab checkout discovery in `collect/gitlab.rs`, workspace member naming in `config.rs` (`each_checkout`, `workspace_members`), `spawn.rs` checkout resolution, `run.rs` `Tick` helpers, or `collect::repository_of` (shared by guard, sessions collector, watch, dashboard)
- **Safety fact:** for a `github` or `local` source the directory name is the repository name by construction (`repositories.rs:445-446`, `config.rs:6229-6245`), so a change confined to the GitLab arm cannot rename any non-GitLab repository — step 2, unproven; `collect::repository_of` is also read by the guard (`guard/rules.rs`) to decide whose checkout a session is in, so a change there moves a permission boundary and needs proving
