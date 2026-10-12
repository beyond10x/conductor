# conductor

Conductor coordinates Claude Code sessions across the repositories of one organization: one
session tracks every repository and starts one controller session per repository.

Documentation: [beyond10x.github.io/conductor](https://beyond10x.github.io/conductor/).

## What it is

- A background Claude Code session, `conductor`, that reads the state of every repository of an
  instance, starts one controller session per repository, and answers the routine decisions those
  controllers raise. Architectural decisions, spending and anything public go to a person, the
  operator.
- The `conductor` CLI, written in Rust. It reads the repositories, keeps the records in an
  append-only store, guards every tool call of the sessions, watches for events, and serves a
  read-only dashboard.
- Three generic profiles in [`.agents/`](.agents/) that tell each session what to do. The rules of
  one organization go in its own `rules.md`; this repository ships none.

## Who it is for

Someone who already works with Claude Code across many repositories and wants the routine
coordination done by agents: what changed, which pull request waits, which CI run is red, which
repository needs work next. It assumes you plan work with AEP stores in each repository and
accept agents that run unattended under your account (see the next section).

## Safety model in short

- **Bypass permissions.** Every session runs with `--permission-mode bypassPermissions`: no tool
  call asks you first. That is what lets conductor and controllers work unattended.
- **The guard is the only check.** A PreToolUse hook,
  `conductor guard record-guard-decision --from-pre-tool-use`, sees every tool call of conductor
  and of each controller. It denies a file write outside the session's own repository, records or
  scratch, a message to anyone but the session's own peers, any `gh` call that is not a read, and
  any tool it has no rule for, MCP tools included. A missing binary blocks the call.
- **Bash is not confined.** The Bash rules are a heuristic: they deny a few forms (a `cd` into
  another checkout, a `gh` write, a write naming a settings or config path), not where a command
  writes. `git push` and `curl` to a forge API are not checked. The guard catches mistakes; it is
  no defence against a hostile prompt in an issue or pull request a controller reads.
- **Not every session is guarded.** conductor-dev runs without the guard by default, a session
  started by hand without the instance's settings is not guarded, and `task trust` lets each
  checkout's own project hooks run. There is no operating-system sandbox yet.
- **Run it under an account whose reach you accept** for an unattended agent: its files, its forge
  credentials and its tokens. Use branch protection and scoped tokens on the forge.

The full model, with what is enforced and what is only instructed:
[Security overview](https://beyond10x.github.io/conductor/docs/security/overview).

## Requirements

| program | pin | used for |
|---|---|---|
| Claude Code (`claude`) | | the sessions; `claude agents --json` for the sessions collector and the tasks |
| Rust toolchain and `cargo` | edition 2024 | building and installing the CLI |
| `git` | | the repositories collector, worktree listing |
| `gh`, logged in | | pull requests, CI runs, releases, repository lists of a GitHub source |
| `task` (go-task) | | the tasks in [`Taskfile.yml`](Taskfile.yml) |
| `jq` | | the tasks in `Taskfile.yml` |
| `aep` | 0.69 | the planning stores: blockers per repository |
| `ess` | 0.56 | the specification gate of `task check` |
| `worktree` | 0.13 | managed worktrees for controllers and their workers |
| `b10x-gates` | | the bot route for commits and pushes; the secret scan of `task check` |
| `notify-send` (optional) | | the watch's desktop notification on a usage limit |

The pins are in [`b10x.toml`](b10x.toml). `aep`, `ess`, `worktree` and `b10x-gates` come from the
`b10x` marketplace (`/plugin marketplace add beyond10x/agentplugins` in Claude Code).

## Install

```console
cargo install --git https://github.com/beyond10x/conductor --tag v0.3.0 --locked conductor-cli
conductor doctor
```

From a checkout, `task install` does the same. `conductor doctor` lists each program conductor
starts with its version, and exits 1 when one is missing or older than its pin.

## Quickstart (no sessions)

Write a config file. The default path is `~/.b10x/conductor/conductor.yaml`; `--config` or
`$CONDUCTOR_CONFIG` names another. An instance needs a name, its sources and its checkouts:

```yaml
version: conductor.config/1
instances:
  - name: personal
    sources:
      - local: ~/example-org
    checkouts:
      root: ~/example-org
      trees: ~/.local/state/worktree/trees/example-org
```

Then check it, take one snapshot, and read it:

```console
conductor config validate        # exit 1 naming each problem by its YAML path
conductor config show            # the effective values after defaults
conductor snapshot start-snapshot
conductor repository activity    # each repository Active or Inactive
```

`snapshot start-snapshot` refuses up front, naming the program, when one it needs is missing.
[`conductor.example.yaml`](conductor.example.yaml) has a fuller instance, a GitHub source and a
catalog; [`docs/config.md`](docs/config.md) lists every field.

## Run sessions

1. `conductor init` writes the instance's records directory: a git repository with `NORTHSTAR.md`,
   `rules.md` (one section per role), a first hand-over in `docs/handoff/`, and the workspace file.
   It refuses to overwrite a file. Write your organization's rules into `rules.md`.
2. Add the roles to the instance. Each `profile` points at this checkout's `.agents/` file, each
   `settings` at its `.claude/` settings file, which wires the guard:

   ```yaml
       roles:
         - role: conductor
           harness: claude
           model: opus
           profile: ~/example-org/conductor/.agents/conductor.md
           settings: ~/example-org/conductor/.claude/conductor-settings.json
         - role: conductor-dev
           harness: claude
           model: opus
           profile: ~/example-org/conductor/.agents/conductor-dev.md
         - role: controller
           harness: claude
           model: opus
           profile: ~/example-org/conductor/.agents/repo-controller.md
           settings: ~/example-org/conductor/.claude/controller-settings.json
   ```

3. `task trust` marks every checkout under `checkouts.root` and the records directory as trusted
   Claude Code workspaces.
4. Accept Claude Code's bypass-permissions disclaimer once: run
   `claude --dangerously-skip-permissions` interactively in the records directory, or set
   `"skipDangerousModePermissionPrompt": true` in the role's settings file. `task conductor:start`
   refuses until one is in place.
5. `task conductor:start` starts conductor in the background in the records directory;
   `task conductor:attach` opens it in this terminal. `task sessions` lists the sessions and
   `task dashboard` serves a read-only page on http://127.0.0.1:7313/.

Sessions start with `--setting-sources project,local`, so your user-level settings and agents are
not loaded; the role's settings file carries what the session needs.

## How it works

1. **Sense.** `conductor snapshot start-snapshot` reads git state, pull requests, CI, releases, planning blockers and sessions into one snapshot.
2. **Decide.** Conductor compares the snapshot with its goals; class C it decides, class O and class H go to the operator.
3. **Dispatch.** Conductor sends each controller a brief; a controller works only in its repository and reports only to conductor.
4. **Guard.** Every tool call of conductor and the controllers passes the guard; denials are recorded.
5. **Watch.** `conductor watch run --follow` wakes conductor when a session ends, CI changes or disk runs low.

## Documentation

- Site: [getting started](https://beyond10x.github.io/conductor/docs/getting-started),
  [CLI reference](https://beyond10x.github.io/conductor/docs/reference/cli),
  [config reference](https://beyond10x.github.io/conductor/docs/reference/config),
  [status](https://beyond10x.github.io/conductor/docs/status)
- Design: [`docs/design/conductor.md`](docs/design/conductor.md)
- Config fields: [`docs/config.md`](docs/config.md)

## Contributing

Agents and people working on this repository follow [`AGENTS.md`](AGENTS.md): specification
first, Rust with clap derive, every command in `Taskfile.yml`, and `task check` before a push.
[Developer onboarding](https://beyond10x.github.io/conductor/docs/develop/onboarding) walks through
the setup and a first change, from story to pull request.

## Licence

Apache-2.0, see [`LICENSE`](LICENSE).
