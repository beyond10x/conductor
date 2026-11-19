---
format: aep.planning-md/3
id: story:tests-clear-instance-env
kind: story
status: draft
title: A spawned test command clears every CONDUCTOR_ variable of the session that runs the suite
summary: common::conductor removes CONDUCTOR_CONFIG and CONDUCTOR_INSTANCE only, so a suite run inside a boxed session inherits CONDUCTOR_API_SOCKET, CONDUCTOR_API_DIAL, CONDUCTOR_API_LOG and CONDUCTOR_STORE and a sec_instance_pods test fails
revision: 3
---
## Problem

`common::conductor` (crates/conductor/tests/common/mod.rs) removes `CONDUCTOR_CONFIG` and
`CONDUCTOR_INSTANCE` from the command it builds. A suite run inside a boxed instance's session
inherits the runtime's other variables (`CONDUCTOR_API_SOCKET`, `CONDUCTOR_API_DIAL`,
`CONDUCTOR_API_LOG`, `CONDUCTOR_STORE`), and the spawned `conductor` talks to the live runtime or
store: one `sec_instance_pods` test failed on it in wave 57, and passed with them unset.
In wave 58 four `spawn.rs` tests failed `UntrustedCheckout` because they inherited the session's
`CLAUDE_CONFIG_DIR` and read the host's `.claude.json`; they passed that check with it unset.

## Outcome

- `common::conductor` removes every variable whose name starts with `CONDUCTOR_`, and
  `CLAUDE_CONFIG_DIR`, then sets what the test passes.
- `tests/isolated_home.rs` fails on a test that spawns `conductor` with a `CONDUCTOR_` variable of
  its process.
- A test that sets `CONDUCTOR_API_SOCKET` in the suite's own environment to a path that does not
  exist still passes the suites that read the store.

## Boundaries

Test harness only; no product change.
