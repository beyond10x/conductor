# conductor

Conductor coordinates coding-agent sessions across the repositories of one organization. One
background session, `conductor`, tracks what happens in every repository, starts one controller
session per repository, and takes the routine decisions those controllers raise. Architectural
decisions go to a person, the operator.

The `conductor` CLI is the session's tool: it reads the repositories, keeps the records, and guards
the sessions.

- Design: [`docs/design/conductor.md`](docs/design/conductor.md)
- Config reference: [`docs/config.md`](docs/config.md)
- Working on this repository (agents and contributors): [`AGENTS.md`](AGENTS.md)

## How it works

1. **Sense.** `conductor snapshot start-snapshot` reads every repository of the instance: git
   state, pull requests, `main` CI, releases, open planning blockers, specification status, live
   sessions and, when the instance names a catalog, whether each repository is in it. The result
   is one snapshot in the store.
2. **Compare and decide.** The conductor session compares the snapshot with its goals and answers
   the decision requests controllers sent. Routine decisions (class C) it takes and records.
   Architectural ones (class O) and ones that need a person's hands (class H) go to the operator.
3. **Dispatch.** Conductor sends each controller a brief. A controller works only in its own
   repository and talks only to conductor.
4. **Guard.** A PreToolUse hook, `conductor guard record-guard-decision --from-pre-tool-use`,
   denies a controller's Edit and Write outside its repository and its messages to anyone but
   conductor, and keeps conductor's own edits to its records. It catches mistakes; it is not a
   sandbox (see [Security model](#security-model)).
5. **Watch.** Between cycles, `conductor watch run --follow` wakes conductor when a session exits,
   hits a usage limit or fills its context, when `main` CI turns red or green, or when disk runs
   low.

## What a session reads

Each session follows a generic profile in `.agents/`: `conductor.md`, `conductor-dev.md` and
`repo-controller.md`. A profile names config values as `section.key`, such as
`thresholds.disk_low`; the value is that field of `conductor config show --format json`.

After its profile, every session reads the instance's `rules.md` in the instance's records
directory: the rules of that one organization, one section per role, each rule with its source.
This repository ships no `rules.md`; you write your own.

For Claude Code, `.claude/agents/` holds one adapter per profile (`conductor.md`,
`conductor-dev.md`, `repo-controller.md`), each a pointer at its `.agents/` file, and `.claude/`
holds the two settings files that wire the guard: `conductor-settings.json` and
`controller-settings.json`.

## Install

Prerequisites:

| tool | used for |
|---|---|
| Rust toolchain (edition 2024) and `cargo` | building the CLI |
| `git` | the repositories collector, worktree listing |
| `gh`, logged in | pull requests, CI runs, releases, repository lists |
| `claude` (Claude Code) | the sessions; `claude agents --json` for the sessions collector, the guard, the watch and the dashboard |
| `aep` | open decision blockers across planning stores |
| `ess` | specification status of each repository; this repository's own build (`task check`, `task regen`) |
| `worktree` | the dashboard's disk view of managed worktrees |
| `task` (go-task) and `jq` | the tasks in `Taskfile.yml` |
| `notify-send` (optional) | the watch's desktop notification on a usage limit |

Install the binary:

```console
cargo install --path crates/conductor --locked
```

`task install` runs the same command.

Controllers start in other repositories' checkouts, so link the controller adapter where Claude
Code finds it from anywhere:

```console
ln -s "$PWD/.claude/agents/repo-controller.md" ~/.claude/agents/repo-controller.md
```

## Configure

Write `~/.b10x/conductor/conductor.yaml`. An instance needs a name, its sources and its checkouts;
every other key has a default.

```yaml
version: conductor.config/1
instances:
  - name: personal
    sources:
      - github: example-org
    checkouts:
      root: ~/example-org
      trees: ~/.local/state/worktree/trees/example-org
    records: ~/example-org/conductor
    roles:
      - {role: conductor, harness: claude, model: opus}
      - {role: conductor-dev, harness: claude, model: opus}
      - role: controller
        harness: claude
        model: opus
        agent: repo-controller
        settings: ~/example-org/conductor/.claude/controller-settings.json
```

`records` is the directory the conductor session runs in, and the guard gives conductor its rules
only there. `task conductor:start` starts the session in this checkout, so the example sets
`records` to this checkout (`~/example-org/conductor`). The controller role's `agent` and
`settings` are the adapter and the settings file conductor starts each controller with.

[`conductor.example.yaml`](conductor.example.yaml) has two instances, one with a catalog and one
without. Every field is in [`docs/config.md`](docs/config.md).

Check the file, and see the effective values after defaults:

```console
$ conductor config validate
~/.b10x/conductor/conductor.yaml: valid, 1 instance(s): personal
$ conductor config show
```

Then write the instance's `rules.md` in its records directory.

## Run

From this checkout:

```console
task trust                # mark every checkout under checkouts.root as a trusted Claude Code workspace
task conductor:start      # start the conductor session in the background
task conductor:attach     # open it in this terminal
task sessions             # which conductor sessions are running
task dashboard            # a read-only page on http://127.0.0.1:7313/
```

`task conductor:start` starts Claude Code with the `conductor` agent, the model the instance's
`conductor` role names and `.claude/conductor-settings.json`, and refuses while a session named
`conductor` is running. `task --list` shows every task.

## Security model

Read this before you start sessions on repositories you care about.

- Every session runs Claude Code with `--permission-mode bypassPermissions`: no tool call asks
  you first. Conductor and controllers work unattended because of it.
- The guard is the only check, and it checks five tools: Edit, Write, NotebookEdit, SendMessage
  and Bash. Any other tool, an MCP server's tools included, is never shown to it.
- For Edit, Write and NotebookEdit the guard resolves the target path, links included, and denies
  a write outside the session's own repository, records or scratch directory.
- The guard does not check where a Bash command writes. A controller can write, overwrite or
  delete any file its user can: other checkouts, conductor's records and the `conductor` binary
  included. For Bash the guard denies only a `cd`, `pushd` or `git -C` into another checkout or
  the records, a `gh` write, a `conductor` write, and a write that names a settings file or
  conductor's config, and a command written to avoid those forms is allowed. A controller that reads
  an issue or pull request written by someone else can be told to run such a command.
- `git push` is not checked; the forge's branch protection is the place for that.
- The conductor-dev session starts without the guard.
- `task trust` marks every checkout under `checkouts.root` as a trusted Claude Code workspace, so
  the project settings and hooks of each of those repositories run without a prompt.
- Run conductor under a user account, and with forge credentials, whose reach you accept for an
  unattended agent. An operating-system sandbox for Bash is planned (story `bash-sandbox`).

## Licence

Licence: Apache-2.0, see [`LICENSE`](LICENSE).
