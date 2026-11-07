---
format: aep.planning-md/3
id: story:tests-clear-conductor-environment
kind: story
status: draft
title: Integration tests start the binary with no inherited CONDUCTOR_ variable
revision: 2
---
## Why

`crates/conductor/tests/session_attach.rs:299` (`attach_types_into_the_session_and_detaches_with_ctrl_bracket_d`) failed in a full `cargo test` run inside a conductor-started session on 2026-11-07 (`session list` printed `ex-alpha attach conductor busy`, the test expects `ex-alpha attach sessions busy`) and passed in CI on the same tree (PR #97). The test removes only `CONDUCTOR_SESSION`, `TMUX` and `TMUX_PANE` (`session_attach.rs:35`); a session started by conductor also carries `CONDUCTOR_CONFIG`, `CONDUCTOR_INSTANCE`, `CONDUCTOR_CWD`, `CONDUCTOR_HARNESS` and `CONDUCTOR_TYPESCRIPT`, which reach the binary under test. The same leak was fixed once for `observation_retention` (`CONDUCTOR_INSTANCE`).

## Acceptance

- Every integration test that runs the conductor binary starts it with every `CONDUCTOR_*` variable removed except the ones the case sets, through one shared helper; a test runs one case with `CONDUCTOR_INSTANCE` and `CONDUCTOR_CONFIG` set to a foreign instance in the parent environment and reads the case's own instance.
- A full `cargo test` inside a conductor-started session gives the same result as in CI.
