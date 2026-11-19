# Wave 60 (2026-11-19: the loop's alerts, a resumed controller's done held)

The fifth wave of the goal that fixes the friction the agent box filed. Forked from `main` at
release 1.17.0. The wave before is [wave 59](wave-59.md).

Width: two units. The other stories of epic `routine-duties-in-loop` (#207) depend on the alert
story of U1 and run in wave 61; #197 and #198 wait on substrate. Rust builds run one at a time
(one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | loop-alerts-when-it-cannot-act | #207 | merged into the wave branch |
| U2 | resumed-controller-done-held | #202 | merged into the wave branch |

## Outcome

Both units merged into `wave/2026-11-19-conductor-60`; both add dispatch log events, kept side by
side, with the generated crate regenerated and the binding re-pinned. The controller profile stays
at its 120-line cap: U2's `withdrawn` form is one line. In the box: `cargo fmt --check`, `cargo
clippy --all-targets -- -D warnings`, `conductor-docs generate --check` and the suites that launch
no session pass; the suites that launch a session or bind a socket are proven by CI.

Next: wave 61 takes the three other stories of `routine-duties-in-loop` (unit-merge-policy,
failing-start-recreates-box, need-starts-unit-of-work), which build on U1's alerts.
