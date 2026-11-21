---
format: aep.planning-md/3
id: story:guard-denial-names-its-config
kind: story
status: active
title: A denied forge write names the instance's route, its policy and the fix
summary: a gates route with no policy was read as a credential rule for two hours
scope:
- confidence: cited
  path: crates/conductor/src/guard/rules.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: crates/conductor/tests/guard_rules.rs
- confidence: cited
  path: website/docs/security/the-guard.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:21:16Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:21:17Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-11-07, relayed by the host supervisor: "make sure the issues are fixed and that
they are persistent in the sense that it's part of conductor and not of some outside rules".
Observed that day on an instance whose repositories are on a self-hosted GitLab, with
`forge.route: gates` and no `gates_policy`: controllers reported "glab writes denied" for about two
hours. The supervisor read the denial as an operator credential rule until the route was switched
to `direct`. The denial named neither the route nor the missing policy.

The denial is also wrong for that case, not only terse: it ends "writes go through the bot route"
for a `glab` write, and the bot route writes to GitHub only (`doctor.rs` `BOT_ROUTE_HOSTS`,
`website/docs/guides/troubleshoot.md:34`). For a GitLab repository on `gates` no policy helps; the
only fix is `forge.route: direct`.

## Where the guard gets the route without a store

The hook already holds the whole instance. `guard::hook` (`crates/conductor/src/guard.rs:474`)
builds its places from `crate::config::active()` (`places_of`, `guard.rs:249`), the config file
the process resolves (`config_file`, `guard.rs:613`: `CONDUCTOR_CONFIG`, else the default file),
parsed once per call before any verdict; no store is opened on that path (the store opens only to
record a denial, `record_within`, `guard.rs:626`). `Places::of`
(`crates/conductor/src/guard/rules.rs:214`) already copies `instance.forge.route` into
`Places.forge_route` (`rules.rs:183`, `:277`); `instance.gates_policy` is on the same `Instance`
(`spec/domains/config.yaml:1100`). So the change is one more field on `Places`, read in
`Places::of`, and the instance's name (`Places.instance`) and config file (`Places.config`, set by
`with_config_file`, `rules.rs:407`) are already there. No new read, no stat of the policy file:
whether the policy is readable stays doctor's check (`route_lines_of`, `doctor.rs:2235`). The
`conductor.lease` copy is not involved: it decides only conductor-role writes the table allows.

Which config the hook reads depends on `CONDUCTOR_CONFIG` reaching it, which
story:spawn-pins-binary-and-config fixes for spawned sessions. Naming the config file in the
denial makes a hook that read the wrong file visible on the denial line itself.

## Design

The denial is the fall-through `return` after the direct-route branch in `Session::bash`
(`rules.rs:1589-1597`). Its first clause stays as it is (`` `<write>` is a <Forge> write: <program>
here only reads (<reads>) ``); what follows depends on the forge and the instance:

- `gh`, `gates`, with a `gates_policy`: "instance `<name>` (`<config file>`) is on
  `forge.route: gates` with a gates_policy: writes go through the bot route (`b10x-gates gh`,
  `b10x-gates api`), or set `forge.route: direct` there".
- `gh`, `gates`, no `gates_policy`: as above but "with no gates_policy, so the bot route has no
  policy to write under: set `gates_policy`, or `forge.route: direct`". The words "writes go
  through the bot route" stay in both `gh` cases (`tests/guard_hook.rs:780-786` asserts them).
- `glab`, `gates`, either policy: "instance `<name>` (`<config file>`) is on `forge.route: gates`
  (gates_policy set|not set), and the bot route writes to GitHub only: set `forge.route: direct`
  there". The words "go through the bot route" do not appear.
- No config file (`Places.records` is `None`, the built-in instance): "no config file: the
  built-in instance is on `forge.route: gates` with no gates_policy" and the fix "write a config
  file whose instance sets `forge.route: direct`" (or a `gates_policy`, for `gh`).

The instance is named by its name, not by `instances[<i>]`: `Places` does not hold the index, and
`Active` does not carry it. The text stays one line (no `\n`), and the reason doctor prints for
its guard line (`doctor.rs:2484` `guard_lines_of`, via `guard::asked`) is the same text, as
`tests/doctor.rs:1563` already requires of doctor and hook.

Unchanged: the credential denial (class H), the anonymous denial, `bot_route_denial`, and the
direct route's denials, which already name `forge.route direct` (`rules.rs:1975`; held by
`tests/guard_rules.rs:6467-6539`, `:6604`, `:6809`, `:6836`). That part of the original
acceptance is already done on main.

No spec change: no noun, field, error or outcome is added; the reason is a string of the existing
`GuardDecision` (AGENTS.md rule 1).

## Acceptance

- `tests/guard_rules.rs`, through `forge_layout` (`:6372`) extended with an optional
  `gates_policy` line, each failing on main:
  - `gates`, no policy, `gh pr merge 3 -R example-org/widget` in a GitHub checkout: denied, the
    reason names the instance's name, its config file path, `forge.route: gates`, "no
    gates_policy", and both fixes (`gates_policy`, `forge.route: direct`).
  - `gates`, with a policy, the same command: denied, the reason says the policy is set and names
    the bot route and `forge.route: direct`; it does not say "no gates_policy".
  - `gates`, no policy and with a policy, `glab mr create -R example-group/widget --fill` in a
    GitLab checkout: denied, the reason names the route, the policy state, "GitHub only" and
    `forge.route: direct`, and does not contain "go through the bot route".
  - Every one of those reasons holds no line break.
- `tests/guard_rules.rs`: with no config file (`guard::decide`, the built-in instance), `gh pr
  merge 3` is denied naming "no config file" and `forge.route: direct`.
- `tests/doctor.rs:1427-1428`, which asserts the GitLab case ends "writes go through the bot
  route", is changed to the new text (names `forge.route: direct`, not the bot route); the rest of
  `tests/doctor.rs` (`:1563`, `:1628`, `:1867`, `:1971`) and `tests/adv_w51_doctor_guard.rs:346`
  stay green unchanged.
- Stay green unchanged: `tests/guard_hook.rs` (`:766`, `:780-786`), the direct-route cases of
  `tests/guard_rules.rs`, `tests/forge_access_anonymous*.rs`.
- Latency: the allowed path is untouched (the new field is read only on the denial branch);
  `tests/guard_latency.rs` (ignored by default) is not required to run.
- `website/docs/security/the-guard.md`, Forge calls (`:127-170`): one paragraph saying what a
  forge-write denial names (instance, config file, route, policy state, fix), and that on `gates`
  a `glab` write's fix is `forge.route: direct`.

None of these tests needs a Unix socket; all run locally.

## Scope

- `crates/conductor/src/guard/rules.rs`: `Places` (`:134-187`, a `gates_policy: bool` field),
  `Places::of` (`:214-280`), the fall-through denial in `Session::bash` (`:1530`, the return at `:1589-1597`), the
  module table at `:30` if its forge cell changes.
- `crates/conductor/tests/guard_rules.rs`: `forge_layout` (`:6372`) and new cases in the
  story:forge-route section (`:6364`).
- `crates/conductor/tests/doctor.rs`: the assertion at `:1427-1428` only.
- `website/docs/security/the-guard.md`: the Forge calls section.
- Not `crates/conductor/src/config.rs`: `gates_policy` is already on `Instance`, and the guard
  reads it there. That keeps this story off story:spawn-pins-binary-and-config's file.
