---
format: aep.planning-md/3
id: story:session-attach-refuses-sessions
kind: story
status: draft
title: A session cannot attach to a session
relations:
- decomposes: epic:session-attach
- depends_on: story:session-attach-command
revision: 1
---
# Story: session-attach-refuses-sessions

## Outcome

`conductor session attach` refuses to run inside a conductor, controller or worker session, so a session cannot type into another session's pty; only the operator's own terminal attaches.

## Context

- Every session runs with `--permission-mode bypassPermissions` and the guard is the only check on its tool calls (README.md "Safety model in short"). A write into another session's pty would go around the rules the guard table lists, such as `SendMessage` only to `conductor` or a sub-agent (docs/design/conductor.md:226-240).
- The same table says "no `conductor` command that writes, only its views" (docs/design/conductor.md § 7); attach writes, so the guard must name it.
- The dashboard's answer endpoint set the pattern: the one write of the dashboard is guarded by a token and by a guard rule against sessions reaching it (crates/conductor/src/dashboard.rs:20-26).
- The provider puts the launch's variables in a session's environment (crates/conductor/src/confine/systemd.rs:15-18); a variable gives the client a cheap first refusal but can be unset, so it is not the control.

## Acceptance

1. Spec first: model the refusal `inside-session` among the attach refusals in `spec/domains/session.yaml` (the model of `story:session-attach-command`) and the guard rule in the guard's model, validated with `ess specify validate --path spec`.
2. The client exits 1 naming `inside-session` when the variable the provider sets is present, before it opens the socket.
3. The guard denies a Bash call whose command is `conductor session attach` (any flags, behind `env`, `sh -c` or `command`) from a conductor, controller or worker session, naming the rule, and records the denial as it records others. The same command from a terminal without the hook is not affected.
4. The relay refuses a connection whose peer process sits in any conductor session's cgroup, even from a client that skipped 2 and 3: a test connects from a process placed in a session unit's cgroup and is refused.
5. `website/docs/security/confinement.md` and the guard table in `docs/design/conductor.md` § 7 state the rule, and `task docs-check` passes.

## Out of Scope

Preventing a local process outside every session, running as the operator's account, from connecting: that is the operator.

## Ambiguities

- Peer-cgroup detection needs the peer's pid (`SO_PEERCRED`) and a cgroup read; it holds on Linux only, which is where the systemd provider exists.
