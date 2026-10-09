---
format: aep.planning-md/3
id: upstream-blocker:harness-gate-staging
kind: upstream-blocker
status: cleared
title: The harness substrate driver cannot stage the programs and cargo config conductor's gate needs
relations:
- blocks: story:substrate-gate-provider
- blocks: story:substrate-tool-server
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-09T02:13:13Z", actor: "human:timo", revision: 3}
---
## What would clear it

Both, each observed in docs/analysis/substrate-tool-spike.md:
- `--driver` stages several host programs (conductor's suite needs `ess`, `aep` and `claude`; 64 of
  559 tests failed without them);
- the rust toolchain reads the machine's `~/.cargo/config.toml` read-only (with `CARGO_HOME` inside
  the workspace, `target/` grew from 2.4G to 6.8G).

Owner: the harness repository; asked 2026-10-09.

## Cleared

2026-10-09, by the operator's choice B on the escalation of this blocker (options: A a harness wave,
B conductor stages its programs itself through substrate, C staging in loom first): no harness
change. `story:substrate-client` carries both items. Harness itself did not change.
