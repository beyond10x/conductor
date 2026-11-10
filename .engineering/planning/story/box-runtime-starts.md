---
format: aep.planning-md/3
id: story:box-runtime-starts
kind: story
status: active
title: 'A runtime at placement systemd starts by the documented steps: HOME and the mounts it needs'
scope:
- confidence: cited
  path: crates/conductor/src/daemon/reconcile.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: inferred
  path: crates/conductor/tests/placement.rs
- confidence: inferred
  path: website/docs/getting-started.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T23:54:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-09T23:54:17Z", actor: "human:timo", revision: 3}
---
## Why

Reported by the operator session of a trial instance on 2026-11-10 from conductor 1.0.1 with substrate-daemon 0.20.0,
on a throwaway instance at `placement: systemd`, set up by website/docs/getting-started.md
sections 3-6 (the reporter's defect list items 15 and 16). Both
are blockers for running an instance in the v1 architecture.

- The runtime has no HOME in the box and refuses to start: "instance serve: HOME is not an
  absolute path". The box gets only CONFIG, INSTANCE, STORE, API_SOCKET, PATH and API_LOG
  (reconcile.rs:938-953, placement.rs:625-635); no config key adds HOME.
- With HOME set by hand: "read the config file …: No such file or directory". The box mounts
  read-only the binary's directory, ~/.local/bin and the checkouts root, and writable
  `<state>/sessions` (placement.rs:636-650, 683-686). Not mounted: the config file, the sqlite
  store, the records, the rest of `<state>` (`run/lock`, `api.sock`), `/etc`, the forge CLI and
  its login, the target of the `claude` symlink.

## Acceptance

- A runtime at `placement: systemd` started by the documented steps serves its API: the box
  carries HOME and mounts what `instance serve` and the loop read and write (the config file,
  the store, the records, the state directory, the forge CLI and its login, the harness binary),
  each named in one list in placement.rs; a test against a fake substrate holds the box request,
  and the onboarding record holds one live run on substrate-daemon 0.20.0.
