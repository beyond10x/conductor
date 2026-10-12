---
format: aep.planning-md/3
id: story:spawn
kind: story
status: draft
title: Starting a controller launches its session
relations:
- depends_on: story:instance-session-names
- depends_on: story:guard-all-tools
- decomposes: epic:session-confinement
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
- confidence: cited
  path: spec/domains/dispatch.yaml
revision: 5
---
# Starting a controller launches its session

## What

`conductor spawn <repo>` starts a repository's controller session. The conductor profile names it
as the form to use once it exists (`.agents/conductor.md:181`) and spells out the form it replaces
(`.agents/conductor.md:179-180`). It is a one-word command with no group, as `trust`, `init` and
`doctor` are (`crates/conductor/src/cli.rs:33-35`, bound in `crates/conductor/cli.yaml:64-65` and
`:160-162`). Its launch lives in `crates/conductor/src/spawn.rs`, which answers `NotImplemented`
today (`crates/conductor/src/spawn.rs:14-30`). It runs these steps in order:
1. It refuses when a controller of the repository is `Running` or `Paused`. That is the
   `already-running` outcome `StartController` already declares
   (`spec/domains/dispatch.yaml:393-413`), so nothing launches.
2. It records `StartController`, as `controller start-controller` does today
   (`crates/conductor/src/controller.rs:53-78`). The session name comes from
   `crate::config::session_name` (`crates/conductor/src/config.rs:700`).
3. It starts, with cwd `<checkouts root>/<repo>`, the profile's start form
   (`.agents/conductor.md:180`):
   `claude --bg -n <session name> --append-system-prompt-file <controller profile> --model <controller model> --permission-mode <conductor's mode> --setting-sources project,local --settings <controller settings> "<charter text>"`.
   Profile, model and settings come from the `controller` role in `conductor config show`
   (`.agents/conductor.md:182-185`).
4. It prints the background session id.

The controller runs in conductor's own permission mode. The profile says why: a controller in
another permission mode holds every message until the operator approves it
(`.agents/conductor.md:190-191`).

The charter text is read from `<records>/charters/<repo>.md`, by repository name, and reaches the
command as one argument, never through a shell (`.agents/conductor.md:188-190`). There is no
`--charter` flag: the CLI has no words beyond the spec. This story never writes charters. A
missing charter file refuses the start before anything is recorded.

Ending the session (`claude stop <id>`, then `StopController`) is the second stub in `spawn.rs`
(`crates/conductor/src/spawn.rs:23-30`). It lands with this story.

## Workspace trust

`claude --bg` in an untrusted checkout answers "Workspace not trusted" and starts nothing (observed
when a controller was started). `conductor trust` marks every git checkout under the instance's
checkouts root, and its records directory, as trusted in `~/.claude.json`
(`crates/conductor/src/trust.rs:1-11`; input `conductor.config.TrustWorkspaces`,
`spec/domains/config.yaml:343-352`). Before launching, `spawn` reads that file without writing
it, through the same reader (`crates/conductor/src/trust.rs:183`). A checkout that is not marked
trusted is refused before anything is recorded, and the refusal names `conductor trust`.

## Acceptance

Spec first: `spawn`'s input and refusals (already running, no charter, untrusted checkout) are in
`spec/domains/dispatch.yaml` and the spec validates. Then `crates/conductor/tests/spawn.rs`, with
`claude` replaced by a recording stand-in against a scratch repository and charter, shows that
`conductor spawn <repo>` starts one session with the controller start form built from the config
(session name, profile, model, settings, conductor's permission mode, cwd the checkout), refuses
while a session of that name runs (launching nothing and recording nothing), and records the
controller `Running` in the store, as `controller controllers` lists it.

## Scope

`spec/domains/dispatch.yaml` (spec first), `crates/conductor-model/`,
`crates/conductor/src/spawn.rs`, `crates/conductor/src/cli.rs` (the `spawn` leaf and its entry in
`LEAVES`), `crates/conductor/cli.yaml` (the binding), `crates/conductor/tests/spawn.rs` (new).
