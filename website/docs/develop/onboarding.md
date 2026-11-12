---
title: Developer onboarding
sidebar_position: 1
description: The programs, the gate, the rules, and a first guard change from story to pull request.
lede: You change conductor the way its own agents do, spec first, story first and test first, and one small guard change shows every step.
source: AGENTS.md; Taskfile.yml (check); b10x.toml; spec/ess-inputs.yaml; crates/conductor/dashboard/.nvmrc; .github/workflows/check.yml; crates/conductor/src/guard.rs; crates/conductor/tests/guard_hook.rs; docs/waves/
---

This page is for changing conductor itself. To run it, start at
[Getting started](../getting-started.md).

## Prerequisites

| program | version | used for |
|---|---|---|
| Rust: `cargo`, `rustfmt`, `clippy` | stable, edition 2024 | the workspace |
| `git` | | many tests create repositories |
| `task` (go-task) | | every command in `Taskfile.yml` |
| `ess` | 0.93 | the specification, its conformance run, `crates/conductor-model` |
| `aep` | 0.79 | the planning store under `.engineering/planning/` |
| `worktree` | 0.18.1 | managed worktrees |
| Node.js and `npm` | the version in `crates/conductor/dashboard/.nvmrc` | the dashboard app's tests and build in `task check`, and this site |
| `b10x-gates` (optional) | | the secret scan in `task check`, and the bot route |

The pins come from `b10x.toml`. Its `substrate-daemon` 0.20.0 pin is not needed to build or test. `aep`, `ess`, `worktree` and `b10x-gates` come from the `b10x` marketplace
(`/plugin marketplace add beyond10x/agentplugins` in Claude Code). Claude Code, `gh`, `jq` and a
conductor config are not needed.

## Clone and run the gate

```console
git clone https://github.com/beyond10x/conductor
cd conductor
task check
```

`task check` runs these steps in order and stops at the first that fails:

| step | checks |
|---|---|
| `b10x-gates scan-text` over `git ls-files` | no secret or private text, when `B10X_GATES_POLICY` names a policy |
| `ess specify validate --path spec` | the specification is valid |
| `ess verify conform run … --target interpreted` | the scenarios pass; the report goes to `.check/conformance.json` |
| `ess generate synthesize` into `.check/conductor-model`, then `diff -r` | `crates/conductor-model` matches the specification byte for byte |
| `npm --prefix crates/conductor/dashboard ci`, then `test` | the dashboard app's Vitest suite |
| `task ui`, then `git diff --exit-code -- crates/conductor/dashboard/dist` | the committed `dist/` equals a fresh build, with no untracked file |
| `cargo fmt --check` | formatting |
| `cargo clippy --all-targets -- -D warnings` | lints; any warning fails |
| `LC_ALL=C cargo test --no-fail-fast` | every test in the workspace |
| `task docs-check` | the generated pages are current, and every page under `website/docs` is passive Markdown |

Without `B10X_GATES_POLICY` the first step prints "B10X_GATES_POLICY is not set; the Gates scan
is skipped" and goes on, having checked no secrets. A pull request merges on CI's `task check`
job and its Gates job, so run the full gate locally only where CI cannot run. While you work, run
the test file you touched: `cargo test -p conductor-cli --test <name>`.

## Build from source

The release archive of [getting started](../getting-started.md#1-install) needs no Rust toolchain.
To build a release from source instead, or on a target the release carries no archive for:

```console
cargo install --git https://github.com/beyond10x/conductor --tag v1.5.0 --locked conductor-cli
```

From a checkout, `task install` runs `cargo install --path crates/conductor --locked`. The binary
carries the role profiles and the `claude-code` settings, so nothing else is copied.

## The repository and its rules

The Layout and Rules sections of
[`AGENTS.md`](https://github.com/beyond10x/conductor/blob/main/AGENTS.md) list every directory and
rule. The v1 design is `docs/design/architecture-next.md`. The rules you meet first:

- **Spec first.** A new noun, field, command, view or error goes into `spec/` before any Rust:
  `ess specify validate --path spec`, `task regen`, then the handler against the generated types.
- **Rust with clap derive.** Commit no Python or shell scripts. Hooks are `conductor` subcommands.
- **Generated files are not edited by hand**
  ([generated files and the gate](./regenerate-and-check.md)).
- **No organization in the product.** Use `example-org` and `~/`.
- **Tests never read the operator's real config or state.** A test gets its command from
  `common::conductor` and runs under a temporary `HOME`.
- **Work in a managed worktree**, and do not set `CARGO_TARGET_DIR`.

## Your first change

The guard ([the guard](../security/the-guard.md)) has rules for six tools, and checks the
working directory an `Agent` or `Task` call sets. Any other tool is allowed only from `READ_ONLY`
in `crates/conductor/src/guard.rs` or the role's `read_only_tools`. `READ_ONLY` once lacked
`ScheduleWakeup`, a prompt the session sends itself, and the guard denied it with "ScheduleWakeup
has no rule and is on no read-only allowlist". The entry shipped in v0.2.0. To follow along, take
it out in your tree.

1. **Open a managed worktree:**

   ```console
   worktree create --purpose "guard: allow ScheduleWakeup" --base main
   cd <the path it prints>
   git switch -c guard-schedule-wakeup
   ```

2. **File the story**, with Why, Acceptance and Scope sections, and record its scope
   ([how changes are planned and shipped](./how-we-work.md)):

   ```console
   aep plan artifact new story guard-allow-schedule-wakeup \
     --title "The guard allows ScheduleWakeup, a prompt the session sends itself" \
     --from "$TMPDIR/story.md"
   aep plan artifact scope story:guard-allow-schedule-wakeup \
     --add crates/conductor/src/guard.rs --add crates/conductor/tests/guard_hook.rs
   ```

3. **Write the failing test.** Add the tool to the allowed list of
   `an_unknown_tool_is_denied_and_an_allowlisted_read_tool_is_allowed` in
   `crates/conductor/tests/guard_hook.rs`, which expects exit 0, no output and no store:

   ```rust
               (
                   "ScheduleWakeup",
                   serde_json::json!({ "delaySeconds": 1200, "prompt": "x", "reason": "x" }),
               ),
   ```

   `cargo test -p conductor-cli --test guard_hook` fails with exit 2.

4. **Make the change.** Add the entry with its comment, and raise the length in the type from 23
   to 24. The test passes.

   ```rust
   const READ_ONLY: [&str; 24] = [
       // ...
       // The session's own one-shot wake-up, a prompt to itself (`/loop` without an interval).
       "ScheduleWakeup",
   ];
   ```

5. **Open the pull request.** Commit the code, the test and the story file with a pathspec. The
   message is [public text](./how-we-work.md#what-goes-in-public-text). The project's agents push
   through `b10x-gates bot -- <git args>`.

6. **Record the gate** once the checks are green, and finish the tree after the merge:

   ```console
   aep plan artifact evidence story:guard-allow-schedule-wakeup \
     --kind test_result --source "task check"
   aep plan artifact move story:guard-allow-schedule-wakeup --to implemented
   worktree finish --discard-cache --archive <tree>
   ```

## Why a guard change gets a security review

Sessions run with `--permission-mode bypassPermissions`, and the guard is their only check
([security overview](../security/overview.md)). A `READ_ONLY` entry is allowed to every session of
every instance, before the guard looks at where the session runs, and is not recorded. A wrong
entry opens every instance at once, with no denial left to show it. So a change that loosens a
guard rule gets an `aep:security-reviewer` pass (`docs/waves/wave-04.md`), and one that only
rewords a denial does not (`docs/waves/wave-05.md`).
