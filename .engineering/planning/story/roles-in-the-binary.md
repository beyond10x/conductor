---
format: aep.planning-md/3
id: story:roles-in-the-binary
kind: story
status: implemented
title: a role's profile ships inside the conductor binary and its agent kind writes the harness adapter at start
summary: role profiles move from .agents/ and .claude/agents/ into the binary; the claude-code kind writes its adapter at agent start; task profiles and the host copies go
relations:
- decomposes: epic:architecture-next
- depends_on: story:agent-kinds
scope:
- confidence: inferred
  path: README.md
- confidence: cited
  path: crates/conductor/roles/controller.md
- confidence: cited
  path: crates/conductor/roles/maintainer.md
- confidence: cited
  path: crates/conductor/roles/supervisor.md
- confidence: cited
  path: crates/conductor/roles/teacher.md
- confidence: cited
  path: crates/conductor/src/agent/claude_code.rs
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/role.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/tests/no_organization.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/roles.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: inferred
  path: docs/design/conductor.md
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/config.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T21:45:10Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-08T21:45:10Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-09T10:21:35Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
A role's profile is product data that ships inside the `conductor` binary, kind-neutral, and an
agent kind turns it into what its harness reads when it starts an agent. Nothing is copied to the
host to make a role run, and a profile is always the version of the binary that starts it.

## Why

The v1 design makes a role slot `(role, kind, placement, profile, limits)` and lets any kind fill
any role (docs/design/architecture-next.md § 4), and a box receives "the conductor binary and its
profiles" as read-only roots (§ 6). The profiles still live where the 0.x layout put them:

- `.agents/` holds the four role profiles, though `.agents/` is the place for instructions to agents
  that develop this repository; `conductor-dev` there is both this repository's own maintainer agent
  and the product's maintainer role.
- `.claude/agents/` holds one Claude Code adapter per profile, committed beside them: the
  `claude-code` kind's packaging, not the role's; a second kind would need a second committed set.
- `task profiles` copies the profiles to `~/.b10x/conductor/profiles/`, and `conductor init`
  installs the supervisor and teacher adapters into `~/.claude/agents/` (never overwritten). A role
  then reads a copy whose version can differ from the binary's; a supervisor ran on an older
  profile than the one committed.

## Acceptance

1. The role profiles live under `crates/conductor/roles/<role>.md`, one per role the spec declares,
   and are compiled into the binary; `conductor role show <role>` prints the text the binary
   carries (spec first).
2. The `claude-code` kind writes the role's harness adapter itself when it starts an agent (the
   appended system prompt or an agent file in the agent's own state), from the binary's text; no
   adapter is committed under `.claude/agents/` for a product role, and no file under the user's
   `~/.claude/agents/` is written for one.
3. `task profiles` and the profiles copy under `~/.b10x/conductor/profiles/` are gone; a profile
   path in the config, where the design keeps one, overrides the built-in text for that role and is
   read at start.
4. `.agents/` holds only instructions for agents that develop this repository; this repository's
   maintainer session starts from the binary's maintainer role like any other.
5. The tests that pinned the profiles (`tests/profiles.rs`, `tests/no_organization.rs`'s scan) read
   the new location; every document naming the old paths is updated (`AGENTS.md`, `README.md`,
   `docs/`, `website/`).
6. The repository has no `.claude/` directory: the settings each role starts with (`conductor-settings.json`,
   `controller-settings.json`, the guard's PreToolUse hook) ship in the binary and the `claude-code`
   kind writes them into the agent's own state at start; an agent role for working on this repository
   outside conductor lives in `.agents/`. A test fails if `.claude/` exists.

## Boundaries

- No change to what a role is told: the text moves as written, except where it names its own
  location.
- Organization rules stay in each instance's `rules.md`.
