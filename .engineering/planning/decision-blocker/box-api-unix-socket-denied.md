---
format: aep.planning-md/3
id: decision-blocker:box-api-unix-socket-denied
kind: decision-blocker
status: open
title: substrate 0.20.0 denies the Unix socket a boxed runtime binds for its API
relations:
- blocks: story:box-runtime-starts
withholds: test_result
revision: 1
---
## What is stopped

`story:box-runtime-starts` cannot record its live run: its acceptance asks for a runtime at
`placement: systemd` that serves its API on substrate-daemon 0.20.0, and the runtime ends at
`bind <state>/api.sock: Permission denied (os error 13)` in the box.

## What was observed

Wave 50, 2026-11-11, a scratch home, substrate-daemon 0.20.0 and a conductord built from
`wave/50` at `59c8f653f`, started as SETUP.md steps 6 to 10 write them (two runs; the second with
a state path short enough that the socket is `<state>/api.sock`):

- `conductor doctor` after substrate started: `box h: systemd, 4 writable roots served by the
  substrate daemon`.
- conductord: `h: Start (Cadence): Succeeded`, then `h: Start (LostRuntime): Refused: the runtime
  of h exited within 10 s of its start (failed start 1 in a row, the next start in 10 s):
  conductor: instance serve: bind <state>/api.sock: Permission denied (os error 13)`, then failed
  starts 2 and 3 with waits of 20 s and 40 s.
- `GET /v1/instances` at 10 s, 25 s and 45 s: `"state":"Unreachable"`.
- The store's sqlite files were written in the store root and `runtime.log` in the state root, so
  the mounts of the box hold.

substrate's changelog for the release states "seccomp denies Unix sockets", and
`substrate-daemon --help` of 0.20.0 has no flag for it.

## What would clear it

A decision on the transport of the instance API in a box, then the work of
`story:box-runtime-api-without-unix-socket`, then the live run of `story:box-runtime-starts`
repeated. The other way out is a substrate release whose box admits a Unix socket inside its own
writable roots; that reverses a hardening substrate accepted as a decision record, and is not
conductor's to decide.
