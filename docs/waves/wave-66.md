# Wave 66 (2026-11-20: a boxed runtime starts its director in the box)

The eleventh wave of the goal that fixes the friction the agent box filed: #220, the option the
operator chose, which starts each boxed instance's director in the box from the profile the binary
carries and keeps it running, and has doctor and the dashboard name a missing one. It forks from
wave 65's head, which shares the config specification, the generated model, doctor and the box
session tests with it. The wave before is [wave 65](wave-65.md). The critic panel's result:
`review-result:w66-critic-panel`.

Width: one unit. `box-registry-egress` (#198) is held upstream until substrate names the release
that lifts its aperture bound; `dashboard-unit-merge-setting` waits for a build with access to a
package registry. Rust builds run one at a time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | box-runtime-starts-director | #220 | merged (dashboard part parked) |

## Outcome

U1: the binary carries a generic director profile (`conductor role show --role director`); every
director start appends it and passes no `--agent`, unless the role names its own `profile` or
`agent`, so a hand start on an unboxed instance also starts from it. In a runtime that runs
in its instance's box, a keeper (`run/director.rs`) runs on each watch tick after the pending
handlers: it starts the director when none is live, at once the first two times and then after
5, 10, 20, 40 and at most 60 minutes, starting over after a session that lived an hour; it
waits for a usage limit's reset, counts a live adopted session as running, does not restart
a director stopped by `session stop --role director`, and raises `director-not-started` once
after three failed starts. `session start` holds a start lock from its check to the launch, so
a hand start and the keeper cannot both start one (`SessionRunning`). Doctor reads the keeper's
`director.json` and says when the runtime starts a missing director and why; its line is part
of the health view, so the dashboard names it, and the Sessions data carries the director's state.

The unit's dashboard commit (the Sessions row naming why no director runs) changes the app's
source, but the box resolves no package registry, so neither Vitest nor the `dist/` rebuild ran;
it stays on the unit branch with `dashboard-unit-merge-setting` until a build with registry access.

In the box: the specification, the HTTP binding, a fresh generation, `cargo fmt --check`, workspace
clippy, the docs check, the library tests and the box_session, config, confine, daemon_pods,
doctor, isolated_home, no_organization, profiles and roles suites; the suites that bind a Unix
socket (dashboard_sessions, three runtime_loop cases, placement's socket cases) are proven by CI.
