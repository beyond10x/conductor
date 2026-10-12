---
format: aep.planning-md/3
id: story:agent-needs-user-setting-source
kind: story
status: implemented
title: A role started with project and local setting sources takes its profile, never a user-level agent
scope:
- confidence: cited
  path: .agents/conductor.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/tests/config.rs
- confidence: cited
  path: crates/conductor/tests/taskfile.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/domains/config.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T12:33:13Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-12T12:33:14Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-12T12:33:19Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Every session of the instance starts with `--setting-sources project,local`, so that it runs
without the operator's user configuration. With those sources Claude Code 2.1.296 loads no
user-level agent: `claude -p --agent repo-controller --setting-sources project,local …` from a
repository checkout answers "--agent 'repo-controller' not found" (reproduced 2026-10-12), and a
`--bg` start exits at once with no transcript (reported by conductor, 2026-10-12: two starts). `.agents/conductor.md:131` and `:176`, and
`docs/config.md` (`roles[].agent`), still offer `--agent <controller agent>` as the form for a role
with no `profile`; `task agents:link` links the adapters into `~/.claude/agents/`, which these
sources never read.

## Acceptance

- `conductor config validate` refuses a `controller` role that names an `agent` and no `profile`,
  naming its YAML path and saying that `--setting-sources project,local` loads no user-level agent.
  Test.
- `.agents/conductor.md` gives one controller start form, `--append-system-prompt-file <controller
  profile>`; the `--agent` alternative is gone. `docs/config.md` says the same of `roles[].agent`.
- The conductor and conductor-dev roles' `--agent` defaults in `task conductor:start` and
  `task dev:start` are checked the same way: either they are started from a cwd whose
  `.claude/agents/` holds the adapter, or they take the profile form; a test in
  `tests/taskfile.rs` holds the choice.

## Scope

`spec/domains/config.yaml` (the refusal), `crates/conductor-model/`, `crates/conductor/src/config.rs`,
`crates/conductor/tests/config.rs`, `.agents/conductor.md`, `docs/config.md`, `Taskfile.yml`,
`crates/conductor/tests/taskfile.rs`.
