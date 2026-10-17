---
format: aep.planning-md/3
id: story:session-list-attachable
kind: story
status: draft
title: conductor session list and task sessions show which sessions can be attached
relations:
- decomposes: epic:session-attach
- depends_on: story:session-pty-relay
revision: 1
---
# Story: session-list-attachable

## Outcome

`conductor session list` and `task sessions` show every session of the instance with its provider and whether `session attach` works on it, so the operator sees what to attach before trying.

## Context

- There is no `session list` command: the CLI has `session start` and `session stop` (crates/conductor/cli.yaml:306-314). `task sessions` prints name, kind, id and state from `claude agents --json` (Taskfile.yml:14-27); `story:sessions-systemd-kind` makes it print the provider from the session record.
- The dashboard's session rows come from the same list plus the current dispatch (crates/conductor/src/dashboard.rs:53-60, crates/conductor/src/dashboard/repo.rs:55-100), so the attachable flag is one more field there.
- A session is attachable when its relay's socket answers (`story:session-pty-relay`), not merely when its unit is active.

## Acceptance

1. Spec first: model `conductor.session.ListSessions` and its row (`name`, `role`, `provider`, `unit`, `attachable`, `state`) in `spec/domains/session.yaml`, and add `[session, list]` to `crates/conductor/cli.yaml`, validated with `ess specify validate --path spec`.
2. `conductor session list` prints one row per live session of the active instance, `attachable` yes for a systemd-provider session whose socket answers, else no with the reason (`background`, `no-socket`); `--format json` prints the same rows.
3. `task sessions` prints the attachable column from `conductor session list`; the ambiguity check of `story:sessions-systemd-kind` is unchanged.
4. A session of another instance on the host is not listed.
5. A test covers one session of each provider and a systemd session whose relay died.

## Out of Scope

Listing other instances' sessions; the dashboard's column (it reads the same field once it exists).

## Ambiguities

- `story:sessions-systemd-kind` edits the same Taskfile lines; deliver it first or merge the edits.
