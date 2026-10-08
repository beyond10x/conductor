---
format: aep.planning-md/3
id: story:spawn
kind: story
status: draft
title: Starting a controller launches its session
relations:
- depends_on: story:instance-session-names
revision: 1
---
# Starting a controller launches its session

## What

`conductor controller start-controller` (`story:controller-commands` records it) gains its
obligation, in `crates/conductor/src/spawn.rs` (today a `NotImplemented` stub). It runs these steps
in order:
1. It refuses when the latest snapshot shows another session bound to the repository.
2. It records `StartController`. The store refuses a second one per repository.
3. It starts
   `claude --bg -n <repo> --agent <controller agent> --model opus --settings <conductor checkout>/.claude/controller-settings.json "<charter text>"`
   with cwd `<checkouts root>/<repo>`.
4. It prints the background session id.

The controller runs in conductor's own permission mode (`--permission-mode` set from conductor's). A
session in another mode holds every cross-session message until its user approves it. Observed:
this held conductor's first hand-over, and it would make every `[DISPATCH]` wait for the operator.

The charter text is read from `charters/<repo>.md`, by repository name. There is no `--charter`
flag: the spec gives `StartController` no such input, and the CLI has no words beyond the spec
(an earlier wave's decision). The dispatching story writes the file (`story:pilot` for the pilot);
this story never writes charters. A missing charter file refuses the start before anything is
recorded.

`controller stop-controller` ends the session through `claude stop <id>` and records
`StopController`.

Depends on `story:controller-profile`, `story:decision-commands`, `story:status-board`,
`story:controller-commands` and `story:guard-hook`.

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
