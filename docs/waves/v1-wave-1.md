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
| U1 integration | `spec/components.yaml`, `spec/ess-inputs.yaml` (merged), `crates/conductor-model` (regenerated), `crates/conductor/src/**`, `crates/conductor/tests/**`, `crates/conductor-docs/src/config.rs`, `docs/config.md`, generated reference pages | `impl/spec-v1` | (after A1 to A9 merge) | `wave-v1-1/spec-v1/scratch` | merged |

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

Delivered 2026-11-08; `story:spec-v1` is implemented (revision 45), on the `task check` run of
pull request 107, merged into `v1` at 37b16db79.

| unit | result |
|---|---|
| A1 to A9 | nine domains or domain groups authored in parallel, each validated alone and merged |
| R1 reconcile | `Actor` became a struct with a kind (Human, Agent, Daemon), so a person-only refusal is a `when:` predicate synthesis tests; one `ActorNotHuman`; the cross-domain relations; design § 3 and § 13 row 14 |
| spec fixes | the spec adversary's findings: four conformance failures, daemon-refused scenarios, actors on two records, the secret view |
| U1 integration | components `conductord` and `conductor-instance`, an `ess-http/1` binding of 41 routes, the regenerated model, the crate on the new types, clap stubs exiting 2 |
| crate fixes | the crate adversary's findings: the run loop's child steps record as the daemon; an agent session cannot claim a human actor |

| gate | value |
|---|---|
| `ess specify validate --path spec` | 86 scenarios, valid |
| conformance, interpreted target | 652 passed, 0 failed, 0 unsupported |
| `cargo test -p conductor-cli` | 1425 passed, 0 failed, 7 ignored |
| CI on pull request 107 | `task check`, Gates and the documentation build pass |

Deviations from the plan:

- The adversary ran twice, on the merged specification and on the merged crate, and not per
  authoring unit.
- `ess` refuses a client component that owns no domain, so the dashboard and the TUI are not
  components and the v0 domains stay on `conductor-cli` (ESS-COMPONENT-001, 004, 005, 007).
- Three `ess` gaps were filed on the `ess` repository: a union variant cannot be tested in a
  predicate (764), an absent optional field carried into a payload (765), the swapped-caller
  rerun renaming every `instance` string (766).

Left for the owner: 74 `UNMAPPED:` markers in `spec/` (`grep -rn UNMAPPED spec`). Backlog: grants
are not read by any command (the grant model has no effect yet); the guard's refusal of a message
to another config file's instance is gone before conductord replaces it; the token commands'
actor.
