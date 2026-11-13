---
format: aep.planning-md/3
id: story:session-cannot-read-runtime-token
kind: story
status: draft
title: A session cannot read or use its runtime's API token
tags:
- from-finding
relations:
- informed_by: review-result:work-start-security-round-1
revision: 1
---
## Why

Found by the security review of the work call's start
(`review-result:work-start-security-round-1`, finding 1). Release 1.7.0 closed the variable;
the token itself stays reachable. Read at `v1.7.0` (`951330f4f4`).

- Closed in 1.7.0: `instance::command`, which builds every process a runtime starts, removes
  `CONDUCTOR_API_TOKEN` (`crates/conductor/src/instance.rs:63-68`), and an instance pod's
  `secret` volume lists its keys without the runtime's token
  (`crates/conductor/src/daemon/pods.rs:486-494`, `:515`).
- How a unit gets the token: the daemon mints it (`crates/conductor/src/daemon/reconcile.rs:1050`,
  `:1072`) and writes `CONDUCTOR_API_TOKEN=<token>` into the unit's environment
  (`reconcile.rs:1516-1523`); `systemd.rs:266-268` passes each variable as `-E` on the command
  line of `systemd-run`.
- How a pod gets it: the variable comes from the pod's Secret by `secretKeyRef`
  (`pods.rs:339-342`); the pod runs as one user (`pods.rs:504-508`) and shares one process
  namespace (`pods.rs:502`).
- How a box gets it: the daemon seals it into a slot (`reconcile.rs:1409-1455`), and the runtime
  reads the slot when it has no variable (`instance.rs:498-506`). Whether a session in the box
  can read that slot: not read.
- Where the runtime holds it: `instance serve` reads the variable (`instance.rs:502-505`) into
  the token of its API (`instance.rs:510-511`); no `remove_var` is under `crates/conductor/src`,
  so the variable stays in the runtime's own environment.
- A session runs as the runtime's user, so it can still read the token from the runtime
  process (its environment, its memory). Not tried on a host or a pod.

## Design first

The story needs a design before any build. Three options, none chosen:

- (a) The token reaches the runtime by an inherited file descriptor, not by the environment.
- (b) The runtime is not dumpable, so `/proc/<pid>/environ` and its memory are closed to a
  process of the same user.
- (c) Sessions run as a user of their own.

Open points, each open:

- A host where the daemon starts instances as units: a session runs as the daemon's user. A
  separation of users there is a design of its own, the operator's to accept; not started.
- A boxed placement: the daemon's config file is mounted read-only into the box
  (`crates/conductor/src/placement.rs:1360`, `:1378`) and names every instance and secret by
  name.
- The settings write of a runtime's API: `PUT /v1/settings`
  (`crates/conductor/src/dashboard.rs:1292`) is admitted by the token the daemon minted for the instance
  (`crates/conductor/src/dashboard/answers.rs:183-189`). It refuses only the seven `FIXED` keys
  (`crates/conductor/src/dashboard/configure.rs:40-48`, `:548-555`); `forge_token` and
  `model_token`, each the name of a secret, fall to `LoopRestart` in `effect`
  (`configure.rs:52-63`). Whether that write refuses a key that names a secret is open. It
  re-validates the file (`configure.rs:582`); which secret names validation refuses there: not
  read.

## Acceptance

- A design record weighs the three options with what each costs and what each leaves open on a
  pod, a unit host and a box.
- Nothing is built before the operator accepts that design.

## Notes

- `story:runtime-token-owner-private` holds the variable and the owner-private file; its
  variable point is closed in 1.7.0, its descriptor point stays.
