---
title: Instances and config
sidebar_position: 5
description: One config file names conductor's instances; how a command finds the file and the instance, what the binary reads and what the sessions read.
lede: An instance is one organization, written as one entry of one config file, so one generic profile serves every organization.
source: docs/design/conductor.md section 9; docs/config.md; spec/domains/config.yaml; crates/conductor/src/config.rs, crates/conductor/tests/config.rs; conductor config validate --help, conductor config show --help
---

## One file, several instances

One file, `conductor.config/1`, names conductor's instances. An instance is one organization:

- where its repositories come from (`sources`) and where their checkouts and managed worktrees are
  (`checkouts`);
- where conductor keeps its records, its state and its cache (`records`, `state`, `cache`);
- which harness, model, profile, settings file and extra read-only tools each role runs with
  (`roles`), and the prefix of its session names (`session_prefix`);
- the port its dashboard serves on 127.0.0.1 (`dashboard_port`);
- how many controllers and sub-agents work at once, its cadence, thresholds, retention, reports
  and authority; the operator's name; and an optional catalog of repositories.

An instance may leave out every key but `name`, `sources` and `checkouts`. The
[config reference](/docs/reference/config) lists every key with its type and default.

## Which file, which instance

| what | order |
|---|---|
| file | `--config`, else `$CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml` |
| instance | `--instance` (on `config validate`, `config show`, `config workspace`, `init` and `trust`), else `$CONDUCTOR_INSTANCE`, else the instance whose `records`, `state`, `checkouts.root` or `checkouts.trees` holds the working directory (the deepest, when one lies inside another's), else the file's `default`, else its only instance |

A file named by `--config` or `$CONDUCTOR_CONFIG` must exist. Without the default file, a
built-in instance applies, with placeholder values; write a config file before you run conductor
for real.

Every command resolves the instance before it runs, and a file that does not validate stops the
command with its problems. A command run without `$CONDUCTOR_INSTANCE` inside another instance's
tree never falls back to the `default`: it acts for that instance, and is refused
(`AmbiguousInstance`) where two instances' trees meet at the same depth. The guard denies every
call while the file does not load (`ConfigUnloaded`), naming the problem, rather than deciding by
built-in paths.

A command that writes the instance's records (the store, the decision, dispatch and message logs,
the board and the snapshot's rollups) refuses a `--state-dir` other than the instance's `state`
(`StateDirNotInstance`): its records would otherwise be written beside another store.

What the instances of one file share lives in the host directory, `host/` beside the config file:
the host ledger of disk reservations, its locks, the last reclaim run and the marker of a fall of
free disk. An instance whose `state` held them before has them moved there once, at its first
host command, and `host.migrated` in its `state` records the move. `conductor config validate` names each problem by its YAML path and
exits 1. `conductor config show` prints the effective instance after defaults, every path absolute,
as `text` (one `path: value` line each), `json` or `yaml`.

## Who reads a field

The binary reads some fields; the sessions and the tasks read the rest. A profile writes a config
value as `section.key`, such as `thresholds.gate_stop`, meaning that field of
`conductor config show --format json`. So one generic profile serves every instance, and a value
that differs per organization is a config key or a rule in the instance's `rules.md`, never an edit
to the profile.

A field that nothing reads yet is still accepted, defaulted and shown. Today that is
`repositories`, `reports`, `authority.class_o` and `operator`.

## Roles: profile, settings, read-only tools

Each entry of `roles` names a role (`conductor`, `conductor-dev`, `controller`), its `harness` and
`model`, and optionally:

| key | what |
|---|---|
| `profile` | the profile file the session starts with, through `--append-system-prompt-file` |
| `settings` | the settings file the session starts with, through `--settings`; the shipped ones wire the guard. Under a config file the session's settings pin the role's instance: `"env": {"CONDUCTOR_INSTANCE": "<name>"}`, and every hook command that runs `conductor` starts `CONDUCTOR_INSTANCE=<name> conductor …`. The harness does not pass the environment of `conductor session start` or `spawn` on to the session it forks, so only the settings select the instance its hooks act for. A settings file that pins no instance, as the shipped ones do, starts pinned: `session start` and `spawn` write its content with `env.CONDUCTOR_INSTANCE` set and each such hook command prefixed to `<state>/sessions/<session name>.settings.json` and start the session with that copy, and `config validate` reports it as a note. `config validate` names a role whose settings file pins another instance, and `session start` and `spawn` refuse it (`InstanceNotPinned`). A `conductor` or `conductor-dev` role without `settings` starts with its default (`.claude/conductor-settings.json` of the working directory for `conductor`) written pinned to the instance as `<state>/sessions/<session name>.settings.json` |
| `agent` | a harness agent, used only by a role without a `profile`; a controller role that names an `agent` must name a `profile` too, or `config validate` refuses it ([roles](./roles.md#profiles-and-rules)) |
| `read_only_tools` | tool names, as the harness names them (such as `mcp__server__tool`), that the guard allows this role's sessions beyond its built-in read-only list; empty by default |

The guard denies every tool it has no rule for and that is on neither list, MCP tools included.
Add a tool to `read_only_tools` only when it cannot write anything: the guard does not inspect what
an allowed tool does.

## Several instances and session prefixes

One file may name several instances, one per organization. `default` names the one a command uses
when neither `--instance` nor `$CONDUCTOR_INSTANCE` names one.

Session names are one namespace for the whole machine, so each instance's sessions carry its
`session_prefix`: `<session_prefix>-conductor`, `<session_prefix>-conductor-dev` and
`<session_prefix>-<repo>` for a controller. `config validate` holds the prefixes apart:

- every instance names one; `conductor config migrate` writes one into an instance written
  before it was required, which until then keeps the bare names `conductor`, `conductor-dev` and
  `<repo>`, for one release;
- no two instances on the host share a prefix, a dashboard port, a state or a records directory,
  in one file or across files: `config validate` keeps every valid file in the host registry,
  `~/.b10x/conductor/host/instances.json`, and refuses an instance that holds what another
  registered file's instance holds, naming both;
- no prefix starts with another prefix followed by `-`, and no prefix followed by `-` begins a
  bare name another instance keeps, such as `conductor-dev`;
- a prefix is letters, digits and `-`, starting and ending with a letter or digit.

A role's `session_name` is derived from the prefix; `config show` prints it, and a file may write
only the derived value. The guard lets a conductor message only the sessions whose name carries its
own instance's prefix (any session, when its instance has none), and the sessions collector keeps another instance's session to its harness
and activity.

The tasks act on the active instance only. `task sessions`, `task conductor:attach`,
`task dev:attach` and `task conductor:restart` find its sessions by their full names, and refuse a
name that more than one live background session holds, since no task can tell which one is this
instance's. The dashboard takes the same care with its port and its service:

- each instance's dashboard serves on its own `dashboard_port`, by default 7313 plus the
  instance's position in the file (7313 for the first), and `config validate` refuses two
  instances that share one; `conductor dashboard serve` without `--port` listens on it;
- where `systemctl` is on `PATH`, `task dashboard` runs it as the user service
  `<session_prefix>-conductor-dashboard`, or `conductor-dashboard` without a prefix;
- elsewhere it runs in the background, with its pid in `dashboard.pid` of the instance's `state`
  directory.

## The workspace file

The blockers collector reads the instance's repositories as the members of a workspace file,
`<records>/.engineering/workspace.yaml` (`aep.workspace/1`), and exports each member's `origin/main`;
the specifications collector reads a `github` or `gitlab` source's repositories in those exports.
`conductor config workspace` writes it from the config, with no network read:

- for a `github` source, each git checkout directly under `checkouts.root` that has an `origin`
  remote;
- for a `local` source, each such checkout directly under its path, but those its `exclude` names;
- for a `gitlab` source, each git checkout under `checkouts.root` whose `origin` remote is on the
  source's `host` and under its group, but those its `exclude` names;
- and the records directory itself.

Each member's `source` is relative to `<records>/.engineering/`. A checkout without an `origin`
remote is no member, because the collectors read every member at `origin/main`.
`conductor config workspace --check` writes nothing and exits 1, naming each member that differs,
when the file no longer matches the config; run it after a repository is cloned or removed.
`--instance` selects the instance, as for `config show`. `conductor init` writes the first version
([initialise the records](../guides/initialise-the-records.md)).

## Repositories and sources

A repository is the directory holding `.git` at depth 1 or 2 under a root (`<root>/<repo>` or
`<root>/<group>/<repo>`), named by its path under the root. A source is one of:

| source | lists |
|---|---|
| `github: <owner>` | the owner's repositories on GitHub, through `gh`; `gh` is needed only with this source |
| `local: <path>` | the repositories under a directory, as they are |
| `gitlab: <group>` | the projects of a GitLab group and its subgroups on `host` (default `gitlab.com`, for example `host: gitlab.example.com`), through `glab api --hostname <host>`: repositories and specifications, merge requests as pull requests, pipelines as workflow runs, releases; `glab` is needed only with this source |

A `local` or `gitlab` source may `exclude` paths under its root. An instance's `catalog` names a
repository under the checkouts root and a directory in it; the repositories collector then records
whether each repository is in that catalog.

## No secrets

The file never holds a secret. A value that looks like a token is refused by its path and never
printed.
