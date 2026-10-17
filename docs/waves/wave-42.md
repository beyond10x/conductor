# Wave 42 (the loop as a program)

Epic: conductor-run, waves 1 and 2. Integration branch `takeover/run-daemon`, one branch per
unit. Target: v0.19.0. Sources: [`../design/conductor-run.md`](../design/conductor-run.md) and the
measured before,
[`../analysis/takeover-baseline-2026-10-17.md`](../analysis/takeover-baseline-2026-10-17.md).

## Units

| unit | story | note |
|---|---|---|
| U1 | mailbox-send-and-deliver | wave 1: `message send`, `session deliver` and `session deliveries`, the `[HANDOVER]` form; the docs generator leaves out `UNMAPPED` notes |
| U2 | run-daemon | wave 1: `conductor run` with `--once`, `--install` and `--uninstall`, its ticks recorded; the conductor session's prompt, tasks and profile retired |
| U3 | decide-model-adapter | wave 1: `decide::classify`, the `Decider` adapter (`ClaudePrint`, `Fake`), `decide::handle_request` and `conductor decision handle` |
| U4 | controller-profile-for-run | wave 2: the controller reports with `conductor message send`; the controller profile from 183 to 120 lines, the conductor-dev profile from 115 to 60; the docs start the loop with `conductor run --install` |
| U5 | release 0.19.0 | wave 2: the version in `Cargo.toml` and `Cargo.lock`, the install lines of the README and the guides |
| U6 | records-committed-by-daemon | wave 2: the daily tick, and `run --once --commit-records`, commit and push the records through the bot route; `records-committed` and `records-commit-refused` lines |
| U7 | classify-tightened | wave 2: each classification rule matches an action on an object and names the phrase; 30 fixture requests |

Not in this wave, from the epic: session-home-spike, maintainer-guard, charter-generated.

## Notes

- Each wave-1 unit was cut from the specification commit and had to compile against the
  regenerated model alone, so each wrote the same stubs: the new `TryContext` obligations in nine
  modules and one test, the new decision fields, the session leaves' argument structs. The merges
  kept one copy of each and the real handler over every stub: U3's `--classification-rule`,
  `--decider-name` and `--prompt-sha256` over U1's exit-2 stubs, U2's `record-run-tick` and
  `run-ticks` over U1's, U1's `session deliver` over a free function of the same name in U2.
- U3 found a gap in the specification: `conductor decision handle` had no input type. It was added
  at integration as `conductor.decision.HandleRequest`; the binding lists `decision handle` before
  `decision show`, and `cli_tree` held the derive tree to that order.
- The daemon's decider seam was wired at integration: the mailbox hands a `[DECISION-REQUEST]`
  whole to `decide::handle_request` and delivers a decided request's `[DECISION]` line to the
  controller that raised it (`decision-sent`). `CONDUCTOR_DECIDER=fake` chooses the fake decider,
  which `tests/run.rs` uses.
- The wave-2 branches merged without a conflict. The guard already held the rules U4's profile
  relies on: `cli::LEAVES` declares `message send` a mailbox write a controller runs and `session
  deliver` a write only conductor runs, and `tests/guard_rules.rs` asserts both. U4's profile no
  longer names a `SendMessage` recipient, which an adversary case of `guard_rules` read from it;
  that case now reads the `conductor message send` command the profile names and asserts the guard
  allows it.

## Numbers

- Wave 1, narrow lanes after U1 and U3 merged: mailbox 8, deliver 5, message_read 7, guard_rules
  137 (2 ignored), cli_tree 7, decide_class 4, decide_model 12, decision 28, store_tree 12
  (1 ignored), adv_cli 6, adv_cli_pass2 3, records_log 14, all passing; cli_tree failed once on the
  order of `decision handle`.
- Wave 1, after U2 merged: run 34 (one new case), cli_tree 7, adv_cli_pass2 3, adv_cli 6,
  decide_model 12, mailbox 8, deliver 5, all passing.
- Wave 1, `task check` with the Gates scan, first run: 1608 files carry no private rule, 402 of 402
  conformance scenarios pass, the dashboard's Vitest passes 188 tests in 25 files, `cargo test`
  passes 1421 tests over 131 test binaries with 0 failed and 7 ignored, clippy and fmt are clean,
  and the 5 generated pages are current. Exit 0.
- Wave 2, narrow lanes after U4 to U7 merged: profiles 24, run_records 10, decide_class 5 (the 30
  fixture requests and 7 more), decide_model 12, run 34, cli_tree 7, adv_cli 6, adv_cli_pass2 3,
  store_tree 12 (1 ignored), all passing; guard_rules failed 1 of 137 on the profile's recipient,
  then passed 137 (2 ignored).
- Wave 2, `task check` with the Gates scan, first run: 1613 files carry no private rule, 402 of
  402 conformance scenarios pass, the dashboard's Vitest passes 188 tests in 25 files, `cargo
  test` passes 1434 tests over 132 test binaries with 0 failed and 7 ignored, clippy and fmt are
  clean, and the 5 generated pages are current. Exit 0.

## Outcome

`task check` green on the integration head with the Gates scan, first local run of each wave.
Version 0.19.0.
