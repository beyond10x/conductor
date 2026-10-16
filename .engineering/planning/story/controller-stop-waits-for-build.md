---
format: aep.planning-md/3
id: story:controller-stop-waits-for-build
kind: story
status: draft
title: Stopping a controller never kills its running build by accident
revision: 2
---
# Story: controller-stop-waits-for-build

## Outcome

Stopping or pausing a controller session never kills a build it is running by accident. The stop waits for the session's running builds to exit, or refuses and names them. A build that has to outlive its session can be started detached.

## Context

Conductor's NEED N-20261016-17, 2026-10-16: EKR's 0.0.36 full gate (`build run --reserve 42G … task check`, about 100 min in, 186 targets green) died at 19:08:03Z. Its controller session had handed over and was ended with `kill -TERM`, because `claude stop` does not reach a session that `conductor spawn` started. Ending the session's unit stops its cgroup, and `build run` passes the signal to its process group (story:build-run-cancel-group), so the build died with it. rules.md item 146 is the stopgap.

## Acceptance

1. `controller stop-controller` and `pause-controller` read the host ledger's Running grants of that session (`session_id`, recorded since 0.16.1). With any present they refuse with `SessionHasRunningBuilds`, naming each grant's id, command and age, unless given `--wait <dur>` (wait for them to exit, then stop) or `--force`. Spec first.
2. `conductor session stop --role …` behaves the same for conductor's own builds.
3. `build run --detach` starts the command in its own transient systemd unit (`conductor_build_<grant>.service`) outside the caller's cgroup, under the same ledger reservation and disk watchdog, and prints the grant id. `host grants` shows it as detached, and `build wait <grant>` waits for its exit and prints its exit code and log path. Without systemd it is refused (`DetachUnavailable`).
4. The repo-controller profile's hand-over step says: hand over after running builds exit, or name them; a long gate that must survive a hand-over is started with `--detach`.
5. Tests cover the refusal, `--wait` and a detached run surviving its caller's exit (a fake systemd-run where needed).

## Out of Scope

Moving process supervision into substrate (epic:process-supervision).

## Ambiguities

None.
