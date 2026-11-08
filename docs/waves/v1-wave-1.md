# v1 wave 1: the specification

Wave 1 of 6 of the v1 plan ([`v1-plan.md`](v1-plan.md)). One story, `story:spec-v1`: the whole v1
system modelled in `spec/` before a handler is written. Every other story of the epic depends on
it, so the wave is one story; the story is split into nine authoring units, one per domain or
domain group, each on its own branch from the scaffold commit, and one integration unit that
merges them, declares the components, regenerates the model and makes the crate build.

Integration branch `wave/v1-1`, cut from the design's pull-request head (7caa341c7) before `main`
merged it; `v1` is cut from `main` at the merge (04973b59c), so `wave/v1-1` merges into `v1` as a
normal merge. The scaffold commit a88e52183 (the seven domain stubs, `conductor.operator.Actor`,
the system and inputs lists) is the base of every authoring unit.

## Units

| unit | files | branch | worktree | scratch | stage |
|---|---|---|---|---|---|
| A1 instance | `spec/domains/instance.yaml`, its scenarios | `spec/instance` | `wt-43aaf7b3c5ee` | `wave-v1-1/instance/scratch` | merged |
| A2 placement | `spec/domains/placement.yaml`, its scenarios | `spec/placement` | `wt-21ada001a3e5` | `wave-v1-1/placement/scratch` | merged |
| A3 agent | `spec/domains/agent.yaml`, its scenarios | `spec/agent` | `wt-49475d3bbeac` | `wave-v1-1/agent/scratch` | merged |
| A4 store | `spec/domains/store.yaml`, its scenarios | `spec/store` | `wt-57c58018ac58` | `wave-v1-1/store/scratch` | merged |
| A5 secret | `spec/domains/secret.yaml`, its scenarios | `spec/secret` | `wt-6f7ec5cdc0d4` | `wave-v1-1/secret/scratch` | merged |
| A6 operator | `spec/domains/operator.yaml`, its scenarios | `spec/operator` | `wt-2c4141a265e7` | `wave-v1-1/operator/scratch` | merged |
| A7 token | `spec/domains/token.yaml`, its scenarios | `spec/token` | `wt-10784dd2a894` | `wave-v1-1/token/scratch` | merged |
| A8 config, improvement, host | the three domain files and their scenarios | `spec/config-host` | `wt-f39c1bc353ae` | `wave-v1-1/config-host/scratch` | merged |
| A9 actor everywhere | the six unchanged domains and their scenarios | `spec/actor-everywhere` | `wt-3af7cf9a9ee3` | `wave-v1-1/actor-everywhere/scratch` | merged |
| U1 integration | `spec/components.yaml`, `spec/ess-inputs.yaml` (merged), `crates/conductor-model` (regenerated), `crates/conductor/src/**`, `crates/conductor/tests/**`, `crates/conductor-docs/src/config.rs`, `docs/config.md`, generated reference pages | `impl/spec-v1` | (after A1 to A9 merge) | `wave-v1-1/spec-v1/scratch` | planned |

Worktrees are under the worktree manager's tree root for this repository; scratch directories are
under the session's scratch root. Each authoring unit's build directory is none: the authors run
`ess` only. U1 builds into its own `target/` inside its worktree.

## The verb's answer

`aep plan artifact waves --kind story --status active --format json`, on 2026-11-08 after the
scope of `spec-v1` was written:

```text
wave 1 ['story:onboarding-run']
wave 2 ['story:spec-v1']
collisions: onboarding-run × spec-v1 on crates/conductor-model (cited), crates/conductor/src/cli.rs (inferred), spec/domains/config.yaml (cited), website/docs/reference/cli.md (cited)
unassessed: 35 ids
```

`story:onboarding-run` is the v0 line's, delivered on `main` by its own waves, and not a candidate
for this plan; it is on another branch, so the collision costs nothing here. The 35 unassessed ids
are the epic's later stories (scoped at their wave's stage 1) and the v0 line's backlog.

## Checks

| property | spec-v1 |
|---|---|
| implementable in this tree | yes: `ess` 0.90.0 installed, `spec/ess-inputs.yaml` requires `ess 0.90`; no credential, no paid run |
| surfaces disjoint | the nine authoring units each own distinct files; `spec/ess-inputs.yaml` is appended by each (resolved at merge); U1 runs after them |
| blast radius | `spec/`, the generated model, and every handler and test that builds a generated command (about 30 files each, inferred) |
| scope | cited for `spec/`, the generated tree and the host-type break sites; inferred for the size of the handler and test fallout |
| terminal move needs | `aep plan artifact evidence story:spec-v1 --kind test_result` against the merge commit, then `move --to implemented` |

## Pre-flight

| check | value |
|---|---|
| base branch | `v1` at 04973b59c (`main` after PR #103); the wave branch forked from 7caa341c7 before the merge |
| primary checkout | `main`, clean |
| other trees | six managed trees of other sessions (the v0 line's waves 45 to 47, a profiles branch, two agent trees); none is this wave's |
| free disk | 142 G on `/` before the first tree; floor 120 G |
| build cache | sccache wired through `~/.cargo/config.toml` |
| one measured build | not yet measured; U1's first build is the measurement |
| model budget | none stated; nine authors at once on the operator's word ("use some multiple Opus agents ... each agent taking one domain") |

## Commits this wave makes

The scaffold commit (made), the plan pages commit, one commit per authoring unit on its branch, their
merges into `wave/v1-1`, one commit on `impl/spec-v1` and its merge, the closing store commit, and the
merge of `wave/v1-1` into `v1`. No tag, no release, no merge into `main`.

## Outcome

Interim, 2026-11-08: the nine authoring units are merged on `wave/v1-1` (2ed44ce95); `ess specify
validate --path spec` prints `conductor v1 — 18 file(s), 81 scenario(s), valid`; the header is at
`ess/25`. Open before U1: one reconcile unit (the Actor becomes a struct with a `kind` of Human,
Agent or Daemon so an agent refusal is a `when:` predicate; one `ActorNotHuman` in the operator
domain; the cross-domain relations the authors left to each other; the design and the page updated
to the struct), then U1. The `UNMAPPED:` markers in `spec/` are the owner's open questions
(`grep -rn UNMAPPED spec`).
