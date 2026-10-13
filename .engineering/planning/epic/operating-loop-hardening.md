---
format: aep.planning-md/3
id: epic:operating-loop-hardening
kind: epic
status: implemented
title: The operating loop moves without the host as its gate, and its gaps found on 2026-10-12 are closed
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T02:04:45Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T02:04:45Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T02:04:45Z", actor: "human:timo", revision: 5}
---
## Why

On 2026-10-12 the operating loop of conductor and conductor-dev stalled or leaked in ways the
product could have prevented:

- every merge waited on this host's disk: the full gate (`task check`) runs only locally, while CI
  builds the site and runs the docs crate's tests and Gates (`.github/workflows/pages.yml`,
  `shared-gates.yml`); one docs round merged without `task check`;
- session hand-overs with internal ids were committed to the public repository and caught only by
  review; history had to be rewritten;
- a conductor restart ran before the product checkout its profile is read from was fast-forwarded,
  so conductor started on a stale profile;
- two conductors ran at once; a controller could run a store write that passes as a read; an
  operator decision does not say which session recorded it;
- smaller frictions: session tasks miss prefixed sessions, fresh records cannot be snapshotted
  without an origin, `task agents:link` serves no session, the dashboard names a fixed context
  threshold, and a refusal message has a verb that does not agree with its subject.

## Outcome

The loop keeps moving without this host's disk as the gate, nothing internal reaches the public
repository unnoticed, a restart cannot start conductor on a stale profile, and the session and
authority gaps found on 2026-10-12 are closed.

## Stories

New: `story:ci-full-gate`, `story:public-text-check`, `story:restart-current-profile`.
Filed earlier the same day and decomposed under this epic: `story:single-conductor-lease`,
`story:guard-store-migrate`, `story:operator-decision-guarded`, `story:session-tasks-follow-prefix`,
`story:init-records-without-origin`, `story:agents-link-user-scope`.

Split out after the plan critics' round 1:

- the conductor lease is three stories: `story:single-conductor-lease` (the spec, `conductor
  session claim` and the role's `SessionStart` hook), `story:lease-guard` (the guard denies a write
  from a conductor session that does not hold the lease) and `story:watch-two-conductors` (`conductor
  watch` reports two live conductor sessions);
- `story:doctor-refusal-wording` takes the refusal wording out of
  `story:session-tasks-follow-prefix`, which now covers only the dashboard's context threshold.

Session-name resolution in the Taskfile tasks (`task sessions`, `conductor:attach`, `dev:attach`
missing prefixed sessions) is owned by `story:instance-scoped-tasks` of epic:multi-instance-host,
which joins wave 07 alongside this epic's stories; it is ordered after
`story:restart-current-profile` because both change `Taskfile.yml` and `tests/taskfile.rs`.

Not in this epic: a `conductor gc` command (rules for the cleanup-only session already name it) and
a `conductor hands` command for class-H items; both need a specification first and are later work.
