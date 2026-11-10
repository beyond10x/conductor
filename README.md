# conductor

An agent asked to set conductor up in a repository follows [`SETUP.md`](SETUP.md), downloaded
whole from <https://raw.githubusercontent.com/beyond10x/conductor/main/SETUP.md> with `curl -fsSL`
(a summary drops commands): each step has a check, the exact fix, and what only the person can do.

Conductor coordinates Claude Code sessions across the repositories of one organization. Its loop,
`conductor run`, has no model call: it tracks every repository and starts one controller session
per repository. Every decision it takes or relays is a record with a class, a reason and evidence.

Documentation: [beyond10x.github.io/conductor](https://beyond10x.github.io/conductor/). To try it
on a repository you have: [one task in an existing repository](https://beyond10x.github.io/conductor/docs/guides/first-task-existing-repo).

## What it is

| part | command | does |
|---|---|---|
| conductord | `conductor daemon serve` | reads one config file, starts each instance's runtime, and serves the dashboard and the global API on `127.0.0.1:<daemon.port>`, 7300 by default |
| instance runtime | `conductor instance serve` | serves one instance's API on `api.sock` in its state directory, and runs its loop |
| the loop | `conductor run` | every `cadence.watch` the watch and the mailbox, every `cadence.cycle` a snapshot and dispatch, once a day the brief `REPORT.md` and a commit of the records |

The runtime runs the loop beside its API, under the loop's lock, `<state>/run/lock`. The records
go to one store, a SQLite or Postgres database (`daemon.store`). The role profiles are compiled
into the binary from [`crates/conductor/roles/`](crates/conductor/roles/)
(`conductor role show --role <role>`). The rules of one organization go in its own `rules.md`,
and this repository ships none.

## Who it is for

Someone who already works with Claude Code across many repositories and wants the routine
coordination done by agents: what changed, which pull request waits, which CI run is red, which
repository needs work next. It assumes you plan work with AEP stores in each repository and
accept agents that run unattended under your account (see the next section).

## Safety model in short

- **The guard is the only check.** Every session runs with
  `--permission-mode bypassPermissions`, so no tool call asks you first. The PreToolUse hook
  `conductor guard record-guard-decision --from-pre-tool-use` sees every tool call of each
  controller, denies writes and messages outside the session's own places, and records each
  denial. The Bash rules are a heuristic, and `git push` and `curl` to a forge API are not
  checked. The guard catches mistakes. It is no defence against a hostile prompt in an issue or
  pull request a controller reads.
- **Not every session is guarded.** The maintainer runs without the guard by default, a session
  started by hand without the instance's settings is not guarded, and `conductor trust` lets each
  checkout's own project hooks run.
- **Conductor confines no session itself.** An instance's runtime runs in a Substrate box when its
  `placement` names an adapter, and a session that runtime starts runs in the box. A
  `claude --bg` session started on the host has no box. Run conductor under an account whose
  files, forge credentials and tokens you accept an unattended agent reaching, with branch
  protection and scoped tokens on the forge.

The full model, with what is enforced and what is only instructed:
[Security overview](https://beyond10x.github.io/conductor/docs/security/overview), and
[the guard](https://beyond10x.github.io/conductor/docs/security/the-guard) for every rule. To
keep conductor away from the rest of a machine, in a VM, a container or a dedicated account:
[Run conductor isolated](https://beyond10x.github.io/conductor/docs/guides/run-isolated).

## Requirements

| program | pin | used for |
|---|---|---|
| Claude Code (`claude`) | | the sessions; `claude agents --json` for the sessions collector |
| `git` | | the repositories collector, worktree listing |
| `gh`, logged in | | pull requests, CI runs, releases, repository lists of a GitHub source; not needed without one |
| `glab`, logged in to the source's host | | merge requests, pipelines, releases, project lists of a GitLab source; not needed without one |
| `jq` | | the supervisor's and the teacher's context checks, and the shell examples of the documentation |
| `aep` | 0.79 | the planning stores: blockers per repository |
| `ess` | 0.90 | the specifications collector |
| `worktree` | 0.18.1 | managed worktrees for controllers and their workers |
| `substrate-daemon` | 0.20.0 | the box of an instance whose `placement` names an adapter; not needed otherwise |
| `b10x-gates` | | under `forge.route: gates`, the default: the bot route for commits and pushes |
| `notify-send` (optional) | | the watch's desktop notification on a usage limit |

The pins are in [`b10x.toml`](b10x.toml). With no `b10x.toml` in the working directory or a
directory above it, `conductor doctor` checks `aep`, `ess` and `worktree` against the
pins the binary was built with. `aep`, `ess`, `worktree`, `substrate-daemon` and `b10x-gates`
install from their release archives: under each one missing or too old, `conductor doctor`
prints the command, and [`SETUP.md`](SETUP.md) step 4 lists them.

A `gitlab` source names the host its group is on, such as `host: gitlab.example.com`
(`gitlab.com` when absent), and needs `glab` logged in to that host; the
[GitLab guide](https://beyond10x.github.io/conductor/docs/guides/gitlab) sets an instance up.

## Install

Each GitHub Release carries `conductor-<version>-<target>.tar.gz` for `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and `aarch64-apple-darwin`, and one
`SHA256SUMS`. Download the archive for your machine, check it, and put the binary on `PATH`:

```console
version=1.1.0 target=x86_64-unknown-linux-gnu
archive="conductor-$version-$target.tar.gz"
url="https://github.com/beyond10x/conductor/releases/download/v$version"
curl -fsSLO "$url/$archive"
curl -fsSLO "$url/SHA256SUMS"
grep " $archive\$" SHA256SUMS | sha256sum --check --strict
tar -xzf "$archive"
mkdir -p ~/.local/bin
install -m 755 "conductor-$version-$target/conductor" ~/.local/bin/
conductor doctor
```

On macOS, `shasum -a 256 --check` stands in for `sha256sum --check --strict`. `~/.local/bin`
must be on `PATH`. To build from source instead, see
[developer onboarding](https://beyond10x.github.io/conductor/docs/develop/onboarding#build-from-source).

To upgrade, run `conductor upgrade`: it reads the newest release, downloads its archive for the
target the binary was built for and `SHA256SUMS`, checks the sum, runs the new binary's
`--version`, and replaces the binary in place, keeping the old one as `conductor.previous`.
`conductor upgrade --check` only says whether a newer release exists, and
`conductor upgrade --release 0.6.0` installs a named release, older ones too. It needs `curl`,
`tar` and `sha256sum` (or `shasum` on macOS). A running instance stays on its own copy of the
binary until `conductor instance upgrade --instance <name> --tag v<version>` moves it
([upgrade and roll back](https://beyond10x.github.io/conductor/docs/guides/upgrade-and-rollback)).

`conductor --version` prints the installed version, `conductor 1.1.0`. `conductor doctor` lists
each program conductor starts with its version, `not needed` for the forge CLI the instance has no
source on, and ends with `ready`, or `not ready:` and each program missing or older than its pin;
it exits 1 when not ready.

## Quickstart (no sessions)

Write a config file. The default path is `~/.b10x/conductor/conductor.yaml`; `--config` or
`$CONDUCTOR_CONFIG` names another. An instance needs a name, a placement, a session prefix, its
sources and its checkouts, and the daemon reads its store from `daemon.store`:

```yaml
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/store/conductor.sqlite
instances:
  - name: personal
    placement: systemd
    session_prefix: personal
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
catalog; the [config reference](https://beyond10x.github.io/conductor/docs/reference/config) lists every field.

## Run sessions

1. `conductor init` writes the instance's records directory: a git repository with `NORTHSTAR.md`,
   `rules.md` (one section per role), a first hand-over in `docs/handoff/`, and the workspace file.
   It refuses to overwrite a file. Write your organization's rules into `rules.md`.
2. Add the roles to the instance. A controller is the `lead` role, and `conductor spawn` refuses
   an instance without one. A role starts from the profile the binary carries for it and, for a
   lead, the settings the binary carries, which wire the guard; a `profile` or `settings` the role
   names takes their place:

   ```yaml
       roles:
         - role: director
           harness: claude
           model: opus
         - role: lead
           harness: claude
           model: opus
   ```

3. `conductor trust` marks every checkout under `checkouts.root` and the records directory as trusted
   Claude Code workspaces.
4. Accept Claude Code's bypass-permissions disclaimer once, as the OS user the sessions run as:
   run `claude --dangerously-skip-permissions` interactively in a checkout, or set
   `"skipDangerousModePermissionPrompt": true` in that user's `~/.claude/settings.json`. A
   background session takes it only from user or managed settings, not from the role's settings
   file.
5. Start `substrate-daemon` yourself: conductord does not. `placement: systemd` runs the
   instance's runtime in a box of the local `substrate-daemon`, which runs in a delegated cgroup
   subtree, on `$XDG_RUNTIME_DIR/substrate/daemon.sock`, and declares a writable root for
   what the runtime writes (`state-<instance>=<state>`, `records-<instance>=<records>`,
   `cache-<instance>=<cache>`, `store=<the store's directory>`) and the egress aperture
   `conductor`. The store is in a directory of its own: a box writes that whole directory, and
   never the config file or `~/.b10x/conductor/daemon`
   ([getting started](https://beyond10x.github.io/conductor/docs/getting-started#5-start-the-substrate-daemon)).
6. `conductor daemon serve --install` installs conductord as the user unit `conductord.service`. It
   opens each instance's box, starts the runtime in it, and lists them at
   `http://127.0.0.1:7300/v1/instances`. The runtime runs the instance's loop, `conductor run`,
   under `<state>/run/lock`: the watch, the cycle, the mailbox and dispatch from the charters in
   `<records>/charters/`, with no conductor session. A `conductor run` started by hand beside it is
   refused; without conductord, `conductor run --once` runs one watch and one cycle tick.
   `conductor session list` lists the sessions, and the dashboard is at
   `http://127.0.0.1:<daemon.port>/?instance=<name>`, port 7300 by default.

Conductor starts a session with `claude --bg`. A session in `bypassPermissions` mode gets `user`
added to its `--setting-sources`, because Claude Code takes the disclaimer acceptance only from
user or managed settings. The role's settings file wins on every key it sets
([sessions](https://beyond10x.github.io/conductor/docs/security/sessions)).

Nothing is copied to the host to make a role run: no profile under `~/.b10x/conductor/`, no
adapter under `~/.claude/agents/`. At start the session's profile and settings are written into
its state directory, from the binary that starts it.

## How it works

1. **Sense.** `conductor snapshot start-snapshot` reads git state, pull requests, CI, releases, planning blockers and sessions into one snapshot.
2. **Decide.** `conductor run` answers the controllers' decision requests. It decides what can be undone (class C). What cannot be undone (class O) and credentials only the operator can enter (class H) go to the operator.
3. **Dispatch.** `conductor run` starts a controller for each Active repository with a charter. A controller works only in its repository and reports only to conductor, with `conductor message send`.
4. **Guard.** Every tool call of a controller passes the guard, and each denial is recorded.
5. **Watch.** `conductor run` runs the watch every `cadence.watch` and acts on each line: a hand-over, a usage limit, a pull request's checks, a stalled session, low disk.

## Documentation

- Site: [getting started](https://beyond10x.github.io/conductor/docs/getting-started),
  [CLI reference](https://beyond10x.github.io/conductor/docs/reference/cli),
  [config reference](https://beyond10x.github.io/conductor/docs/reference/config),
  [status](https://beyond10x.github.io/conductor/docs/status)
- Config: [config](https://beyond10x.github.io/conductor/docs/concepts/config), which part of
  conductor reads each key
- Design: [`docs/design/architecture-next.md`](docs/design/architecture-next.md) for v1, and
  [`docs/design/conductor.md`](docs/design/conductor.md) for the roles, decision classes and
  message formats

## Contributing

Agents and people working on this repository follow [`AGENTS.md`](AGENTS.md): specification
first, Rust with clap derive, and every command in `Taskfile.yml`. A pull request merges when its
CI gate and its Gates job are green.
[Developer onboarding](https://beyond10x.github.io/conductor/docs/develop/onboarding) walks through
building from source, the gate and a first change, from story to pull request.

## Licence

Apache-2.0, see [`LICENSE`](LICENSE).
