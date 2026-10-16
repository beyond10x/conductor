---
format: aep.planning-md/3
id: story:agent-run-grants
kind: story
status: draft
title: A lead starts each worker with agent run and a grant it compiles, never wider than its own, and moves to tool itself
relations:
- decomposes: epic:agent-tools
- depends_on: story:implementers-on-tool
- depends_on: story:tool-forge-verbs
- depends_on: story:tool-plan-memo-wait
scope:
- confidence: cited
  path: .agents
- confidence: cited
  path: .claude/controller-settings.json
- confidence: inferred
  path: crates/conductor-broker
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: inferred
  path: crates/tool
- confidence: inferred
  path: spec/domains/tool.yaml
revision: 5
---
# Story: agent-run-grants

## Outcome

A lead starts each worker as its own session with `tool agent run --role <r> --brief <text> --tools <list>`. The broker grants the named tools minus anything outside the lead's own grant, and the worker explores what it got with `tool list` and `tool describe`. The lead itself then works through `tool` with Bash denied.

## Context

docs/design/roles-and-surfaces.md § 6.4, § 3 (default tools per role; "the lead itself codes nothing"), § 7 step 4 ("grants by `agent run`; the lead moves to `tool`"). Until this story, implementers run as sub-agents on the lead's grant (story:implementers-on-tool). Here each worker gets its own session, socket and grant.

## Domain relations

- Grant (parent) → Grant (child), one-to-many: a child's grant is the intersection of the named tools and limits with the parent's, so its time to live never exceeds the parent's remaining time. Decided in docs/design/roles-and-surfaces.md § 6.4 ("a child never gets more than its parent"), with § 6.2 making time to live a limit of the grant. acceptance 1 declares it in `spec/domains/tool.yaml`.
- Role → default tools, one-to-many: these apply when the lead names none. Decided in § 3 and § 6.4.
- Worker session → Team, many-to-one: it joins its lead's team. Decided in § 2.
- Grant (parent) → Grant (child), lifecycle: a child's grant ends with its parent's session; the broker revokes it. Decided in decision-blocker:worker-grant-after-lead-ends (cleared 2026-10-16).

## Acceptance

A lead whose grant lacks `push` runs `tool agent run --role implementer --tools show,test,commit,push`. The worker's `tool list` shows `show`, `test` and `commit` and not `push`, and the narrowing is recorded with the rule that removed `push`.

1. **Spec first.** `spec/domains/tool.yaml` declares `AgentRun` (role, brief, requested tools, model, granted tools, removed tools with the rule) and the parent → child grant relation, with the invariant that the child is a subset of the parent. Validate and regenerate.
2. **Start.** `tool agent run --role <r> --brief <text> [--tools <list>] [--model m]` is held by the lead only. It starts a worker session in the lead's repository with the role's profile, a socket and the compiled grant, and prints the worker's id. With no `--tools`, the role's default tools from § 3 apply and are intersected the same way.
3. **Narrow only.** Test `crates/conductor-broker/tests/grant_compile.rs` (property-style over generated grants): the child is a subset of the parent for tools, repositories, paths and budgets, and the child's time to live is at most the parent's remaining time. A worker that calls `agent run` itself is refused, because no worker role holds it.
4. **Explore.** `tool list` prints the caller's verbs, one per line. `tool describe <verb>` prints its arguments and the limits the grant applies. Both are in every grant.
5. **Wait on a worker.** `tool wait --agent <id> [--deadline d]` resolves when the worker reports, extending story:tool-plan-memo-wait's wait.
6. **The lead moves.** The lead's settings deny Bash and start `tool mcp`. The guard denies a lead's Bash call and records it, as it does for implementers. `conductor resource tool-calls --role lead` taken before and after is recorded as evidence.
7. **Revocation.** When a lead's session ends (stopped, handed over or lost), the broker revokes the grant of every worker it started, recursively, and records each revocation. A revoked worker's next call is refused `GrantRevoked` ("lead ended") and recorded. Test `crates/conductor-broker/tests/grant_revoke.rs`: a lead with two workers ends, and each worker's next `tool show` is refused `GrantRevoked`; a worker of another lead is unaffected. `spec/domains/tool.yaml` declares `GrantRevoked`.

## Scope

- `spec/domains/tool.yaml`, `crates/conductor-model` (regenerated)
- `crates/conductor-broker/src/` (grant compile, agent run), `crates/conductor-broker/tests/grant_compile.rs`, `crates/conductor-broker/tests/grant_revoke.rs`
- `crates/tool/src/`
- `crates/conductor/src/spawn.rs`, `crates/conductor/src/guard/rules.rs`, `.claude/controller-settings.json` (or the lead's settings after epic:role-names), `.agents/` lead profile

## Out of Scope

- Handing a grant over to a lead's successor session (a later story, if work is lost through revocation).
- Aliases (story:tool-define-aliases).
- The director's verbs (story:tool-director-verbs).

## Ambiguities

None.
