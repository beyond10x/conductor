---
format: aep.planning-md/3
id: story:dashboard-broker-units
kind: story
status: draft
title: conductor dashboard and conductor broker start, stop and report their systemd user units
relations:
- decomposes: epic:binary-only-operator
- depends_on: story:profiles-embedded
- depends_on: story:tool-broker-read
- informed_by: story:host-dashboard-instances
revision: 2
---
# Story: dashboard-broker-units

## Outcome

`conductor dashboard start|stop|status` and `conductor broker start|stop|status` manage the instance's dashboard and broker as systemd user units, one pair per instance, with the logic that now sits in Taskfile shell. The operator starts, stops and asks about them with the binary alone.

## Context

- `task dashboard` starts the dashboard as the user unit `<session_prefix>-conductor-dashboard` (`conductor-dashboard` without a prefix) through `systemd-run --user`, setting `PATH`, `CONDUCTOR_INSTANCE` and `CONDUCTOR_CONFIG`, and where `systemctl` is absent runs it under `nohup` with a pid file in the state directory (Taskfile.yml:253-290); `task dashboard:stop` stops that unit or that pid (:291-317).
- Only `dashboard serve` exists as a command (crates/conductor/cli.yaml:184); the port is the instance's `dashboard_port`, 7313 plus the instance's index (crates/conductor/src/config.rs:1116).
- `story:host-dashboard-instances` adds the one host view with an instance switcher; the unit stays per instance, so it does not change this story.
- The broker is designed in docs/design/roles-and-surfaces.md section 6.2: it holds the forge credentials and the session's grant and serves a socket per instance. This tree has no broker task or crate yet (Taskfile.yml lists none, `crates/` holds conductor, conductor-docs and conductor-model), so the broker commands bind to the broker `epic:agent-tools` delivers; this story depends on `story:tool-broker-read`.
- `crates/conductor/src/doctor.rs:786` already asks `systemctl --user is-active` of a unit; the status command reuses that probe.
- Shared surface: `crates/conductor/cli.yaml`, `spec/domains/host.yaml`, `crates/conductor/src/cli.rs` and the generated reference with the other stories of the epic.

## Acceptance

1. Spec first: model the noun `conductor.host.ServiceUnit` (fields `service` as `dashboard` or `broker`, `instance`, `unit`, `state`), the commands `StartService`, `StopService` and `ServiceStatus` and their refusals (`already-running`, `not-running`, `no-systemd`, `port-in-use`) in `spec/domains/host.yaml`, validated with `ess specify validate --path spec`; add `[dashboard, start]`, `[dashboard, stop]`, `[dashboard, status]`, `[broker, start]`, `[broker, stop]` and `[broker, status]` to `crates/conductor/cli.yaml`.
2. `conductor dashboard start` creates the unit named as `task dashboard` names it, with the same environment, prints the URL, and is a no-op that exits 0 when the unit is active; `--instance` (or `CONDUCTOR_INSTANCE`) picks the instance, and two instances run two units on their two ports.
3. `conductor dashboard stop` stops the unit and exits 0 saying so when it was not running; `conductor dashboard status` prints `active` or `inactive`, the unit, the URL and the port, and exits 0 when active, 3 when not.
4. Without `systemctl` on `PATH`, `start` and `stop` fall back to the pid file as `task dashboard` does today and `status` reads the pid file; the pid-file cases of Taskfile.yml:272-290 are cases of the command.
5. `conductor broker start|stop|status` behave as items 2 to 4 for the instance's broker unit and socket, and `start` refuses naming the missing credential or grant when the broker has none to hold.
6. `conductor doctor` names `conductor dashboard start` and `conductor broker start` where it reports a unit not running (crates/conductor/src/doctor.rs:29).
7. `task dashboard`, `task dashboard:stop` and the broker tasks (where they exist) each run one `conductor` command; the tests of `crates/conductor/tests/taskfile.rs` that run them keep passing.
8. `website/docs/reference/cli.md` is regenerated and `task docs-check` passes.

## Out of Scope

A dashboard or broker unit for the whole host (`story:host-dashboard-instances` is the one host view; the unit stays per instance); service managers other than systemd beyond the existing pid-file fallback; the broker's own behaviour.

## Ambiguities

- Decided: `status` exits 3 when inactive, as `systemctl is-active` does, so a script can branch on it.
- Open: whether `dashboard start` should also open the browser; as written it prints the URL only, as the task does.
- Open: the broker is not in this tree yet; if `epic:agent-tools` names its command or unit differently, item 5 follows that name.
