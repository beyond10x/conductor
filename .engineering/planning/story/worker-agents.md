---
format: aep.planning-md/3
id: story:worker-agents
kind: story
status: draft
title: Every lead starts with planner, implementer, reviewer and releaser agents, each with its profile, default tools and model tier
relations:
- decomposes: epic:role-names
- depends_on: story:role-names-in-files
scope:
- confidence: cited
  path: .agents/implementer.md
- confidence: cited
  path: .agents/lead.md
- confidence: cited
  path: .agents/planner.md
- confidence: cited
  path: .agents/releaser.md
- confidence: cited
  path: .agents/reviewer.md
- confidence: cited
  path: .claude/agents
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/spawn.rs
- confidence: cited
  path: crates/conductor/tests/worker_agents.rs
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: spec/domains/session.yaml
revision: 3
---
# Story: worker-agents

## Outcome

Every lead session starts with four worker agents available, planner, implementer, reviewer and
releaser, each with its own profile, default tools and model tier, and the lead profile starts one
worker per unit.

## Context

Design: `docs/design/roles-and-surfaces.md` § 3 (roles available by default) and § 2 (the lead
codes nothing); `docs/design/conductor.md` § 2 (a worker is a sub-agent of a controller) and § 11
(profiles and adapters). Today a controller's workers are generic sub-agents with the full tool
set. A lead starts in its repository's checkout with `--setting-sources project,local`, so it sees
no agent of this repository's `.claude/agents/`: the worker agents must reach it from
`conductor spawn`.

The design's tool names (§ 6: read, search, edit, test, commit, plan, release, pr, ci, note, ask)
belong to the `tool` surface of epic `agent-tools`. Until that ships, each worker's default tools
are the harness's tools that cover the same ground:

| worker | tools (Claude Code) | model |
|---|---|---|
| planner | Read, Grep, Glob, Edit, Write, Bash | opus |
| implementer | Read, Grep, Glob, Edit, Write, Bash | sonnet; the lead names opus for a large unit |
| reviewer | Read, Grep, Glob, Edit, Write, Bash (its profile: writes test files only) | opus |
| releaser | Read, Grep, Glob, Bash | sonnet |

## Domain relations

- Lead → Worker, one-to-many (zero allowed), the lead starts and ends each worker, a worker lives
  within one unit of its lead — inferable from `docs/design/roles-and-surfaces.md` §§ 2–3
  (operator-approved) and `docs/design/conductor.md` § 2; the bound on how many is
  `conductor.config.Controllers.max_subagents` in `spec/domains/config.yaml` (a field, not a
  `relations:` entry). No `ess/1` document declares the relation.

## Acceptance

1. Spec first: `spec/domains/session.yaml` gives `conductor.session.SessionRequest` the agents a
   session starts with (name, profile path, tools, model), set for a `lead` session only;
   `ess specify validate --path spec` and `task regen` pass.
2. `.agents/planner.md`, `implementer.md`, `reviewer.md`, `releaser.md` hold each worker's job
   from design § 3; `.claude/agents/<worker>.md` carries `tools:` and `model:` frontmatter as in the
   table above and a pointer at its profile; `task profiles` installs the four. Test:
   `crates/conductor/tests/worker_agents.rs` reads each adapter's frontmatter and checks it against
   the table.
3. `conductor spawn` of a lead passes the four agents to the harness (`--agents` with each one's
   description, prompt from its installed profile, tools and model). Test:
   `crates/conductor/tests/spawn.rs` with a fake harness that records its arguments.
4. The lead profile (`.agents/lead.md`) says the lead codes nothing, starts one worker per unit by
   its agent name, and names opus for an implementer of a large unit. Test:
   `crates/conductor/tests/profiles.rs` holds those sentences.

## Scope

`spec/domains/session.yaml`, `crates/conductor-model` (generated), `crates/conductor/src/session.rs`,
`crates/conductor/src/spawn.rs`, `.agents/planner.md`,
`.agents/implementer.md`, `.agents/reviewer.md`, `.agents/releaser.md`, `.claude/agents/` (four
adapters), `.agents/lead.md`, `Taskfile.yml` (`profiles`), `crates/conductor/tests/worker_agents.rs`,
`crates/conductor/tests/spawn.rs`, `crates/conductor/tests/profiles.rs`, `docs/design/conductor.md`
§ 11.

## Out of Scope

- The `tool` surface, the broker, `agent run` and grants compiled by the lead (epic `agent-tools`,
  story `agent-run-grants`); this story's tool lists are replaced there.
- Worker roles as config entries with per-instance model overrides.
- Bash denied to workers (epic `agent-tools`).
