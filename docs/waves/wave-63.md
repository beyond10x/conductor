# Wave 63 (2026-11-20: a spawned controller gets its brief, a resume starts what it reports)

The eighth wave of the goal that fixes the friction the agent box filed. Both units and the
integration branch fork from `main` at release 1.19.0. The wave before is [wave 62](wave-62.md).

Width: two units, every ready one (`controllers.max_subagents` is 2). Both touch
`crates/conductor/src/spawn.rs`, in different functions: U1 the charter's `## Dispatch` section, U2
the resume path. Rust builds run one at a time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | spawn-charter-carries-brief | #224 | building |
| U2 | resume-starts-gone-session | #226 | building |

## Outcome

Written when the wave merges.
