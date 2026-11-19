---
format: aep.planning-md/3
id: review-result:routine-duties-design-r1
kind: review-result
status: active
title: Design critic, round 1, epic routine-duties-in-loop
summary: 'changes: wrong failing-start path, invalid NEED reference, alert route that does not exist'
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
revision: 1
---
changes

Each story's mechanism was read against the code and spec. Three stories name a wrong mechanism or
a surface that does not exist.

**story:unit-merge-policy.** The key fits: `conductor.config.Controllers`
(`spec/domains/config.yaml:379-388`) is the instance's `controllers` section
(`spec/domains/config.yaml:1020`), `CONTROLLER_KEYS` in `crates/conductor/src/config.rs:905`.
Defects:
- `is_default_charter` is called at `crates/conductor/src/run.rs:800` (`Tick::spawn`), not at
  `run/dispatch.rs:405`, which is `merge_human`.
- The default charter is written once, when missing or empty (`default_charter`,
  `crates/conductor/src/spawn.rs:784-813`, called at `:182`), and stays in the records. A
  repository that already got the `human` form keeps it after the instance turns `controller`,
  which is the case of #207's running instances. The story must say that a charter equal to one
  rendered default form is rewritten to the form the setting names at the next unit start; any
  other charter is untouched.
- The controller profile's merge rule is `crates/conductor/roles/controller.md:48-51`; the
  `controller` form of the charter must also drop "never merge it" from its Boundaries
  (`spawn/charter.md`), not only the last line.
- The spec change re-pins `crates/conductor/http.yaml`'s `source_digest` (Rule 1), a surface the
  story must name.

**story:failing-start-recreates-box.** The failing path is not the `retry_after` back-off it
names. A boxed start first closes the stored box: `start_runtime` -> `close_strays`
(`crates/conductor/src/daemon/reconcile.rs:1460-1468`) -> `stop_runtime` -> `close_box`
(`:1617-1640`); when substrate answers that the workspace does not exist, `closed?` fails, the
entry stays in `boxes.json` and nothing is opened, so every pass fails the same way. The run-bound
path `keep_boxes` -> `restart_in_box` (`:723-835`) also reuses `opened.workspace` and leaves
`started_at` old on a refusal, so it retries every tick. The sound fix is small: a close or restart
substrate answers with its not-found name forgets the stored box (removes it, frees its slot) and
the same pass opens a new box. No "N failed starts" counter is needed. The instance state is
`Unreachable` (`spec/domains/instance.yaml:201`); there is no `failed` state.

**story:need-starts-unit-of-work.**
- The reference `need:<request-id>` is invalid: a reference is 1 to 64 characters of
  `A-Z a-z 0-9 . _ -` (`spec/domains/dispatch.yaml:799-805`). A request id is also unique only per
  sender.
- The spec already models a NEED becoming a dispatch: `RouteNeed` records the dispatch it opened on
  the message (`spec/domains/dispatch.yaml:601-607`, `:1973-2010`; design `docs/design/conductor.md:51-53`;
  `crates/conductor/src/message.rs:474`). The story must open the dispatch as the work call does
  (`crate::dispatch::send_work`, `crates/conductor/src/dispatch.rs:187`, which answers an existing
  reference with its dispatch) and route the message to it, not invent a parallel record.
- The unit start of the cycle needs a `Sent` dispatch with a reference
  (`crates/conductor/src/run/dispatch.rs:18-24`); it writes the default charter
  (`spawn.rs:181-183`). Good.

**story:loop-alerts-when-it-cannot-act.**
- "The instance's configured notification route" does not exist: the loop's operator route is the
  notifier `notify-send` (`crates/conductor/src/run/handlers.rs:13`, `:158-161`, `notify` at
  `:1284`), and wave 56 decided the notifier is not made configurable (`docs/waves/wave-56.md`,
  "A missing notifier gets a doctor row; it is not made configurable"). Use that route.
- "Recorded in the records' decisions log": an alert is no decision. The records' line for a loop
  event is a `DispatchLogEvent` (`spec/domains/dispatch.yaml:249-278`, e.g. `StartRefused`); one
  new variant `Alert` is the smallest record, spec first.
- The daemon's condition (a box that cannot be recreated) runs outside the loop, where no tick
  exists; the alert must be a module both call (`crate::alert`), and the daemon story calls it.

```findings
[
{"file":".engineering/planning/story/unit-merge-policy.md","line":33,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"is_default_charter is called at run.rs:800, not run/dispatch.rs:405 (merge_human)"},
{"file":"crates/conductor/src/spawn.rs","line":784,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"a default charter already written in human form is never rewritten, so the setting would not reach existing unit repositories"},
{"file":"crates/conductor/src/daemon/reconcile.rs","line":1467,"category":"judgement","severity":"blocker","verdict":"CONFIRMED","origin":"pre-existing","message":"the endless failure is close_strays/stop_runtime failing to close a gone stored box (and restart_in_box reusing it), not retry_after; story names the wrong mechanism"},
{"file":"spec/domains/instance.yaml","line":201,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"no failed instance state; a failing instance reads Unreachable"},
{"file":"spec/domains/dispatch.yaml","line":801,"category":"judgement","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"reference need:<request-id> contains a colon and is refused InvalidReference; request ids are unique only per sender"},
{"file":"spec/domains/dispatch.yaml","line":1973,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"RouteNeed already records the dispatch a NEED opened; the story must use it rather than a parallel record"},
{"file":"crates/conductor/src/run/handlers.rs","line":1284,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"no configurable notification route exists and wave 56 decided not to make the notifier configurable; use notify-send"},
{"file":"spec/domains/dispatch.yaml","line":249,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"an alert is not a decision; record it as a new DispatchLogEvent variant Alert, spec first"}
]
```
