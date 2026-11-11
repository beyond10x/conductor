---
title: Config
sidebar_position: 3
description: One conductor.config/2 file names the daemon, the global defaults and every instance; how a command finds the file and the instance, the three levels of a setting, how a key is changed or a /1 file migrated, and which part of conductor reads each key.
lede: One file is the desired state of a deployment, and an instance is one organization in it, so one generic profile serves every organization.
source: docs/design/architecture-next.md section 9; spec/domains/config.yaml; crates/conductor/src/config.rs, config/convert.rs, config/put.rs, config/effective.rs, daemon/api.rs, daemon/install.rs, build.rs, metric.rs, attach.rs, trust.rs; spec/domains/session.yaml; conductor config --help
---

## One file

The file is `conductor.config/2`. `daemon` is conductord's own section, `globals` the defaults
every instance starts from, `instances` the instances it reconciles. An instance names where its
repositories come from, where their checkouts are, where conductor keeps its records, state and
cache, its placement, its roles, and its cadence, thresholds and authority. The
[config reference](/docs/reference/config) lists every key with its type, default and rule.

```yaml
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/store/conductor.sqlite
globals:
  roles:
    - {role: supervisor, harness: claude, model: opus}
instances:
  - name: example-org
    session_prefix: example
    placement: linux
    sources:
      - github: example-org
    checkouts:
      root: ~/example-org
      trees: ~/.local/state/worktree/trees/example-org
    forge: {route: direct}
```

An instance needs `name`, `placement`, `sources` and `checkouts`. `config validate` also refuses
one without `session_prefix`. conductord starts only on a file that writes `daemon.store`
([the host](./the-host.md)). `forge: {route: direct}` lets a controller write its pull requests
with the forge login; the default route, `gates`, writes through the bot route and needs a
`gates_policy` ([forge calls](/docs/security/the-guard#forge-calls)), and `conductor doctor` is
not ready on it without one. Writing a file is [configure an instance](/docs/guides/configure-an-instance).

## Which file, which instance

| what | order |
|---|---|
| file | `--config`, else `CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml` |
| instance | `--instance`, where the command takes it, else `CONDUCTOR_INSTANCE`, else the instance whose `records`, `state`, `checkouts.root` or `checkouts.trees` holds the working directory (the deepest), else the file's `default`, else its only instance |

A file `--config` or `CONDUCTOR_CONFIG` names must exist. Without the default file a built-in
instance applies: name and GitHub owner `example-org`, checkouts under `~/example-org`, trees under
`~/.local/state/worktree/trees/example-org`, the working directory as `records`, `state/` under it,
the cache `~/.cache/conductor-watch`, every other key at its default. Write a file before you run
conductor for real.

Every command resolves the instance first, and a file that does not validate stops it. A command
inside another instance's tree acts for that instance, and is refused (`AmbiguousInstance`) where
two instances' trees meet at the same depth. The guard denies every call while the file does not
load (`ConfigUnloaded`).

## Levels

A key an instance writes overrides the global value, key by key, and a role's own key overrides
both: role over instance over global, then conductor's built-in default. An instance role takes each
key it leaves out from the global role of the same name. An instance that writes no `roles` takes
the built-in roles, each with every key its global role writes. A global role no instance names,
such as `supervisor`, stays the daemon's, and its session is named after the role, with no prefix.

`conductor config show --effective [--instance <i>]` prints every key with its value and level,
`default`, `global`, `instance` or `role`: one `<path>: <value> (<level>)` line each, a role's key
under `roles.<role>.`. With `--format json` or `yaml` it prints the rows of `EffectiveSettings`.

A profile writes a value as `section.key`, such as `thresholds.gate_stop`: that field of `conductor
config show --format json` for the session's instance.

## Validate, show, change

`conductor config validate` prints each problem with its YAML path, such as
`instances[0].cadence.watch`, and exits 1. `conductor config show` prints the effective instance,
every path absolute, as `text`, `json` or `yaml`, and writes an absent optional value as `null`.

`conductor config put --level global|instance|role [--instance <i>] [--role <r>] --path <key>
[--value <v>] --reason <why>` changes one key. `--value` is read as YAML. Without it the key is
removed at that level. A level with the wrong scope is refused (`LevelScopeMismatch`), and a file
that would not validate (`ConfigInvalid`). An agent's change is refused (`ActorNotHuman`) unless
`globals.tuning.keys` names the path and the value is a number inside its range. An accepted change
rewrites the whole file, so its comments are not kept, commits it alone with the actor in the
subject and the reason in the body, and records a `ConfigRevision` under the daemon's tenant. A
failed commit puts the old text back. `PUT /v1/config` takes the same change as JSON from an
operator holding `ChangeSettings`, and answers `422`, or `403` for `ActorNotHuman`.

## Migrate a `conductor.config/1` file

`conductor config migrate` reads a `/1` file once and writes it as `/2`, one note per change:

- `dashboard_port`, `confinement.mode`, `confinement.provider` and `store` are dropped
- `confinement.limits` moves to the instance's `limits`
- `placement` is written as the name the `/1` provider reads as
- the roles of `~/.b10x/conductor/host/host.yaml` move to `globals.roles`

It writes no `daemon` section. The old file is kept as `<file>.config-1.bak`. A result that would not
validate is refused and nothing is written. It also writes a `session_prefix` into each instance
without one: its name in letters, digits and `-`, with 2, 3, … appended on a clash. `--check`
prints the notes and writes nothing.

conductord refuses a `/1` file on one line naming `conductor config migrate`. For one release a
`/1` file still loads for the other commands. Its keys are in
[`docs/config.md`](https://github.com/beyond10x/conductor/blob/main/docs/config.md).

## Session names

Session names are one namespace for the machine: `<session_prefix>-conductor`,
`<session_prefix>-conductor-dev`, and `<session_prefix>-<repo>` for a controller, a nested
`<group>/<repo>` written `<group>-<repo>`. A role's `session_name` is derived, and the file may
write only that value. `config validate` refuses two instances with one prefix, a prefix that starts with
another followed by `-`, and a prefix followed by `-` that begins a bare name another instance
keeps, such as `conductor-dev`. For one release an instance without a prefix keeps the bare names.
The guard lets a conductor message only its own instance's sessions. `conductor session attach
--name <name>` refuses a name two instances hold (`Ambiguous`).

## Workspace and sources

`conductor config workspace` writes `<records>/.engineering/workspace.yaml` (`aep.workspace/1`)
with no network read: one member per checkout with an `origin` directly under `checkouts.root` for a
`github` source, directly under its path for a `local` source, and under `checkouts.root` with an
`origin` on the host and group for a `gitlab` source, each but its `exclude`. The records
directory is a member too. A member's `name` is its directory's name. Two sources that give one name to two
directories are refused. `--check` exits 1 and prints each member that differs. Run it after a
repository is cloned or removed.

A repository is the directory holding `.git` at depth 1 or 2 under a root. Sources are `github:
<owner>` through `gh`, `local: <path>`, and `gitlab: <group>` on `host` through `glab api
--hostname` ([GitLab](/docs/guides/gitlab)).

## Values and secrets

A path is absolute or starts with `~/`. A duration is a whole number above 0 with `s`, `m`, `h` or
`d`. A count is 1 or more, but `controllers.max_open_reviews`, which may be 0. A value that looks
like a token is refused by its path and not printed: a word starting `ghp_`, `github_pat_`,
`glpat-` or `xox<letter>-`. A key ending in `token`, `password`, `secret` or `bearer` takes only
`secret:<name>`. `conductor secret put --name <name>` stores the value in `daemon.secrets`
([operators and credentials](/docs/security/operators-and-credentials)).

## Who reads each key

A key nothing reads yet is accepted, defaulted and shown: `repositories[].activity`, `reports`,
`authority.class_o` and `operator`.

| key | read by |
|---|---|
| `daemon.*` | `conductor daemon serve`; `port` and `timeout` also by `conductor status` |
| `globals.roles` | conductord (host roles), every role lookup |
| `placement`, `remote.*`, `model_token` | conductord, opening the box |
| `session_prefix` | `session start`, `session attach`, guard, sessions collector, `controller start-controller` |
| `sources`, `checkouts.*` | collectors, watch, guard, dashboard, `resource usage`, `config workspace`, `conductor trust` |
| `records`, `state`, `cache` | every store command, guard, watch, dashboard, collectors |
| `roles[]` | `session start`, `spawn`, `controller resume-controller`, `session attach --role`, watch; `read_only_tools` by the guard; `grant` by `conductor broker serve` |
| `controllers.*`, `authority.class_c`, `cadence.cycle`, `cadence.daily` | profiles, `conductor run` |
| `cadence.watch`, `cadence.ci` | watch |
| `thresholds.*` | watch, profiles, `conductor build run`, `conductor gc`, `host reclaim` ([disk](./disk.md)) |
| `retention.snapshots` | store |
| `usage.probe`, `usage.watcher`, `usage.rotate` | usage collector, watch, `conductor status`; `conductor doctor`; the watch's resume of a pane on `Usage limit reached` |
| `catalog`, `secret_syncs` | repositories and secrets collectors, board |
| `gates_policy` | `conductor repository refresh-hooks --apply`, `conductor doctor` |
| `forge.route` | guard, `conductor run`, broker `commit`, `conductor status`, `conductor doctor` |
| `repositories[].no_releases` | `metric compute` |

## What the reference does not say

| key | behaviour |
|---|---|
| `daemon.loopback: false` | `daemon serve --install` writes `/etc/systemd/system/conductord.service`, running as the installing user with its home; refused while the config file lies in no git checkout or `daemon.secrets.source` is `keychain`. A workstation runs `conductor login --daemon <url>`, then `--daemon` or `CONDUCTOR_DAEMON` on `join`, `leave`, `config put` and `instance upgrade` |
| `daemon.tls` | the same app and `/v1/` as loopback, both at once; every request but the app's files and a CORS preflight carries a bearer; any origin may read with one. `identity` is `https://`, or `http://` on `127.0.0.1`, `localhost` or `*.svc.cluster.local`; a bearer resolves through `GET <identity>/v1/access-authority`, audience `conductor`; an operator is named by the principal's email, else its subject |
| `daemon.operators` | a write needs its grant: `answer-decisions` for answers and the Plan page, `change-settings` for settings and `PUT /v1/config`, `upgrade-instances` for an upgrade; else `GrantMissing`. With none written, the Unix user holds every grant on loopback |
| `roles[].settings` | a session started with `claude --bg` passes `--setting-sources user,project,local`, since Claude Code takes the bypass-permissions acceptance only from user or managed settings; the `--settings` file wins on every key it sets. A role's own file carries what its sessions need beyond the guard: `env` with absolute paths, `attribution`, `enabledPlugins`, `extraKnownMarketplaces`, `skipDangerousModePermissionPrompt`, `claudeMdExcludes`, and the shipped `hooks` with the matcher `*` ([sessions](/docs/security/sessions)) |
| `roles[].token_owner` | the controller starts with `--remote-control`; without it the agent leases the Available token leased the fewest times; with no token in the pool every start is unchanged |
| `roles[].grant` | `repositories` is kept and not checked yet; a call outside the grant is refused `NotGranted: <rule>` and recorded |
| `repositories[].no_releases` | `metric compute` prints `K5\|<repository>\|<day>: skipped: ships without releases: <reason>`. A repository that tags `v<major>.<minor>.<patch>` without Release objects needs no rule |
| `forge.route` | under either route the guard denies an assignment of `GH_HOST`, `GH_REPO`, `GITLAB_HOST`, `GL_HOST`, `GITLAB_URI` or `GITLAB_REPO` on a direct write, and allows a `git push` from the session's own checkout or trees. Gates hooks are reported stale only under `gates` or with a `gates_policy`. A host without `b10x-gates` writes `forge: {route: direct}` |
| `secret_syncs` | the board shows `secret unread: <name>` when a read failed. Where `rules.md` grants no run of `command`, the profile hands the sync over as class H with the output of `dry_run` |
| `catalog` | each repository carries `in_catalog`, true or false; absent for all when no catalog or no checkout of it |
| `usage.probe` | one JSON object per snapshot, as below; an id, health or window name holds letters, digits, `.`, `_`, `:` and `-`; any other shape records one member failure `usage`. The watch and `conductor status` print `usage: pool runs out ~<hour>, next reset <minute>` from the newest two snapshots |

```json
{"accounts": [
  {"id": "account-a", "active": true, "health": "ready",
   "usage": {"windows": [{"name": "seven_day", "used_percent": 40.0,
                          "resets_at": "2026-10-16T11:00:00Z"}]}}
]}
```
