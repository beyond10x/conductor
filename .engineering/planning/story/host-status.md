---
format: aep.planning-md/3
id: story:host-status
kind: story
status: archived
title: One command prints what runs across every instance of the host
summary: 'A row per instance: run unit, snapshot age, Active repositories, open dispatches, live sessions'
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T10:28:15Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-11-07: "what is currently running across all the instances. It could just be
a subcommand in the conductor binary itself".

Today the answer takes four tools, once per instance: `systemctl --user list-units 'conductor_*'
--all`; `claude agents --json` filtered by each instance's `session_prefix`;
`CONDUCTOR_INSTANCE=<name> conductor status` for the snapshot age (`conductor status --instance`
is refused: "unexpected argument '--instance'", as is `conductor repository activity
--instance`); and `conductor dispatch dispatches --format json` with `conductor repository
activity` for open dispatches and Active repositories.

## Proposal

`conductor host status` (name open): one row per instance of the host registry
(`~/.b10x/conductor/host/instances.json`):

- whether the instance's run unit is active;
- the newest complete snapshot's age, or `none`;
- Active repositories, and how many of them the operator marked;
- open dispatches by state;
- live sessions of the instance: name, role, busy / waiting / stopped, and whether a dialog holds
  it;
- whether its conductor-dev session is live.

`--format json`, and `--instance <name>` to narrow it to one.

## Acceptance

On a host with three instances (one with its run unit active and a 35-minute-old snapshot, one
with no run unit and a 22-day-old snapshot, one with no complete snapshot and two live sessions
conductor did not start), the command prints three rows that say exactly that, and `--instance`
prints one.

## Archived 2026-11-09 (draft sweep)

Host registry and per-instance run units removed (docs/design/architecture-next.md section 11); the daemon's instances list answers it (docs/design/architecture-next.md section 5).
