---
format: aep.planning-md/3
id: approval-record:wave-2026-11-14-conductor
kind: approval-record
status: approved
title: 'Conductor wave 2026-11-14: a boxed runtime dials its instance API out to conductord'
relations:
- decides: story:box-runtime-dials-conductord
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-11-14T20:52:22Z", actor: "human:agent", revision: 3, executor: "human:timo"}
---
# Conductor wave 2026-11-14: a boxed runtime dials its instance API out to conductord

## Approval

The operator approved this wave on 2026-11-14, in the session that built it: commit and push the
change on an integration branch with a planning item for every change, without a release.

## Units

| unit | artifact | note |
|---|---|---|
| U1 | `story:box-runtime-dials-conductord` | `daemon.dial_port`, the dial-in listener (`crates/conductor/src/daemon/dial.rs`), the runtime's `CONDUCTOR_API_DIAL`, and a local box's start naming the apertures `conductord` and `conductor`; `b10x-substrate-sdk` pinned to substrate `main` `befef3aa45c7d78436556c690a0125d85316b1e8`, which carries the SDK's aperture list (substrate `story:sdk-aperture-list`) |

## Terms

- **Gate:** the pull request's checks; nothing merges red and no check is overridden.
- **Release:** none in this wave; the release decision is separate.
- **Evidence:** `tests/placement.rs`, `tests/instance_api.rs` and the `daemon::dial` unit tests;
  an end-to-end run on a host with substrate-daemon 0.20.0: the instance `running`, no `api.sock`
  bind error, the instance API answering through conductord over the dialed connections, and a run
  in the instance's workspace reaching the model host through the aperture `conductor`.
