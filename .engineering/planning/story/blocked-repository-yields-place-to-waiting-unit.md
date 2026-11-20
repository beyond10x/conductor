---
format: aep.planning-md/3
id: story:blocked-repository-yields-place-to-waiting-unit
kind: story
status: active
title: A place a stop frees goes to the waiting unit of work, not to the Blocked repository
summary: A Blocked-only repository yields a place to the unit of work its own need opened, and is not respawned while any unit waits for a place
refs:
- provider: github
  reference: beyond10x/conductor#245
relations:
- decomposes: epic:routine-duties-in-loop
- informed_by: story:idle-controller-yields-place
- informed_by: story:stopped-controller-stays-stopped
scope:
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: cited
  path: crates/conductor/src/run/dispatch.rs
- confidence: cited
  path: crates/conductor/src/run/handlers.rs
- confidence: cited
  path: crates/conductor/tests/run.rs
- confidence: cited
  path: docs/design/conductor-run.md
- confidence: cited
  path: website/docs/concepts/the-loop.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T14:13:19Z", actor: "human:unknown", revision: 10}
- {from: "proposed", to: "active", at: "2026-11-20T14:13:20Z", actor: "human:unknown", revision: 11}
---
## Why

Issue #245 (filed against 1.19.2; unchanged on main at 1.20.1). With `controllers.max_working`
reached, repository `S` had a `Blocked` dispatch (its controller had reported `blocked handover`),
and repository `G` a `Sent` unit of work, opened by `S`'s `[NEED]`, waiting on `ControllerLimit`.
After `controller stop-controller` on `S`'s controller, the loop spawned `S` again instead of `G`;
only `controller pause-controller` let `G` start.

Read in the code on main:

- The watch tick runs its line handlers before any cycle (`crates/conductor/src/run.rs:516-518`).
  `session gone` restarts any repository with an open dispatch, `Blocked` included
  (`crates/conductor/src/run/handlers.rs:948-967`, `tick.restart` at `:965`; `Tick::open_dispatch`,
  `run.rs:768-782`).
- That start takes the freed place first come, first served: `Tick::restart_with` ->
  `Tick::start_again` -> `Tick::try_start` (`run.rs:1146-1190`, `:881`, `:818-860`), which checks
  only `make_room` and `at_limit` (`run.rs:834-837`, `:1222-1233`). The cycle's reconcile starts a
  gone controller's repository again the same way (`run/dispatch.rs:268`), before dispatch v1's
  units of work (`run/dispatch.rs:487`).
- So the place a stop frees goes to the `Blocked` repository before the unit of work that waits on
  `ControllerLimit` (`run.rs:1280-1288`) is tried, although the `Blocked` dispatch waits on that
  very unit: the unit's reference is `need-<repo>-<request>` (`run/mailbox.rs:763-766`), and
  nothing reads it when a place is given.
- A refused restart is an error to the watch tick: `restart_with` turns any refusal of
  `start_again` into `Err` (`run.rs:1176-1179`), which `handovers` passes up with `?`
  (`run/handlers.rs:344`, `:351`, `:362`), skipping the tick's later hand-overs.

## Outcome

- **Yield to the unit a need opened.** A start of a repository `R` whose every open dispatch is
  `Blocked` is not made while a unit of work that `R`'s need opened waits for a place: a `Sent`
  dispatch whose reference starts `need-` and whose brief, the whole `[NEED <repo> <request-id>]`
  message (`run/mailbox.rs:788-811`), names `R` as `<repo>` on its first line (read as
  `crate::message::read` reads a first line, so `alpha` never matches `alpha-two`), its repository
  with no `Running` or `Paused` controller, not left to the operator after its failed starts. This holds for every start through
  `Tick::try_start`: the watch tick's restart after `session gone`, a hand-over restart, the cycle's
  reconcile restart and dispatch v1's start of an Active repository.
- **No respawn of a Blocked repository while units wait.** A respawn of a repository whose every
  open dispatch is `Blocked` is not made while any unit of work waits as above and the free places
  under `controllers.max_working` do not exceed the units waiting. This covers a controller that
  reported `blocked handover` and was then stopped.
- A respawn is a start for which no controller was held when the start began. `restart_with` and
  the cycle's reconcile stop the held record before they start again (`run.rs:1153-1172`,
  `run/dispatch.rs:229`, `:268`), so the newest record is `Stopped` in every case by the time
  `Tick::try_start` runs; the caller says instead whether it held one: `restart_with` passes
  whether it found a `Running` or `Paused` record (a hand-over restart, or `session gone` while the
  record was still `Running`), reconcile passes that it held one (the record was `Running`), and
  dispatch v1's `Tick::spawn` passes none. A restart of a held controller (it gives up and takes
  back the same place in one step) keeps its place, but for the first rule; a `session gone` after
  `controller stop-controller` finds no held record, and is a respawn.
- The check runs inside the `STARTS` lock and before `make_room`, so no idle controller is stopped
  for a start that then yields. The refusal reads `max_working <n>: the place is kept for
  <dispatch> of <repo>`, naming the unit the first rule found, else the first waiting unit in store
  order. In the cycle it is the repository's `waiting` reason and its `<repo> waits: ...` part of
  the `cycle` line, as any refusal of `Tick::spawn`. In the watch tick it is no error: one journal
  line `<repo> waits: <refusal>`, the session forgotten (`handlers::forget`), and the tick goes on
  with its other lines and hand-overs. Any other refusal of `start_again` stays an error of the
  watch tick, as today (`run.rs:1176-1179`). A hand-over restart that yields has stopped the controller
  and leaves its dispatch `Blocked`.
- A repository with an open `Sent` or `Started` dispatch, and a `Blocked` repository when no unit of
  work waits or a place is left for it, start as today.

Decision (class C, planner, wave 70): a `Running` controller's hand-over restart does not yield to
an unrelated unit, so a routine context hand-over never loses its place; only the unit its own need
opened takes it. Undone by applying the second rule to hand-over restarts too.

Spec: no change. Only dispatch v1 writes the `waiting` field, as today, with a reason that is free
text (`spec/domains/dispatch.yaml:586-588`); `ess specify validate --path spec` stays green and
`crates/conductor-model` is not regenerated.

## Acceptance (tests that fail on the current code)

Fixture of the first three: `max_working: 1`; `alpha` spawned with an open dispatch, reported
`blocked handover ...` through `[REPORT alpha <id>]`; `beta` holds a `Sent` dispatch sent with
`--reference need-<repo>-r1` and the brief `[NEED <repo> r1] beta ...`, refused `ControllerLimit`
by a cycle. `<repo>` is `alpha` where a test says `alpha`'s need, else `gamma`.

- `tests/run.rs` `a_stopped_blocked_repository_yields_its_place_to_the_unit_its_need_opened`:
  `beta` is `alpha`'s need; `alpha`'s controller is stopped with `controller
  stop-controller`; a watch tick with `session gone: ex-alpha ...` launches nothing and its journal
  names `alpha waits: max_working 1: the place is kept for <beta's dispatch> of beta`; the next
  cycle says `spawned beta` and launches no `ex-alpha`.
- `tests/run.rs` `a_stopped_blocked_repository_is_not_respawned_while_another_unit_waits`: as
  above with `beta` `gamma`'s need; the watch tick launches nothing, its journal names
  `beta`'s dispatch, and the next cycle says `spawned beta`.
- `tests/run.rs` `a_handover_restart_yields_to_the_unit_its_own_need_opened`: `beta` is `alpha`'s
  need; `alpha`'s hand-over is asked (`context: alpha ...` over the threshold), a
  hand-over file is written and `alpha` goes idle; the next watch tick stops `alpha`, launches
  nothing, leaves its dispatch `Blocked`, writes no error for it, and the next cycle says `spawned
  beta`.
- `tests/run.rs` `a_handover_restart_keeps_its_place_while_an_unrelated_unit_waits` (new; passes on
  today's code, it guards the decision): as the previous with `beta` `gamma`'s need;
  the hand-over restarts `alpha` (`controller-restarted`, `ex-alpha` launched again).
- `tests/run.rs` `a_blocked_repository_takes_a_free_place_when_no_unit_waits` (new; passes on
  today's code, it guards the rule): `max_working: 1`; `alpha` spawned with a `Blocked` dispatch
  and no unit of work anywhere; `session gone: ex-alpha ...` writes `controller-restarted` and
  launches `ex-alpha`.
- `tests/run.rs` `a_gone_session_with_an_open_dispatch_is_respawned` passes unchanged.

## Boundaries

- Whether a controller stopped with `controller stop-controller` stays stopped, when no unit waits,
  is `story:stopped-controller-stays-stopped` (draft; it also scopes `run/dispatch.rs`, so it is not
  in the same wave as this story).
- `story:idle-controller-yields-place` is implemented on main (`Tick::make_room`); its order
  (`make_room` after the yield check) is the only change to it.
- Who starts a non-Active repository's `Blocked` dispatch after a refused restart is unchanged:
  today a restart refused at `max_working` is not retried either.
- The order of the units of work among themselves (store order) and of the Active repositories
  (`story:dispatch-v1-once-per-goal`) is unchanged; so is `at_limit`, which counts `Running`
  controllers. `controller pause-controller` keeps its meaning; no new command, flag or state.
- `crate::watch::{Options, Watch}` is used as it is; `crates/conductor/src/watch.rs` is
  `story:supervisor-watch-command`'s, and an API change there is rebased onto.
