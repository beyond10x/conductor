---
format: aep.planning-md/3
id: review-result:routine-duties-design-r2
kind: review-result
status: active
title: Design critic, round 2, epic routine-duties-in-loop
summary: approve
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
revision: 1
---
approve

Each round-1 finding is resolved against the code and spec.

- `story:unit-merge-policy`: the call site is `run.rs:800`; the rewrite of a charter equal to
  either default form is an acceptance line; the controller form changes the Boundaries, not only
  the last line; `http.yaml` re-pin is in scope. The key sits in `conductor.config.Controllers`
  (`spec/domains/config.yaml:379`), per instance (`:1020`). `merge_human` (`run/dispatch.rs:406`)
  reads `merge: controller` as not human, so the review cap needs no change.
- `story:failing-start-recreates-box`: names the real path, `close_strays` -> `stop_runtime` ->
  `close_box` (`daemon/reconcile.rs:1460-1468`, `:1617-1640`) and `restart_in_box` (`:775-835`);
  forgetting a box substrate answers as gone is the smallest fix and reuses `start_runtime`, the
  existing row of failed starts (`failed_start`, `:1216`) and the `Unreachable` state
  (`spec/domains/instance.yaml:201`). The not-found name becomes one constant in `placement.rs`,
  beside the other refusal names (`placement.rs:121-2475`).
- `story:need-starts-unit-of-work`: the reference `need-<repo>-<request-id>` fits
  `spec/domains/dispatch.yaml:799-805` and is unique per sender; the dispatch is opened by
  `send_work` (`dispatch.rs:187`), whose reference idempotence gives "once"; the message is routed
  with `RouteNeed`, the spec's own record (`dispatch.yaml:1973`). No parallel model.
- `story:loop-alerts-when-it-cannot-act`: the route is the existing notifier (`run/handlers.rs:1284`),
  moved to `crate::alert`, with no config key (as wave 56 decided); the record is one new
  `DispatchLogEvent` variant, spec first (`dispatch.yaml:249-278`), beside `StartRefused`; the seen
  file follows the loop's `<state>/run/` files (`run/handlers.rs:22-36`). The daemon gets the same
  module, deduplicated in its own directory.

Note, not a finding: a boxed runtime's `notify-send` may be missing in the box; the alert is then
recorded and `conductor doctor`'s notifier row (`doctor.rs:1061`) names the gap, as the story says.

```findings
[]
```
