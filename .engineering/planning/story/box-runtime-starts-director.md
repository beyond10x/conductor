---
format: aep.planning-md/3
id: story:box-runtime-starts-director
kind: story
status: draft
title: A boxed instance's runtime starts its director in the box from its profile and keeps it running
summary: 'Option B: the runtime keeps one director per boxed instance, backing off as the loop does; doctor and the dashboard name a missing one and why'
refs:
- provider: github
  reference: beyond10x/conductor#220
relations:
- decomposes: epic:architecture-next
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: crates/conductor-model
- confidence: inferred
  path: crates/conductor/dashboard
- confidence: inferred
  path: crates/conductor/http.yaml
- confidence: inferred
  path: crates/conductor/roles/director.md
- confidence: cited
  path: crates/conductor/src/alert.rs
- confidence: cited
  path: crates/conductor/src/dashboard/sessions.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/records.rs
- confidence: cited
  path: crates/conductor/src/role.rs
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: inferred
  path: crates/conductor/src/run/director.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/tests/box_session.rs
- confidence: cited
  path: crates/conductor/tests/dashboard_sessions.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: inferred
  path: crates/conductor/tests/profiles.rs
- confidence: inferred
  path: crates/conductor/tests/roles.rs
- confidence: inferred
  path: crates/conductor/tests/runtime_loop.rs
- confidence: inferred
  path: docs/design/conductor-run.md
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/dispatch.yaml
- confidence: cited
  path: spec/domains/session.yaml
- confidence: inferred
  path: website/docs/concepts/roles.md
- confidence: inferred
  path: website/docs/guides/run-isolated.md
revision: 5
---
## Why

Issue #220, expected 1 and the dashboard half of 4. `story:director-boxed-start` stopped a boxed
director from starting unboxed and gave doctor a row, but nothing starts one in the box. The
operator chose option B: the boxed runtime starts the director in the box from its profile and
keeps it running, one instance-level session per instance, and doctor and the dashboard name a
missing one.

On main:

- The runtime's loop never starts a director. A watch tick runs `handlers::pending`, the line
  handlers and the mailbox (`crates/conductor/src/run.rs:505-520`). A `session gone` line restarts
  only conductor-dev (`run/handlers.rs:977-1006`) or a controller with an open dispatch
  (`run/handlers.rs:947-962`). A director has no repository, so `repo_of` answers none and
  nothing happens.
- `session start --role director` runs only in the box (`session.rs:1760-1767`). In the box it
  starts through the `child` provider, because the box refuses Unix sockets
  (`confine/child.rs:1-40`).
- The binary carries no director profile (`role.rs:18-23`; `profile_name` answers `None` for the
  director at `role.rs:36`). So `harness_args` starts it with `--agent conductor`
  (`session.rs:1583-1590`), and the box has no such agent. This is the issue's "no agent named
  'conductor' — starting with the default template". A role's own `profile` file does not help in
  a box: the box reads no role file (`placement.rs:1506-1522`, the read list).
- Doctor's line says "nothing starts one there yet" (`doctor.rs:2560-2563`). The dashboard's
  health view runs `doctor::check_in` (`dashboard/health.rs:208`), which does not include the
  director lines (`doctor.rs:2509-2518`). The Sessions page shows the director row only as `not
  running`, with no reason (`dashboard/sessions.rs:178-183`, `:226-232`).
- The loop's back-off is `limit::backoff`: 5 minutes, doubling per attempt, at most 1 hour, and
  none for the first attempt (`limit.rs:27-41`). It is used when a session the loop resumed is
  stopped by a usage limit again.

## Outcome

- **The profile.** The binary carries `crates/conductor/roles/director.md`, a generic profile of
  at most 60 lines. It covers only what the loop does not: the start-of-session list (the
  Director section of `rules.md`, `NORTHSTAR.md`, `STATUS.md`, the newest hand-over under the
  records' `docs/handoff/`), answering the operator and the host supervisor, and decisions within
  its grant. It arms no timer, runs no watch, and relays or schedules nothing (`DIRECTOR_PROMPT`,
  `session.rs:1407`). `conductor role show --role director` prints it. The `claude-code` kind
  writes it to `<state>/sessions/<session>.profile.md`, which the box writes, so the session
  starts with `--append-system-prompt-file` and no `--agent`. A role whose own `profile` or
  `agent` is set keeps it.
- **The keeper.** Each watch tick of a runtime that runs in its instance's box
  (`collect::checkouts::in_box`) runs a keeper, `run/director.rs`, after `handlers::pending`. The
  keeper acts only when the instance has a director role (`config::director`). A director runs
  when the box's registry holds a live entry under one of `config::director_names`. When none
  runs and no wait is pending, the keeper runs `conductor session start --role director` the way
  `start_maintainer` does (`run/handlers.rs:1061-1092`). That covers `--config`, `--state-dir`,
  the daemon actor and the box's subscription token. The session starts in the box, in the
  records directory, under `<session_prefix>-director`.
- **"Keeps it running."** A director that ended, was stopped by hand, or never started is
  started again:
  - `attempts` counts the starts since the last session that lived at least
    `limit::BACKOFF_MOST` (1 hour). Before the next start the keeper waits
    `limit::backoff(attempts - 1)` after the last start.
  - So the first start, and the first restart after an end, happen at once, as a gone controller
    is restarted at once (`run/handlers.rs:947-962`). Each further early end waits 5, 10, 20, 40,
    then 60 minutes, as a session the loop resumed backs off.
  - A failed start (the command exits non-zero, or its session is gone by the next tick) counts
    as an attempt.
  - While `limited.json` marks the director's session, the keeper waits until that mark's resume
    instant and then starts a fresh session. It does not resume the old transcript.
  - Each start writes a dispatch log line `director-started`, with `session` and `reason`
    (`none ran`, `gone`, `failed: <line>`) and `tick_id`.
  - After 3 failed starts in a row, the keeper raises the alert `director-not-started` once while
    the failures last (`crate::alert`). It keeps trying at the back-off cap.
  - The keeper's state is written to `<state>/run/director.json` as a `DirectorKeep`. The only
    way to stop the keeping is to remove the director role from the instance's `roles`.
- **Doctor.** Doctor's director line reads `director.json`:
  - `director <instance>: <session> runs in its box`, or
  - `director <instance>: no director session runs in its box (<session>); the runtime starts it
    <at once | again at <t>>[: last start failed: <reason>]`.
  - When `director.json` is missing: `…; the runtime has not tried yet`.

  It is still a report line, not a failure.
- **The dashboard.** `/v1/data.json`'s director row (role `conductor`) carries
  `director: {state, attempts, next_start_at, reason}` from `director.json` for a boxed instance,
  and `null` for an unboxed one. The Sessions panel shows that reason in the row instead of a bare
  `not running`.
- **Unboxed instances are unchanged.** For placement `none`, or a `/1` file, the keeper does
  nothing: no start, no `director.json`, no `director-started` line. The director is still
  started by hand with `session start --role director`, and doctor and the dashboard show what
  they show today.

## Specification (modelled first)

- `spec/domains/session.yaml`:
  - `conductor.session.DirectorState`, an enum `[Running, Waiting, Limited]`.
  - `conductor.session.DirectorKeep`, a struct with `session` (`conductor.config.SessionName`),
    `state`, `attempts` (Integer), `started_at` (Optional Timestamp), `next_start_at` (Optional
    Timestamp) and `reason` (Optional String). It is the keeper's record in `<state>/run/`, which
    doctor and the dashboard read.
  - The `session start` comment names the runtime as the director's starter in a box.
- `spec/domains/dispatch.yaml`: `DispatchLogEvent` gains `DirectorStarted` (`director-started`)
  with its fields, and the `alert` conditions gain `director-not-started`.
- `spec/domains/agent.yaml`: the `ShowRole` comment lists `director` among the profiles the binary
  carries.
- Then `ess specify validate --path spec`, `task regen`, and re-pinning `source_digest` in
  `crates/conductor/http.yaml`. No new command, view or error: the keeper is loop behaviour, and
  `/v1/data.json` is not bound.

## Acceptance (fail on main)

- `tests/box_session.rs` `a_boxed_runtime_starts_its_director_in_the_box`: a runtime in the box
  (sockets refused, `CLAUDE_CONFIG_DIR` the box's) whose instance has a director role and no
  director session. Within one watch tick the box's registry holds a live entry
  `<session_prefix>-director` whose working directory is the records directory. Its arguments
  carry `--append-system-prompt-file` naming a file equal to `role show --role director`, and no
  `--agent`. The dispatch log holds `director-started` with reason `none ran`.
- `tests/box_session.rs` `a_boxed_runtime_starts_its_director_again_once_it_ends`: the stand-in
  session is killed, and the next tick starts it again with reason `gone`. After a second early
  end, `director.json` reads `Waiting`, `attempts` 2 and `next_start_at` = last start + 5
  minutes, and no third start happens in that tick.
- `crates/conductor/src/run/director.rs` unit tests, on an injected clock:
  `the_keeper_backs_off_after_starts_that_end_early` (0, 5, 10, 20, 40, 60, 60 minutes),
  `a_director_that_lived_an_hour_is_started_again_at_once`,
  `a_limited_director_waits_for_its_mark`, and
  `three_failed_starts_raise_director_not_started_once`.
- `tests/runtime_loop.rs` `an_unboxed_runtime_starts_no_director`: this holds on main and guards
  the boundary. No start, no `director.json`, no `director-started` line.
- `tests/doctor.rs` `doctor_says_when_the_runtime_starts_the_missing_director`: with a
  `director.json` in `Waiting` and a failure reason, the line names the next start and the reason.
  The text "nothing starts one there yet" is gone. Update
  `doctor_names_a_boxed_instance_without_a_director_session`.
- `tests/dashboard_sessions.rs` `the_director_row_names_why_no_director_runs_in_its_box`: the row
  carries `director` from `director.json` for a boxed instance, and `null` for an unboxed one.
- `crates/conductor/dashboard/tests/sessions.spec.ts`: the panel shows the reason and the next
  start in the director's row. A Sessions scenario state in `ux/fixtures.ts` holds a waiting
  director, and `task ui:ux-gate` passes on the rebuilt `dist/`.
- `tests/roles.rs`: `ROLES` gains `director`, so `the_role_profiles_live_under_…` and
  `role_show_prints_…` cover it. `tests/profiles.rs`: 5 profiles shipped, `director.md` capped at
  60 lines, and none of the retired loop words.

## Boundaries

- No hand-over at a context threshold for the director. The `context:` handler stays a
  controller's. An ended director starts fresh from its profile and the records' hand-over.
- Starting the session goes through `session start`; it is not moved onto `conductor.agent`'s
  `StartAgent`, which the runtime does not use yet.
- No mount for a role's own `profile` or `settings` file in the box. A director role that names
  one outside the box's roots fails its start, which the keeper reports and backs off.
- The keeper does not route the loop's mailbox or decisions to the director. Messages to
  `conductor` stay the loop's.
- The `conductor` role word on the dashboard and the `<session_prefix>-conductor` alias stay as
  they are (`story:controller-name-not-conductor`).

## Scope

- `spec/domains/session.yaml`, `spec/domains/dispatch.yaml`, `spec/domains/agent.yaml`;
  `crates/conductor-model/` (generated); `crates/conductor/http.yaml` (digest)
- `crates/conductor/roles/director.md` (new), `crates/conductor/src/role.rs`
- `crates/conductor/src/run.rs`, `crates/conductor/src/run/director.rs` (new),
  `crates/conductor/src/run/handlers.rs` (shared role start; `gone` leaves the director to the
  keeper), `crates/conductor/src/records.rs`, `crates/conductor/src/alert.rs`
- `crates/conductor/src/doctor.rs`
- `crates/conductor/src/dashboard/sessions.rs`; `crates/conductor/dashboard/src/types.ts`,
  `src/panels/SessionsPanel.vue`, `tests/sessions.spec.ts`, `ux/fixtures.ts`, `dist/`
- tests: `crates/conductor/tests/box_session.rs`, `runtime_loop.rs`, `doctor.rs`,
  `dashboard_sessions.rs`, `roles.rs`, `profiles.rs`
- docs: `AGENTS.md` (Who reads this, Layout: roles), `docs/design/conductor-run.md` § 4 (the
  director profile is back, for the boxed session only), `website/docs/concepts/roles.md`,
  `website/docs/guides/run-isolated.md`
