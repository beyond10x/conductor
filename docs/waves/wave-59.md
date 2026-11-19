# Wave 59 (2026-11-19: the cycle's places and reasons, rules show, boxed spawn, limits and upgrade skew)

The fourth wave of the goal that fixes the friction the agent box filed. Forked from `main` at
release 1.16.0. The wave before is [wave 58](wave-58.md).

Width: four units, coded in parallel; their Rust builds run one at a time (one lock, one build per
box, the operator's memory rule). Story `resumed-controller-done-held` (#202) shares the dispatch
and report code with U1 and waits for wave 60; the loop's routine duties (#207) are decomposed into
stories in this wave and delivered from wave 60.

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | dispatch-waiting-reason, idle-controller-yields-place, dispatch-v1-once-per-goal | #194 #203 #204 | merged into the wave branch |
| U2 | rules-show-unnumbered-items, host-role-session-unprefixed, disk-low-fits-disk, daily-tick-logged | #221 #214 #213 #212 | merged into the wave branch |
| U3 | spawn-refuses-outside-box, director-boxed-start, controller-name-not-conductor, deliver-gone-and-chartless-dispatch, supervisor-boxed-controller-start | #216 #220 #218 #215 | merged into the wave branch |
| U4 | upgrade-checks-daemon-protocols, limit-mark-expires-mailbox-kept | #223 #219 | merged into the wave branch |
| plan | epic routine-duties-in-loop: unit-merge-policy, failing-start-recreates-box, need-starts-unit-of-work, loop-alerts-when-it-cannot-act; critic panel passed in three rounds | #207 | stories active, delivered from wave 60 |

## Outcome

All four units merged into `wave/2026-11-19-conductor-59`; each spec change was merged, the
generated crate regenerated from the merged specification and the binding re-pinned. In the box:
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `conductor-docs generate
--check` pass. The suites that bind a Unix socket or launch a session (`run`, `spawn`, `broker`,
`token_pool`, `api_parity`, `daemon_probes`, `dashboard_sessions`, and four `maintainer:start`
cases of `taskfile`) cannot run in the box and are proven by CI's `task check`.

Left for later: starting a director inside the box (`story:box-runtime-starts-director`, draft:
whether a boxed instance runs a director beside `conductor run` is open), #202 (wave 60), and #197
and #198, which wait on substrate.
