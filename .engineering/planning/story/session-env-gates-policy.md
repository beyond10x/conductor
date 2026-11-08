---
format: aep.planning-md/3
id: story:session-env-gates-policy
kind: story
status: active
title: Sessions start with B10X_GATES_POLICY set to the instance's gates_policy
summary: 'Controllers on a gates-routed instance could not write to GitHub: no policy in their environment'
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T14:41:29Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T14:41:30Z", actor: "human:timo", revision: 4}
---
## Why

Host supervisor, 2026-11-08 15:40 local: an instance with `forge.route: gates` and `gates_policy`
set started its controllers without `B10X_GATES_POLICY`; a controller's `b10x-gates gh` and `api`
then refused ("protected file … unavailable: not found (B10X_GATES_POLICY/KEY unset)"), and it
could not rerun its own main CI. The same command with `--policy <gates_policy>` worked. In
`crates/conductor/src` only `broker.rs` reads `B10X_GATES_POLICY`; no session start sets it.

## Acceptance

1. `conductor session start` and the loop's spawn set `B10X_GATES_POLICY=<gates_policy>` in the
   environment of every controller and of the maintainer when the instance names a
   `gates_policy`, and set nothing when it does not. Test for both.
2. `.agents/repo-controller.md` says the policy is in `B10X_GATES_POLICY` and is never passed by
   hand; pinned in `tests/profiles.rs`, within the 120-line cap.

## Ambiguities

None.
