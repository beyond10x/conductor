---
format: aep.planning-md/3
id: story:spawn
kind: story
status: draft
title: Starting a controller launches its session
relations:
- depends_on: story:instance-session-names
- depends_on: story:guard-all-tools
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
- confidence: cited
  path: docs/analysis/spawn-run.md
- confidence: cited
  path: spec/domains/dispatch.yaml
revision: 3
---
# Starting a controller launches its session

## What

`conductor controller start-controller` (`story:controller-commands` records it) gains its
obligation, in `crates/conductor/src/spawn.rs` (today a `NotImplemented` stub). It runs these steps
in order:
1. It refuses when the latest snapshot shows another session bound to the repository.
2. It records `StartController`. The store refuses a second one per repository.
3. It starts, with cwd `<checkouts root>/<repo>`, the command the profile's controller start
   line names today (`.agents/conductor.md:172`):
   `claude --bg -n <session name> --append-system-prompt-file <controller profile> --model <controller model> --permission-mode <conductor's mode> --setting-sources project,local --settings <controller settings> "<charter text>"`.
   The session name is the controller's `session_name` (`story:instance-session-names`); profile,
   model and settings come from the controller role in `conductor config show`.
4. It prints the background session id.

The controller runs in conductor's own permission mode (`--permission-mode` set from conductor's). A
session in another mode holds every cross-session message until its user approves it. Observed:
this held conductor's first hand-over, and it would make every `[DISPATCH]` wait for the operator.

The charter text is read from `<records>/charters/<repo>.md`, by repository name. There is no `--charter`
flag: the spec gives `StartController` no such input, and the CLI has no words beyond the spec
(an earlier wave's decision). The dispatching story writes the file (`story:pilot` for the pilot);
this story never writes charters. A missing charter file refuses the start before anything is
recorded.

`controller stop-controller` ends the session through `claude stop <id>` and records
`StopController`.

The command, `StartController` and its record already exist (`spec/domains/dispatch.yaml:388`,
`crates/conductor/src/controller.rs`); this story fills `spawn.rs`, which answers `NotImplemented`
today.

## Workspace trust

`claude --bg` in an untrusted checkout answers "Workspace not trusted. Run `claude` in <path> once
and accept the trust prompt, then retry." (observed when a controller was started).
`~/.claude.json` holds `projects["<path>"].hasTrustDialogAccepted`; observed: five checkouts of one
instance read `false`.

`controller start-controller` reads that field read-only before launching; `false` is refused with
an outcome that names the class-H step; a missing entry launches, and the "Workspace not trusted"
answer is recorded as that same outcome. Spec first.

## Acceptance

Against a scratch repository under the checkouts root, with a scratch `charters/<repo>.md`:
- start launches one session that `claude agents --json` lists with that cwd and name;
- that session's first message to conductor starts with `[REPORT <repo> `, which shows the profile
  resolved (moved here from `story:controller-profile`);
- a `[DISPATCH]` sent to it is delivered without a hold: no `[Cross-session delivery notice]` saying
  it was held;
- a second start is refused, and nothing new launches;
- stop ends the session, and the controller is recorded `Stopped`;
- with `hasTrustDialogAccepted` `false` for the checkout, start is refused with the class-H outcome
  and nothing launches.

## Scope

`spec/domains/dispatch.yaml` (the trust outcome, spec first), `crates/conductor-model/`,
`crates/conductor/src/spawn.rs`, `crates/conductor/tests/spawn.rs` (new),
`docs/analysis/spawn-run.md` (new, the live acceptance against a scratch repository). After
`instance-session-names` (the session name) and `guard-all-tools` (shared generated crate); before
`instance-scoped-tasks` for the same reason.
