---
format: aep.planning-md/3
id: story:box-runtime-api-without-unix-socket
kind: story
status: draft
title: A boxed runtime serves the instance API without a Unix socket
scope:
- confidence: cited
  path: crates/conductor/src/instance.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
revision: 3
---
## Outcome

A runtime in a substrate box serves the instance API, and conductord reaches it, without a Unix
socket inside the box.

## Why

`instance serve` binds `<state>/api.sock` (`crates/conductor/src/instance.rs`), and the relay
session of a remote box connects to that socket. substrate-daemon 0.20.0 denies Unix sockets in
a box by its seccomp profile (its changelog: "seccomp denies Unix sockets"), and no daemon flag
changes that. The live run of wave 50 on substrate-daemon 0.20.0 (`docs/analysis/onboarding-run.md`,
"Wave 50 live run") shows it: the box opens with its four writable roots, the runtime writes
`runtime.log` into its state directory, and then ends with
`instance serve: bind <state>/api.sock: Permission denied (os error 13)`. The cases of
`story:box-runtime-starts` hold the box request against a fake substrate, which admits the bind.

The transport is open: substrate 0.20.0 gives a box's session its own input and output and an
egress aperture, so the candidates are an exchange over the runtime session's input and output,
or a loopback TCP listener inside the box if the box's network admits one. The design names the
choice before code is written, and `Substrate::exchange` and the relay session follow it.

## Acceptance

Given substrate-daemon 0.20.0 started as SETUP.md step 9 writes it and a config with one instance
at `placement: systemd`, when conductord starts, then within 60 s `GET /v1/instances` answers the
instance as `Running` with a `box_id`, `conductor status` answers from the runtime, and
`runtime.log` holds no bind error; the onboarding record holds that run.

Given the same for a box on a remote daemon, when conductord relays one API exchange, then the
exchange answers without any process in the box binding or connecting a Unix socket.
