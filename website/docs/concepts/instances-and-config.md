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
- how many controllers and sub-agents work at once, its cadence, thresholds, retention, reports
  and authority; the operator's name; and an optional catalog of repositories.

An instance may leave out every key but `name`, `sources` and `checkouts`. The
[config reference](/docs/reference/config) lists every key with its type and default.

## Which file, which instance

| what | order |
|---|---|
| file | `--config`, else `$CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml` |
| instance | `--instance` (on `config validate`, `config show`, `config workspace`, `init` and `trust`), else `$CONDUCTOR_INSTANCE`, else the file's `default`, else its only instance |

A file named by `--config` or `$CONDUCTOR_CONFIG` must exist. Without the default file, a
built-in instance applies, with placeholder values; write a config file before you run conductor
for real.

Every command resolves the instance before it runs, and a file that does not validate stops the
command with its problems. `conductor config validate` names each problem by its YAML path and
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
| `settings` | the settings file the session starts with, through `--settings`; the shipped ones wire the guard |
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

- no two instances share a prefix, and at most one instance has none (it keeps the bare names
  `conductor`, `conductor-dev` and `<repo>`);
- no prefix starts with another prefix followed by `-`, and no prefix followed by `-` begins a
  bare name another instance keeps, such as `conductor-dev`;
- a prefix is letters, digits and `-`, starting and ending with a letter or digit.

A role's `session_name` is derived from the prefix; `config show` prints it, and a file may write
only the derived value. The guard lets a conductor message only the sessions whose name carries its
own instance's prefix (any session, when its instance has none), and the sessions collector keeps another instance's session to its harness
and activity.

## The workspace file

The blockers collector reads the instance's repositories as the members of a workspace file,
`<records>/.engineering/workspace.yaml` (`aep.workspace/1`), and exports each member's `origin/main`;
the specifications collector reads a `github` source's repositories in those exports.
`conductor config workspace` writes it from the config, with no network read:

- for a `github` source, each git checkout directly under `checkouts.root` that has an `origin`
  remote;
- for a `local` source, each such checkout directly under its path, but those its `exclude` names;
- for a `gitlab` source, none;
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
| `github: <owner>` | the owner's repositories on GitHub, through `gh` |
| `local: <path>` | the repositories under a directory, as they are |
| `gitlab: <group>` | a GitLab group; the binary does not read it, and the session records its rows |

A `local` or `gitlab` source may `exclude` paths under its root. An instance's `catalog` names a
repository under the checkouts root and a directory in it; the repositories collector then records
whether each repository is in that catalog.

## No secrets

The file never holds a secret. A value that looks like a token is refused by its path and never
printed.
