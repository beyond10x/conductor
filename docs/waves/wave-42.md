# Wave 42 (the loop as a program, part 1)

Epic: conductor-run, wave 1. Integration branch `takeover/run-daemon`, one branch per unit. Target:
v0.19.0. Sources: [`../design/conductor-run.md`](../design/conductor-run.md) and the measured
before, [`../analysis/takeover-baseline-2026-10-17.md`](../analysis/takeover-baseline-2026-10-17.md).

## Units

| unit | story | note |
|---|---|---|
| U1 | mailbox-send-and-deliver | `message send`, `session deliver` and `session deliveries`, the `[HANDOVER]` form; the docs generator leaves out `UNMAPPED` notes |
| U2 | run-daemon | `conductor run` with `--once`, `--install` and `--uninstall`, its ticks recorded; the conductor session's prompt, tasks and profile retired |
| U3 | decide-model-adapter | `decide::classify`, the `Decider` adapter (`ClaudePrint`, `Fake`), `decide::handle_request` and `conductor decision handle` |

Not in this wave, from the epic: session-home-spike, controller-profile-for-run, maintainer-guard,
charter-generated.

## Notes

- Each unit was cut from the specification commit and had to compile against the regenerated
  model alone, so each wrote the same stubs: the new `TryContext` obligations in nine modules and
  one test, the new decision fields, the session leaves' argument structs. The merges kept one copy
  of each and the real handler over every stub: U3's `--classification-rule`, `--decider-name`
  and `--prompt-sha256` over U1's exit-2 stubs, U2's `record-run-tick` and `run-ticks` over U1's,
  U1's `session deliver` over a free function of the same name in U2.
- U3 found a gap in the specification: `conductor decision handle` had no input type. It was added
  at integration as `conductor.decision.HandleRequest`; the binding lists `decision handle` before
  `decision show`, and `cli_tree` held the derive tree to that order.
- The daemon's decider seam was wired at integration: the mailbox hands a `[DECISION-REQUEST]`
  whole to `decide::handle_request` and delivers a decided request's `[DECISION]` line to the
  controller that raised it (`decision-sent`). `CONDUCTOR_DECIDER=fake` chooses the fake decider,
  which `tests/run.rs` uses.

## Numbers

- Narrow lanes after U1 and U3 merged: mailbox 8, deliver 5, message_read 7, guard_rules 137
  (2 ignored), cli_tree 7, decide_class 4, decide_model 12, decision 28, store_tree 12
  (1 ignored), adv_cli 6, adv_cli_pass2 3, records_log 14, all passing; cli_tree failed once on the
  order of `decision handle`.
- After U2 merged: run 34 (one new case), cli_tree 7, adv_cli_pass2 3, adv_cli 6, decide_model 12,
  mailbox 8, deliver 5, all passing.
- `task check` with the Gates scan, first run: 1608 files carry no private rule, 402 of 402
  conformance scenarios pass, the dashboard's Vitest passes 188 tests in 25 files, `cargo test`
  passes 1421 tests over 131 test binaries with 0 failed and 7 ignored, clippy and fmt are clean,
  and the 5 generated pages are current. Exit 0.

## Outcome

`task check` green on the integration head with the Gates scan, first local run. Target v0.19.0.
