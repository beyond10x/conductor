---
format: aep.planning-md/3
id: story:tool-build-commit-report
kind: story
status: draft
title: A session tests, checks, formats, commits, sends and reports through tool, and the broker holds the bot route and the store writes
relations:
- decomposes: epic:agent-tools
- depends_on: story:tool-broker-read
scope:
- confidence: inferred
  path: crates/conductor-broker
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/build.rs
- confidence: cited
  path: crates/conductor/src/dispatch.rs
- confidence: cited
  path: crates/conductor/src/gates.rs
- confidence: cited
  path: crates/conductor/src/message.rs
- confidence: inferred
  path: crates/tool
- confidence: inferred
  path: spec/domains/tool.yaml
revision: 4
---
# Story: tool-build-commit-report

## Outcome

A session runs `tool test`, `tool check`, `tool fmt`, `tool commit`, `tool send` and `tool report`, each a plain argv call checked against its grant, and the broker runs the build under the host ledger, commits through the bot route and writes the message and report records, so none of these needs a shell line, the Gates policy or a store write in the session.

## Context

docs/design/roles-and-surfaces.md § 6.2 and § 6.3 (build, version control `commit`, team `send` and `report`), § 7 step 3. The 48 h analysis (docs/design/agent-tools.md § 1, § 5): the `build run … cargo test > log; echo EXIT=$?` pattern followed by a grep and a poll, and the commit route boilerplate, are the largest round-trip sources after reads; 96 % of Bash calls were compound. Builds reserve the measured peak `build run` uses since v0.16.4; the broker reuses that path and adds no reservation logic of its own. Read-only store access follows story:views-open-as-reader (v0.16.3): the broker opens the store as a reader for reads and writes only through the commands that record an event.

## Domain relations

- Message → Dispatch, many-to-one optional, `conductor.dispatch.Message` relation `opened_dispatch` in `spec/domains/dispatch.yaml` — inferable (ess/1). `tool report` and `tool send` record a `Message` as `conductor message` does today.
- Session → Team, many-to-one; a team is a lead and its workers in one repository, and `send` never leaves it. Decided in docs/design/roles-and-surfaces.md § 2 ("the only sideways channel is a team's, inside one repository") and § 6.3; no `ess/1` document declares Team yet, so acceptance 1 declares it.
- Session → Level above, many-to-one: `report` goes one level up (a lead's to its director, a worker's to its lead). Decided in § 2 ("a message goes one level up or down").
- Controller → Repository, one per repository — inferable (inferred from `spec/domains/dispatch.yaml` `conductor.dispatch.Controller` field `repository`, a field and not a `relations:` entry).

## Acceptance

A session whose grant names the six verbs runs a failing test, fixes it, commits it and reports done without one Bash call, and each step prints one line.

1. **Spec first.** `spec/domains/tool.yaml` gains the verbs' inputs and errors (`OutsideTeam`, `NotOneLevelUp`, `BuildFailed { log }`, `CommitRefused { reason }`) and the `Team` entity with its relation to sessions. Validate and regenerate.
2. **Build.** `tool test [<crate>] [--filter f]`, `tool check`, `tool fmt` run in the caller's tree through the host ledger at nice 19, as `conductor build run` runs them. Success prints `ok test <n> passed`; failure prints the failing tests' names and first assertion lines, at most 40 lines, and the log path. Test `crates/conductor-broker/tests/build_verbs.rs` with a fake ledger and a fixture crate: a pass, a failure and a wait for a slot.
3. **Commit.** `tool commit -m <msg> -- <paths>` stages the named paths, new ones included, and commits through the bot route with the instance's Gates policy, which only the broker holds. It prints `<sha> <n> files`. A path outside the grant's paths or outside the caller's tree is refused `NotGranted` before anything is staged. Test `crates/conductor-broker/tests/commit.rs` against a fixture repository and a fake bot route.
4. **Send.** `tool send <role-or-name> "<text>"` delivers to a session of the caller's team and records a `Message`. A recipient outside the team or in another instance is refused (`OutsideTeam`, `ForeignInstance`) and recorded. Test `crates/conductor-broker/tests/team.rs`.
5. **Report.** `tool report <status> "<line>"` records the report to the level above, with the time from the clock, and moves the caller's open dispatch as `dispatch report` does. A report to any other level is refused `NotOneLevelUp`. Same test file.
6. All six verbs appear in `tool mcp`'s `tools/list` when granted; `crates/tool/tests/mcp.rs` gains a call of each.

## Scope

- `spec/domains/tool.yaml`, `crates/conductor-model` (regenerated)
- `crates/conductor-broker/src/` (build, commit, team modules), `crates/conductor-broker/tests/`
- `crates/tool/src/`, `crates/tool/tests/mcp.rs`
- `crates/conductor/src/build.rs`, `crates/conductor/src/message.rs`, `crates/conductor/src/dispatch.rs`, `crates/conductor/src/gates.rs` (called from the broker, not changed in behaviour)

## Out of Scope

- `branch`, `merge`, `push` and every forge verb (story:tool-forge-verbs).
- Denying Bash (story:implementers-on-tool).
- Changing how `build run` reserves disk or picks a slot.

## Ambiguities

None.
