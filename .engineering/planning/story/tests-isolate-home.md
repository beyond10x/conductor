---
format: aep.planning-md/3
id: story:tests-isolate-home
kind: story
status: implemented
title: Every test that runs the binary isolates HOME
scope:
- confidence: cited
  path: crates/conductor/tests
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-09T02:22:49Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-09T02:22:49Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-09T03:29:30Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Why

Wave 03 U1 found 17 test files that run the `conductor` binary without isolating HOME, so they read
the operator's live `~/.b10x/conductor/conductor.yaml`. With a live config the new rules refuse,
87 of 578 tests failed on this host; with HOME isolated, all 578 passed (U1 implementor report,
2026-10-09, logs `full-realhome.log` and `full-isolated.log` in the wave's scratch). The suite's
result depends on the machine it runs on.

Files named in that report: `adv_cli.rs`, `adv_decision_ids.rs`, `adv_decision_views.rs`,
`adv_dispatch_w06.rs`, `adv_state_dir.rs`, `adv_store_w05.rs`, `collect_stores.rs`,
`collector_bounds.rs`, `decision.rs`, `dispatch.rs`, `followup_w06.rs`, `goal_resource.rs`,
`repository_marks.rs`, `shipped.rs`, `store_tree.rs`, and others the check below finds.

## Acceptance

- Every test that runs the binary sets `HOME` to a test directory and removes `CONDUCTOR_CONFIG` and
  `CONDUCTOR_INSTANCE`, through one shared helper.
- A test fails when a test file under `crates/conductor/tests/` builds a `Command` for the binary
  without that helper (a source scan, like the existing literal checks).
- `cargo test -p conductor-cli --no-fail-fast` gives the same counts with HOME set to an empty
  directory and with HOME set to a directory holding an invalid `~/.b10x/conductor/conductor.yaml`.

## Scope

`crates/conductor/tests/` (the helper module and the files above).
