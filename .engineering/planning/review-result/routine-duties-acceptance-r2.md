---
format: aep.planning-md/3
id: review-result:routine-duties-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, epic routine-duties-in-loop
summary: approve
relations:
- reviews: story:unit-merge-policy
- reviews: story:failing-start-recreates-box
- reviews: story:need-starts-unit-of-work
- reviews: story:loop-alerts-when-it-cannot-act
revision: 1
---
approve

Every acceptance line now names a surface a test reads and a test that fails on main.

- `story:unit-merge-policy`: three tests in `tests/work_start.rs` (controller form written;
  human-form default rewritten; edited charter untouched): all fail on main, where the key is
  unknown and `default_charter` writes only a missing charter. `config validate` naming the YAML
  path and `config show` printing the key are read by the existing config tests.
- `story:failing-start-recreates-box`: three in-file tests with a fake substrate
  (`daemon/reconcile.rs` already holds substrate fakes): close answered not-found, restart answered
  not-found, recreated box refused over three passes with one alert. On main the first two record
  `Refused` every pass and open nothing; the third raises nothing.
- `story:need-starts-unit-of-work`: three tests in `tests/mailbox.rs`; the reference is named
  exactly, the message state (`Handled`, `dispatch_id`) is a view the test reads, "sent twice opens
  one" is checkable. On main the NEED is kept in `pending.json` and no dispatch exists.
- `story:loop-alerts-when-it-cannot-act`: one test in `tests/run.rs`, whose fake `notify-send`
  logs each call (`tests/run.rs:20`, `:621`): exactly one `alert` line and one notifier call over
  five cycles. On main the variant does not exist.

```findings
[]
```
