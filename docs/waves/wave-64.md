# Wave 64 (2026-11-20: a NEED to its own repository stays, a pause reaches the box, a select for unit_merge)

The ninth wave of the goal that fixes the friction the agent box filed, and of #207 (the loop
carries the operator's routine duties). The units and the integration branch fork from `main` at
release 1.19.1. The wave before is [wave 63](wave-63.md). The critic panel's result:
`review-result:w64-critic-panel`.

Width: two units at a time (`controllers.max_subagents` is 2); U3 starts when U1 or U2 finishes.
The three file sets are disjoint. Rust builds run one at a time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | need-to-own-repo-not-echoed | #207 | merged into the wave branch |
| U2 | pause-reaches-boxed-session | #207 | merged into the wave branch |
| U3 | dashboard-unit-merge-setting | #207 | left out: no npm registry in the box (#198) |

## Outcome

U1: a `[NEED]` whose `<other-repo>` is its own `<repo>` is neither delivered to nor kept for that
repository: it stays in conductor's mailbox for the supervisor, as a need to `conductor` does, the
message stays `Received`, and no `need-unroutable` alert is raised. A running controller of the
repository is answered once that it stays there; one that does not run is told nothing, since
telling it would keep a message for it.

U2: on the host, `pause-controller` for a session in the box goes to the box's runtime through
conductord (`POST controllers/{id}/pause`, a write under the operator's settings grant, as stop is):
the runtime stops the session and records `Paused`; a runtime not reached leaves the record
`Running` and exits 1.

U3 is committed on its own branch (`CHOICES` gains `controllers.unit_merge`, with its Vitest case),
but the box resolves no package registry, so neither `npm ci`, Vitest nor the `dist/` rebuild ran;
it is left out of this wave rather than merged with a stale `dist/`, and waits for a build with
registry access.

In the box: the specification, a fresh generation, `cargo fmt --check`, workspace clippy, the docs
check and the touched suites; the suites that launch a session or bind a socket are proven by CI.
