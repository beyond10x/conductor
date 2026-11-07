---
format: aep.planning-md/3
id: story:guard-denial-names-its-config
kind: story
status: active
title: A denied forge write names the instance's route, its policy and the fix
summary: a gates route with no policy was read as a credential rule for two hours
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:21:16Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:21:17Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-11-07, relayed by the host supervisor: "make sure the issues are fixed and that
they are persistent in the sense that it's part of conductor and not of some outside rules".
Observed that day on an instance whose repositories are on a self-hosted GitLab, with
`forge.route: gates` and no `gates_policy`: controllers reported "glab writes denied" for about two
hours. The supervisor read the denial as an operator credential rule until the route was switched
to `direct`. The denial named neither the route nor the missing policy.

## Acceptance

- When the controller guard denies a `gh` or `glab` write, the denial names the instance's
  `forge.route`, whether `gates_policy` is set, and the one-line fix: `forge.route: direct`, or a
  policy for the bot route. A test in `crates/conductor/tests/guard_rules.rs` reads all three for
  `gates` with no policy, `gates` with a policy, and `direct` denying another repository's write.
- The denial stays one line and the guard stays within its latency bound (AGENTS.md rule 6): the
  route and the policy come from the places the guard already loads, with no store open.
