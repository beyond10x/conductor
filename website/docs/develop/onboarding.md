---
title: Developer onboarding
sidebar_position: 1
description: Set up a checkout of conductor, run the gate, learn the layout and the rules, then make a first real change to the guard from story to pull request.
lede: You change conductor the way its own agents do, spec first, story first and test first, and one small guard change shows every step.
source: AGENTS.md; Taskfile.yml; b10x.toml; spec/ess-inputs.yaml; crates/conductor/src/guard.rs; crates/conductor/tests/guard_hook.rs; docs/waves/
---

## What you will do

This page is for people who want to change conductor itself: its CLI, its specification, its
profiles or this site. You will:

1. install the programs the repository needs;
2. clone it and run the gate, `task check`, once;
3. take a short tour of the layout and the rules every change follows;
4. make a first change: let the guard allow one more read-only harness tool, from a story to a
   pull request.

Plan on an hour or more. Most of it is the first `task check`, which compiles the whole workspace
and runs every test. The change itself is a few lines.

If you only want to run conductor, start at [Getting started](../getting-started.md) instead.

## Prerequisites

| program | version | used for |
|---|---|---|
| Rust toolchain: `cargo`, `rustfmt`, `clippy` | stable, with edition 2024 | building, formatting, linting and testing the workspace |
| `git` | | the repository; many tests create small repositories |
| `task` (go-task) | | every command in `Taskfile.yml` |
| `ess` | 0.59 (`b10x.toml`); the specification needs at least 0.59 (`requires:` in `spec/ess-inputs.yaml`) | validating the specification, the conformance run, generating `crates/conductor-model` |
| `aep` | 0.71 | the planning store under `.engineering/planning/` |
| `worktree` | 0.18 | managed worktrees |
| `b10x-gates` (optional) | | the secret scan in `task check`, and the bot route maintainers commit and push through |
| Node.js and `npm` (optional) | 22 in CI | previewing this site |

The repository has no `rust-toolchain` file; any current stable toolchain that supports edition
2024 builds it. The pins for `aep`, `ess` and `worktree` are in `b10x.toml`; `conductor doctor`
checks the installed versions against them once you have a `conductor` binary. `aep`, `ess`,
`worktree` and `b10x-gates` come from the `b10x` marketplace (`/plugin marketplace add
beyond10x/agentplugins` in Claude Code).

You do not need Claude Code, `gh`, `jq` or a conductor config to build and test conductor. The
tests set their own `HOME` and never read a real config.

## Clone and run the gate

```console
git clone https://github.com/beyond10x/conductor
cd conductor
task check
```

`task check` is the gate. It runs, in order, and stops at the first step that fails:

| step | what it checks |
|---|---|
| `b10x-gates scan-text` over every tracked file | no secret or private text, when `B10X_GATES_POLICY` names a policy |
| `ess specify validate --path spec` | the specification is valid |
| `ess verify conform run … --target interpreted` | the specification's scenarios pass; the report goes to `.check/conformance.json` |
| `ess generate synthesize` into `.check/conductor-model`, then `diff -r` against `crates/conductor-model` | the generated crate matches the specification byte for byte |
| `cargo fmt --check` | formatting |
| `cargo clippy --all-targets -- -D warnings` | lints; any warning fails |
| `cargo test` | every test in the workspace |
| `task docs-check` | the generated site pages are current, and every page under `website/docs` is passive Markdown |

Without `B10X_GATES_POLICY` the first step prints "B10X_GATES_POLICY is not set; the Gates scan
is skipped" and the gate goes on. That run checked no secrets, so it is not the gate a maintainer
merges on; it is fine for working locally.

:::tip[A faster loop]
While you work, run only the test file you touched: `cargo test -p conductor-cli --test
<name>`. Run the full `task check` once, before you push. Run at most one at a time per
repository.
:::

## The repository

| path | what it holds |
|---|---|
| `crates/conductor/` | the `conductor` CLI: handlers, collectors, guard, watch, dashboard, store; `tests/` holds the integration tests |
| `crates/conductor-docs/` | the documentation generator: the reference pages, the status page, the site's provenance |
| `crates/conductor-model/` | Rust generated from `spec/` by `ess`; never edited by hand |
| `spec/` | the ESS specification: domains, components, scenarios; the source of truth for every noun and command |
| `.engineering/planning/` | the AEP planning store: one Markdown file per epic, story or decision |
| `.agents/` | the three generic profiles: `conductor.md`, `conductor-dev.md`, `repo-controller.md` |
| `.claude/` | the Claude Code adapters for those profiles, and the two settings files that wire the guard |
| `website/` | this site (Docusaurus); some pages are generated |
| `docs/design/` | the design, `conductor.md`; the sections the rules below refer to |
| `docs/waves/` | one page per wave of delivered stories |
| `Taskfile.yml` | every command |

## The rules that shape every change

The full list is the Rules section of [`AGENTS.md`](https://github.com/beyond10x/conductor/blob/main/AGENTS.md).
The ones you meet first:

- **Spec first** (Rules, "Spec first"). A new noun, field, command, view or error goes into
  `spec/` before any Rust: `ess specify validate --path spec`, then `task regen`, then the handler
  in `crates/conductor` against the generated types.
- **Rust, with clap derive** (Rules, "clap derive only" and "Rust only"). Running code is Rust; the
  CLI parses arguments with clap's derive API; hooks are `conductor` subcommands.
- **Every command is a task** (Commands, and Rules, "Rust only"). Commands live in `Taskfile.yml`.
  Do not commit Python or shell scripts.
- **Generated files are never edited by hand** (Rules, "Spec first"; Commands). That covers
  `crates/conductor-model/`, `website/docs/reference/`, `website/docs/status.md` and
  `website/data/status.json`. See [Generated files and the gate](./regenerate-and-check.md).
- **No organization in the product** (Rules, "No organization in the product"). Code, tests,
  specification and tasks name no organization. Use `example-org` in code, tests and fixtures,
  and write `~/` rather than an absolute home path. `crates/conductor/tests/no_organization.rs`
  enforces it.
- **Tests never read the operator's real config or state** (Rules). A test that spawns
  `conductor` gets its command from `common::conductor` and runs under a temporary `HOME`.
- **Work in a managed worktree** (Worktrees and builds). Not in the primary checkout, and never
  with `CARGO_TARGET_DIR` set.

## Your first change

The guard is the hook that sees every tool call of conductor and its controllers
([The guard](../security/the-guard.md)). Six tools have rules of their own. Any other tool is
allowed only if it is on a built-in list of harness tools that write no file, run no command and
reach no other session; everything else is denied.

That list is `READ_ONLY` in `crates/conductor/src/guard.rs`. Each entry has a comment saying why
it is there, and the array's length is part of its type:

```rust
const READ_ONLY: [&str; 24] = [
    // Reads a file.
    "Read",
    // Lists paths matching a pattern.
    "Glob",
    // ...
    // The session's own wake-up schedule, a prompt to itself (conductor's cycle jobs,
    // `.agents/conductor.md`): create, list, delete.
    "CronCreate",
    "CronList",
    "CronDelete",
    // The session's own one-shot wake-up, a prompt to itself (`/loop` without an interval).
    "ScheduleWakeup",
];
```

The last entry is a real past change. The guard denied conductor's `ScheduleWakeup` calls, which
showed up in `conductor guard guard-decisions` with the reason "ScheduleWakeup has no rule and is
on no read-only allowlist". `ScheduleWakeup` only schedules a prompt the session sends itself,
as the Cron tools do, so it belongs on the list. The change shipped with release v0.2.0. Below
it is done step by step, the way a new change should be done. To follow along, pick a tool that
is not on the list yet, or take the entry out in your tree and put it back.

### 1. Open a managed worktree

Filing a story writes a file under `.engineering/planning/`, so open the tree first and do
everything in it:

```console
worktree create --purpose "guard: allow ScheduleWakeup" --base main
cd <the path it prints>
git switch -c guard-schedule-wakeup
```

`worktree create` makes a detached linked worktree from `main` and registers it, so it can be
finished and cleaned up safely later. `git switch -c` gives your work a branch.

### 2. Plan it as a story

Write the story's body to a file, outside the tree:

```markdown
## Why

The guard denies ScheduleWakeup (conductor guard guard-decisions). It is a one-shot prompt the
session sends itself, as CronCreate is, and writes no file, runs no command and reaches no
other session.

## Acceptance

- The guard allows ScheduleWakeup to every session, in a repository checkout and in the records,
  and the allowance opens no store.
- An unknown tool is still denied with a reason naming the read-only allowlist.

## Scope

`crates/conductor/src/guard.rs` (READ_ONLY), `crates/conductor/tests/guard_hook.rs`,
`website/docs/security/the-guard.md` (the list of allowed tools).
```

Then file it, and record the files it lands on so a wave can be planned around it:

```console
aep plan artifact new story guard-allow-schedule-wakeup \
  --title "The guard allows ScheduleWakeup, a prompt the session sends itself" \
  --from "$TMPDIR/story.md"
aep plan artifact scope story:guard-allow-schedule-wakeup \
  --add crates/conductor/src/guard.rs \
  --add crates/conductor/tests/guard_hook.rs \
  --add website/docs/security/the-guard.md
```

The story starts as `draft`. When it has been reviewed, move it on:

```console
aep plan artifact move story:guard-allow-schedule-wakeup --to proposed
aep plan artifact move story:guard-allow-schedule-wakeup --to active
```

[How changes are planned and shipped](./how-we-work.md) explains the lifecycle.

### 3. Write the failing test first

The test that holds the list is `an_unknown_tool_is_denied_and_an_allowlisted_read_tool_is_allowed`
in `crates/conductor/tests/guard_hook.rs`. It sends each tool in a list as a PreToolUse payload,
from a repository checkout and from the records, expects exit 0 and no output, and checks that
no store was created. Then it sends an MCP write tool and an unknown tool and expects both to be
denied and recorded.

Add the new tool to the allowed list, with an input of the shape the harness sends:

```rust
            ("ToolSearch", serde_json::json!({ "query": "x" })),
            (
                "ScheduleWakeup",
                serde_json::json!({ "delaySeconds": 1200, "prompt": "x", "reason": "x" }),
            ),
```

Run that one test file and watch it fail:

```console
cargo test -p conductor-cli --test guard_hook an_unknown_tool_is_denied_and_an_allowlisted_read_tool_is_allowed
```

It fails because the guard exits 2 for `ScheduleWakeup`; the assertion names the tool and prints
the guard's reason. A test that passes before the change proves nothing about the change.

### 4. Make the change

Add the entry to `READ_ONLY` with its comment, and raise the length in the type from 23 to 24:

```rust
const READ_ONLY: [&str; 24] = [
    // ...
    // The session's own one-shot wake-up, a prompt to itself (`/loop` without an interval).
    "ScheduleWakeup",
];
```

If you forget the length, the compiler refuses the array; that is the point of writing it in the
type. Update the list of allowed tools in `website/docs/security/the-guard.md` too, since it
names every entry.

Run the test file again; it passes:

```console
cargo test -p conductor-cli --test guard_hook
```

### 5. Run the gate

```console
task check
```

Every step must exit 0. For the gate that counts, set `B10X_GATES_POLICY` so the scan runs; a
run that printed "the Gates scan is skipped" is not a gate (Commits, gates and public text). When
it has run, `cargo clean` frees the tree's `target/`.

### 6. Record the evidence and finish the story

Record the gate on the story, then move it:

```console
aep plan artifact evidence story:guard-allow-schedule-wakeup \
  --kind test_result --source "task check"
aep plan artifact move story:guard-allow-schedule-wakeup --to implemented
```

`move` finds the evidence recorded on the story without being told. `aep plan artifact explain
story:guard-allow-schedule-wakeup` prints what each move was decided on.

### 7. Open a pull request

Commit the code, the test, the page and the story file with a pathspec, push the branch and open
a pull request against `main`. The commit message says what changed and why, in public words: no
private repository, session or host (see [public text](./how-we-work.md#what-goes-in-public-text)).
Maintainers and the project's agents commit, push and open pull requests through the bot route,
`b10x-gates bot -- <git args>`. Once the branch is merged, finish the tree:

```console
worktree finish --discard-cache --archive <tree>
```

### Why a guard change gets a security review

Sessions run with `--permission-mode bypassPermissions`: no tool call asks a person first. The
guard is the only check ([Security overview](../security/overview.md)). An entry on `READ_ONLY`
is allowed to every session of every instance, before the guard looks at where the session runs
or what the call names, and the allowance is not recorded. A wrong entry is therefore a silent
hole in every instance at once, and the denial record that would have shown it is gone.

So a change that loosens a guard rule (a new allowlist entry, a narrower Bash form, a wider write
place) gets a second, independent look before it merges: does the tool really write no file, run
no command and reach no other session, under every input the harness accepts? The project's
waves use an `aep:security-reviewer` pass for that (see `docs/waves/wave-04.md`). A change that
only rewords a denial the guard already gave does not need one (`docs/waves/wave-05.md`). Either
way, a change to a guard rule is reported with its diff (Rules, "Profiles stay generic").
