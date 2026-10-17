---
format: aep.planning-md/3
id: story:session-pty-relay
kind: story
status: draft
title: A systemd-provider session runs its harness under a conductor-owned pty relay in place of script
relations:
- decomposes: epic:session-attach
revision: 1
---
# Story: session-pty-relay

## Outcome

A session started under the systemd provider runs its harness under a conductor-owned pty relay instead of `script`, and everything the provider did through `script` still works, so attach has something to connect to.

## Context

- The unit's launcher is `exec 3<>"$1" && exec script -qfec "$3" "$2" <&3`: `/bin/sh` holds the session's FIFO on fd 3 and `script` gives `claude` a pty and a typescript (crates/conductor/src/confine/systemd.rs:12-21, :72-74). Nothing but `script` holds the pty master, so no process can attach.
- `Provider::prompt` types the start prompt into the FIFO as one bracketed paste and Enter (systemd.rs:22-23); `Provider::stop` is `systemctl --user stop <unit>` (:24); `measure` reads the unit's cgroup (:31-32). These keep working.
- The unit's name, slice and limits do not change (systemd.rs:12-17, :92).
- The relay is a hidden subcommand of the same binary, so the pty and socket code needs no host program. The mechanism and the rejected alternatives are in `epic:session-attach`.
- The socket is `<state>/sessions/<name>.sock`, beside `.fifo` and `.typescript`.

## Acceptance

1. Spec first: model the relay's socket protocol (frames for output bytes, input bytes, window size and detach; the modes `read-write` and `watch`; the refusal reasons) in `spec/domains/session.yaml` beside `conductor.session.StartSession` (spec/domains/session.yaml:31), validated with `ess specify validate --path spec`; the code follows the model.
2. `session start` under the systemd provider starts the unit with `conductor session relay` as the harness's parent; no `script` process is in the unit's cgroup (`systemctl --user show -p ControlGroup`, then the cgroup's process list).
3. The prompt `Provider::prompt` types reaches the harness as before: a stand-in harness that echoes its input shows the bracketed paste and Enter.
4. `<name>.typescript` still grows with the harness's output, and the harness sees a pty (`tty` succeeds) of the size the provider sets.
5. `conductor session stop` ends the unit, the relay exits and `<name>.sock` is removed; after a relay killed without cleanup, the next `session start` replaces the stale socket and starts.
6. The socket is mode 0600 and a connection from another uid is refused (checked by the peer's credentials).
7. `measure` reads the same `memory.max`, `cpu.weight` and `pids.max` as before the change (the existing measurement tests pass).

## Out of Scope

The `attach` command (`story:session-attach-command`); any other provider.

## Ambiguities

- Whether the relay keeps the FIFO for conductor's own prompts or moves them to the socket. Smallest change: keep the FIFO; revisit if attach and prompt delivery must not interleave.
- Pty library: a small crate such as `rustix` or `nix` is a build dependency, not a host one; the implementor takes the one the workspace already carries.
