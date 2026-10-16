---
format: aep.planning-md/3
id: story:tool-define-aliases
kind: story
status: draft
title: tool define makes an alias of verbs the caller holds, scoped, expiring, checked per step against the caller's grant, and its use recorded
relations:
- decomposes: epic:agent-tools
- depends_on: story:agent-run-grants
scope:
- confidence: inferred
  path: crates/conductor-broker
- confidence: cited
  path: crates/conductor-model
- confidence: inferred
  path: crates/tool
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/improvement.yaml
- confidence: inferred
  path: spec/domains/tool.yaml
revision: 4
---
# Story: tool-define-aliases

## Outcome

A session defines an alias with `tool define <name> '<steps>'`: a named sequence of verbs with parameters. Calling it runs each step through the broker against the **caller's** grant, at call time. An alias belongs to its session unless the lead publishes it to the team or the director to the instance. It expires when unused, and every use is recorded, so an alias used often reaches the maintainer as a finding for a built-in verb.

## Context

docs/design/roles-and-surfaces.md § 6.5 and § 7 step 5. The 48 h analysis (docs/design/agent-tools.md § 4) estimated up to about 1,800 avoidable round trips a day from sequences, and measured that a quoted shell step slips past the guard. So steps are verbs only, never a shell string.

## Domain relations

- Alias → Owner scope (session, team or instance), many-to-one. An alias belongs to its session by default; the lead may publish it to the team and the director to the instance; an unused alias expires. Decided in docs/design/roles-and-surfaces.md § 6.5. acceptance 1 declares it in `spec/domains/tool.yaml`.
- Alias → Author: none at call time. An alias never carries its author's rights. Decided in § 6.5.
- AliasUse → Finding, many-to-one: a frequently used alias becomes a finding the maintainer files as a draft story. The finding side is `spec/domains/improvement.yaml` (ess/1, the finding entity story:findings-to-stories uses); the threshold is a config key, not a relation.

## Acceptance

An alias defined by one worker as `test` then `commit` is refused at its `commit` step when a teammate whose grant lacks `commit` calls it. The same alias passes for its author, and both calls are recorded as uses.

1. **Spec first.** `spec/domains/tool.yaml` declares `Alias` (name, parameters, steps, scope, expires_at, author) and `AliasUse`. Steps are a typed tree of `seq`, `par` and verb calls with no string step. The config gains `tools.alias_idle_expiry` and `tools.alias_finding_uses`. Validate and regenerate.
2. **Define.** `tool define <name> '<steps>'` parses `{seq: [...]}` or `{par: [...]}` with `$1…$n` parameters. A step naming a verb the author does not hold, or any string step, is refused at definition. `tool list` and `tool describe <alias>` show aliases beside verbs.
3. **Call.** `tool <alias> <args>` expands in the broker. `seq` stops at the first failure, and `par` runs every step and reports every result. Each step is checked against the caller's grant at call time, and a refused step names the rule.
4. **Scope.** An alias is visible only to its session by default. `tool define --publish team` is held by the lead and `--publish instance` by the director. An alias unused for `tools.alias_idle_expiry` is removed.
5. **Use.** Every call records an `AliasUse`. An alias whose uses pass `tools.alias_finding_uses` is recorded as a finding naming the alias's steps, never its arguments. The maintainer reads findings from every instance and files them through story:findings-to-stories.
6. Tests: `crates/conductor-broker/tests/aliases.rs` covers the caller-grant refusal above, `seq` stopping, `par` reporting all, the scope and publish rights, expiry under a clock fake, and the finding at the threshold.

## Scope

- `spec/domains/tool.yaml`, `spec/domains/config.yaml`, `spec/domains/improvement.yaml`, `crates/conductor-model` (regenerated)
- `crates/conductor-broker/src/` (alias module), `crates/conductor-broker/tests/aliases.rs`
- `crates/tool/src/`, `docs/config.md`

## Out of Scope

- Turning a finding into a built-in verb; that is the maintainer's ordinary story work.
- Aliases shared across instances.

## Ambiguities

None.
