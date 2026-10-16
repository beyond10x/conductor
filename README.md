# conductor

An agent asked to set conductor up in a repository follows [`SETUP.md`](SETUP.md), the step list
at <https://raw.githubusercontent.com/beyond10x/conductor/main/SETUP.md>: each step has a check,
the exact fix, and what only the person can do.

Conductor coordinates Claude Code sessions across the repositories of one organization: one
session tracks every repository and starts one controller session per repository.

Documentation: [beyond10x.github.io/conductor](https://beyond10x.github.io/conductor/). To try it
on a repository you have: [one task in an existing repository](https://beyond10x.github.io/conductor/docs/guides/first-task-existing-repo).

## What it is

- A background Claude Code session, `conductor`, that reads the state of every repository of an
  instance, starts one controller session per repository, and answers the routine decisions those
  controllers raise. Architectural decisions, spending and anything public go to a person, the
  operator.
- The `conductor` CLI, written in Rust. It reads the repositories, keeps the records in an
  append-only store, guards every tool call of the sessions, watches for events, and serves a
  dashboard where the operator reads the state and answers decisions.
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
  scratch, a message to anyone but the session's own peers, any `gh` or `glab` call that is not a
  read, and any tool it has no rule for, MCP tools included. A missing binary blocks the call.
- **Bash is not confined.** The Bash rules are a heuristic: they deny a few forms (a `cd` into
  another checkout, a `gh` or `glab` write, a write naming a settings or config path), not where a
  command writes. A Bash command is decided on its write targets: a read of a protected file
  redirected into scratch passes, a redirection or `sed -i` into it does not. `git push` and
  `curl` to a forge API are not checked. The guard catches mistakes; it is
  no defence against a hostile prompt in an issue or pull request a controller reads.
- **Not every session is guarded.** conductor-dev runs without the guard by default, a session
  started by hand without the instance's settings is not guarded, and `task trust` lets each
  checkout's own project hooks run. There is no operating-system sandbox yet.
- **Run it under an account whose reach you accept** for an unattended agent: its files, its forge
  credentials and its tokens. Use branch protection and scoped tokens on the forge.

The full model, with what is enforced and what is only instructed:
[Security overview](https://beyond10x.github.io/conductor/docs/security/overview). To keep
conductor away from the rest of a machine today, in a VM, a container or a dedicated account:
[Run conductor isolated](https://beyond10x.github.io/conductor/docs/guides/run-isolated).

## Requirements

| program | pin | used for |
|---|---|---|
| Claude Code (`claude`) | | the sessions; `claude agents --json` for the sessions collector and the tasks |
| Rust toolchain and `cargo` | edition 2024 | building and installing the CLI from source |
| `git` | | the repositories collector, worktree listing |
| `gh`, logged in | | pull requests, CI runs, releases, repository lists of a GitHub source; not needed without one |
| `glab`, logged in to the source's host | | merge requests, pipelines, releases, project lists of a GitLab source; not needed without one |
| `task` (go-task) | | the tasks in [`Taskfile.yml`](Taskfile.yml) |
| `jq` | | the tasks in `Taskfile.yml` |
| `aep` | 0.71 | the planning stores: blockers per repository |
| `ess` | 0.59 | the specification gate of `task check` |
| `worktree` | 0.18 | managed worktrees for controllers and their workers |
| `b10x-gates` | | the bot route for commits and pushes; the secret scan of `task check` |
| `notify-send` (optional) | | the watch's desktop notification on a usage limit |

The pins are in [`b10x.toml`](b10x.toml). With no `b10x.toml` in the working directory or a
directory above it, `conductor doctor` checks `aep`, `ess` and `worktree` against the
pins the binary was built with. `aep`, `ess`, `worktree` and `b10x-gates` come from the `b10x`
marketplace (`/plugin marketplace add beyond10x/agentplugins` in Claude Code).

A `gitlab` source names the host its group is on, such as `host: gitlab.example.com`
(`gitlab.com` when absent), and needs `glab` logged in to that host; the
[GitLab guide](https://beyond10x.github.io/conductor/docs/guides/gitlab) sets an instance up.

## Install

```console
cargo install --git https://github.com/beyond10x/conductor --tag v0.15.0 --locked conductor-cli
conductor doctor
```

From a checkout, `task install` does the same. Without a Rust toolchain, take the pre-built binary
from the release instead: each GitHub Release from `v0.6.0` on carries
`conductor-<version>-<target>.tar.gz` for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
`x86_64-apple-darwin` and `aarch64-apple-darwin`, and one `SHA256SUMS`:

```console
version=0.15.0 target=x86_64-unknown-linux-gnu
archive="conductor-$version-$target.tar.gz"
gh release download "v$version" -R beyond10x/conductor -p "$archive" -p SHA256SUMS
grep " $archive\$" SHA256SUMS | sha256sum --check --strict
tar -xzf "$archive"
install -m 755 "conductor-$version-$target/conductor" ~/.local/bin/
conductor doctor
```

On macOS, `shasum -a 256 --check` stands in for `sha256sum --check --strict`.

To upgrade, run `conductor upgrade`: it reads the newest release, downloads its archive for the
target the binary was built for and `SHA256SUMS`, checks the sum, runs the new binary's
`--version`, and replaces the binary in place, keeping the old one as `conductor.previous`.
`conductor upgrade --check` only says whether a newer release exists, and
`conductor upgrade --release 0.6.0` installs a named release, older ones too. It needs `curl`,
`tar` and `sha256sum` (or `shasum` on macOS).

`conductor --version` prints the installed version, `conductor 0.15.0`. `conductor doctor` lists
each program conductor starts with its version, `not needed` for the forge CLI the instance has no
source on, and ends with `ready`, or `not ready:` and each program missing or older than its pin;
it exits 1 when not ready.

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
           profile: ~/.b10x/conductor/profiles/conductor.md
           settings: ~/example-org/conductor/.claude/conductor-settings.json
         - role: conductor-dev
           harness: claude
           model: opus
           profile: ~/.b10x/conductor/profiles/conductor-dev.md
         - role: controller
           harness: claude
           model: opus
           profile: ~/.b10x/conductor/profiles/repo-controller.md
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
   `task dashboard` serves the dashboard on http://127.0.0.1:7313/.

Sessions start with `--setting-sources project,local`, so your user-level settings and agents are
not loaded; the role's settings file carries what the session needs.

For an interactive session of your own in one repository, `task agents:link REPO=<path>` links
this checkout's agents (`conductor`, `repo-controller`, `conductor-dev`) into
`<path>/.claude/agents/` and lists each link in that repository's `.git/info/exclude`. It writes
nothing to `~/.claude/agents/`, and removes the links an earlier version of the task left there.

## How it works

1. **Sense.** `conductor snapshot start-snapshot` reads git state, pull requests, CI, releases, planning blockers and sessions into one snapshot.
2. **Decide.** Conductor compares the snapshot with its goals; it decides everything it can undo (class C); only what cannot be undone (class O) and credentials only the operator can enter (class H) go to the operator.
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
