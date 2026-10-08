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
- which harness, model, agent and settings file each role runs with (`roles`);
- how many controllers and sub-agents work at once, its cadence, thresholds, retention, reports
  and authority; the operator's name; and an optional catalog of repositories.

An instance may leave out every key but `name`, `sources` and `checkouts`. The
[config reference](/docs/reference/config) lists every key with its type and default.

## Which file, which instance

| what | order |
|---|---|
| file | `--config`, else `$CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml` |
| instance | `--instance` (on `config validate` and `config show`), else `$CONDUCTOR_INSTANCE`, else the file's `default`, else its only instance |

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
