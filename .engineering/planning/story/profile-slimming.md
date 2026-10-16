---
format: aep.planning-md/3
id: story:profile-slimming
kind: story
status: draft
title: Each profile keeps only its own level's job, and the director profile is under 250 lines, measured
relations:
- decomposes: epic:role-names
- depends_on: story:role-names-in-files
- depends_on: story:worker-agents
- informed_by: story:controller-stop-waits-for-build
scope:
- confidence: cited
  path: .agents/director.md
- confidence: cited
  path: .agents/implementer.md
- confidence: cited
  path: .agents/lead.md
- confidence: cited
  path: .agents/maintainer.md
- confidence: cited
  path: .agents/planner.md
- confidence: cited
  path: .agents/releaser.md
- confidence: cited
  path: .agents/reviewer.md
- confidence: cited
  path: crates/conductor/tests/profile_size.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: docs/design/conductor.md
revision: 3
---
# Story: profile-slimming

## Outcome

Each role's profile keeps only its own level's job, and the director profile is under 250 lines
(from 393), measured by a test.

## Context

Design: `docs/design/roles-and-surfaces.md` § 1 point 3 (the conductor profile is 393 lines, the
controller profile 170, and `rules.md` holds corrections of roles doing the wrong thing) and §§ 2–3
(one job per level); `docs/design/conductor.md` § 11 (profiles stay generic) and § 12 (what a
start reads).

Measured baseline, 2026-10-16: a conductor session reached 200k tokens of context in about
35 minutes. In that session there were 120 Bash calls and 228 KB of tool results. The rest was
start reading, the 393-line profile and an instance `rules.md` of more than 800 lines, plus
messages. A shorter profile is one part of the start reading; the tool results are epic
`agent-tools`.

What leaves the director profile: what is a lead's job (repository planning, workers, reviews,
releases) goes to `.agents/lead.md`; what is a worker's goes to the worker profiles of story
`worker-agents`; reference material the director only needs on demand (message formats, decision
classes) is replaced by a pointer to `docs/design/conductor.md` §§ 5–6, which § 12 already lists
in the start reading. The lead profile sheds its workers' jobs the same way.

The lead profile's hand-over step follows story `controller-stop-waits-for-build` (hand over after
running builds exit, or name them); this story keeps that step as that story writes it and does not
restate it.

## Acceptance

1. `crates/conductor/tests/profile_size.rs` counts lines: `.agents/director.md` under 250,
   `.agents/lead.md` at most 170 and `.agents/maintainer.md` at most 112 (today's counts, so they
   do not grow); the test prints each count.
2. Each rule `crates/conductor/tests/profiles.rs` pins stays in the profile of the level whose job
   it is: a pinned sentence that moves to another profile moves its test with it, and none is
   dropped.
3. The director profile names no repository-level or unit-level task; a test in
   `crates/conductor/tests/profiles.rs` checks it holds no instruction to plan a repository's
   waves, start workers or review a change.
4. Before and after, the director's start reading is measured in lines (profile plus
   `conductor rules show --section director` on a fixture `rules.md`) and the two numbers are
   recorded on the story as evidence.

## Scope

`.agents/director.md`, `.agents/lead.md`, `.agents/maintainer.md`, the worker profiles under
`.agents/`, `crates/conductor/tests/profile_size.rs`, `crates/conductor/tests/profiles.rs`,
`docs/design/conductor.md` § 11.

## Out of Scope

- Shortening an instance's `rules.md`: that file is the organization's, not this repository's.
- The controller's hand-over around running builds (story `controller-stop-waits-for-build`).
- Tool-result size and Bash round trips (epic `agent-tools`).
