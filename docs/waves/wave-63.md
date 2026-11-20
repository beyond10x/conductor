# Wave 63 (2026-11-20: a spawned controller gets its brief, a resume starts what it reports)

The eighth wave of the goal that fixes the friction the agent box filed. Both units and the
integration branch fork from `main` at release 1.19.0. The wave before is [wave 62](wave-62.md).

Width: two units, every ready one (`controllers.max_subagents` is 2). Both touch
`crates/conductor/src/spawn.rs`, in different functions: U1 the charter's `## Dispatch` section, U2
the resume path. Rust builds run one at a time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | spawn-charter-carries-brief | #224 | merged into the wave branch |
| U2 | resume-starts-gone-session | #226 | merged into the wave branch |

## Outcome

U1: the charter's `## Dispatch` section ends with the dispatch's brief, verbatim in a fenced
block, for every dispatch whose brief has text, not only for a unit of work: a controller started
for a goal's dispatch reads its brief there, since `dispatch show` stays closed to controllers.

U2: `resume-controller` prints `resumed` and records `Running` only once the resumed session is
live; else it exits 1 naming why and the controller stays `Paused`. Two causes reproduced the
issue: a resume on the host of a boxed instance (now refused `OutsideBox`, as `spawn` is), and the
`child` provider reading an `init` line of an earlier run in the session's output as a new start.
The `none` provider's resume now waits for the session list to show the session.

Drafted `pause-reaches-boxed-session`: on the host, `pause-controller` for a session in the box
still records `Paused` without reaching it.

In the box: the specification, a fresh generation, `cargo fmt --check`, workspace clippy, the docs
check and the touched suites; the suites that launch a session or bind a socket are proven by CI.
