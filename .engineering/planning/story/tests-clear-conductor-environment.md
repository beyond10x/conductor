---
format: aep.planning-md/3
id: story:tests-clear-conductor-environment
kind: story
status: draft
title: Integration tests start the binary with no inherited CONDUCTOR_ variable
revision: 3
---
## Why

`crates/conductor/tests/session_attach.rs:299` (`attach_types_into_the_session_and_detaches_with_ctrl_bracket_d`) fails on the development host and passes in CI on the same tree (PR #97, CI run 42393994433). On the host, `session list` prints `ex-alpha attach conductor busy`; the test expects `ex-alpha attach sessions busy`. It failed in a full run inside a conductor-started session, and again (2026-11-07, 136 suites, 1507 passed, this 1 failed) with every `CONDUCTOR_*` variable unset and `LC_ALL=C`, so an inherited `CONDUCTOR_*` variable is not the cause. Cause not yet known; the third column's source is the place to start.

## Acceptance

- The cause is named with the line that reads the host's state, and the test gives the same result on the host as in CI; a test pins the column's value against a fixture that differs from the host's.
