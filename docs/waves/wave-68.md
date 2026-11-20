# Wave 68 (2026-11-20: a failed collector keeps the snapshot's other rows; a dial test's race)

The thirteenth wave of the goal that fixes the friction the agent box filed: the second half of
#241, and a test that failed once on CI on the same code it passed on. It forks from `main` after
wave 67. The wave before is [wave 67](wave-67.md). The critic panel's result:
`review-result:w68-critic-panel`.

Width: two units, disjoint in scope (the snapshot, its collector runs and their readers; the token
pool's probe and its test). Rust builds run one at a time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | collector-failure-stays-its-own | #241 | merged |
| U2 | instance-api-refusal-said-once-flake | - | merged |

## Outcome

U1: a collector that fails no longer fails the snapshot. The snapshot ends `Complete`;
the failed run is recorded with its error as its `reason`, and the snapshot names its
`failed_collectors`, which `status`, `snapshot snapshots` and the dashboard's health data show;
`snapshot start-snapshot` names each failed collector on standard error and exits 0. The
dispatch cycle's count of open reviews and the github collector's reuse base read the newest
snapshot whose github and gitlab runs both completed, and retention keeps that snapshot too,
so a failed forge read is not taken for no open reviews. Spec first: an optional `reason`
on a collector run and `failed_collectors` on a complete snapshot, with a scenario.

U2: a refused call to the host's token pool reads the same whether conductord's close reached
the runtime as a reset or as an end of stream, so a dialing runtime says its refusal once;
the test that failed once on CI counts each line by text only that line holds.

In the box: the specification, the HTTP binding, a fresh generation, `cargo fmt --check`,
workspace clippy, the docs, the library tests and the 21 test files the wave changed, but for
the suites that start sessions or bind a socket (run, dashboard, dashboard_overview, instance_api
and token_pool in part), which CI proves.
