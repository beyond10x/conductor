---
title: Configure an instance
sidebar_position: 1
description: Write a config file for one organization, validate it, read a refusal, and see the effective values the sessions read.
lede: A config file is checked before any command runs, so write it, validate it and read its effective values before you start a session.
source: docs/config.md; conductor.example.yaml; crates/conductor/src/config.rs, crates/conductor/tests/config.rs; conductor config validate --help, conductor config show --help
---

## 1. Write the file

Write the file `$CONDUCTOR_CONFIG` names, here `~/example-org/conductor.yaml`; `--config` names one
for a single command, and without either the default path applies
([instances and config](../concepts/instances-and-config.md#which-file-which-instance)). This instance reads a GitHub owner's repositories, checked out under `~/example-org`, with a
checkout of the conductor repository at `~/example-org/conductor` for the profiles and settings:

```yaml
version: conductor.config/1
default: personal
instances:
  - name: personal
    sources:
      - github: example-org
    checkouts:
      root: ~/example-org
      trees: ~/.local/state/worktree/trees/example-org
    records: ~/example-org/records
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
        agent: repo-controller
        settings: ~/example-org/conductor/.claude/controller-settings.json
        profile: ~/example-org/conductor/.agents/repo-controller.md
```

| key | why |
|---|---|
| `sources` | where the repositories come from: `github: <owner>`, `local: <path>` or `gitlab: <group>` |
| `checkouts.root`, `checkouts.trees` | where their checkouts and managed worktrees are; the guard and the collectors read them |
| `records` | the directory conductor runs in and keeps its records in; the guard gives conductor its rules only there. Default `~/.b10x/conductor/<name>/records` |
| `roles` | the harness and model of each session; a role's `profile` is the file its session starts with through `--append-system-prompt-file`, and its `settings` the settings file that wires the guard. A list given replaces the whole default |

Why a controller role that names an `agent` must name a `profile` too: every session starts with
`--setting-sources project,local`, which loads no user-level agent from `~/.claude/agents/`, and a
repository's own checkout does not carry conductor's adapter. So a controller always starts from
its profile, never from `--agent`, and `conductor config validate` refuses a controller role with
an `agent` and no `profile`. With the `profile` line above removed:

```console
$ conductor config validate
conductor: config validate: ~/example-org/conductor.yaml: instances[0].roles[2].profile: is absent beside an agent: a controller starts from its profile, the file its session starts with through --append-system-prompt-file, never from an agent, because --setting-sources project,local loads no user-level agent
```

The repository's `conductor.example.yaml` has two complete instances, one with a catalog and one
with a `local` source. Every key is in the [config reference](/docs/reference/config).

## 2. Validate it

```console
$ conductor config validate
~/example-org/conductor.yaml: valid, 1 instance(s): personal
```

A problem is named by its YAML path, and the command exits 1. This `broken.yaml` writes a
duration without its unit and a size in the wrong unit:

```yaml
version: conductor.config/1
instances:
  - name: demo
    sources:
      - local: ~/work
    checkouts:
      root: ~/work
      trees: ~/.local/state/worktree/trees/work
    cadence:
      cycle: 90
    thresholds:
      disk_low: 100GB
```

```console
$ conductor config validate --config broken.yaml
conductor: config validate: broken.yaml: instances[0].cadence.cycle: 90 is not a duration: write a whole number above 0 with s, m, h or d, such as 180s or 2h
conductor: config validate: broken.yaml: instances[0].thresholds.disk_low: "100GB" is not a size: write a whole number of GiB with G, such as 15G
$ echo $?
1
```

Every other command resolves the instance first and stops on the same problems.

## 3. Read the effective values

`config show` prints the instance after defaults, with every path absolute. The sessions read a
value written `section.key` from its JSON form:

```console
$ conductor config show --format json | jq -c '.instances[0] | {records, state, cache}'
{"records":"~/example-org/records","state":"~/.b10x/conductor/personal/state","cache":"~/.cache/b10x/conductor/personal"}
$ conductor config show --format json | jq -c ".instances[0].roles[2]"
{"role":"controller","harness":"claude","model":"opus","agent":"repo-controller","settings":"~/example-org/conductor/.claude/controller-settings.json","profile":"~/example-org/conductor/.agents/repo-controller.md","session_name":null,"read_only_tools":[]}
```

(The CLI prints the home directory in full; it is written `~` here.) `session_name` is derived,
never chosen: `<session_prefix>-<role>` for conductor and conductor-dev, and `null` for the
controller role, whose sessions are named per repository. `read_only_tools` is the list of extra
tools the guard allows this role's sessions beyond its built-in read-only list; empty here.
`--format yaml` prints the same instance as a config file.

## 4. Write the instance's rules

`rules.md` in the instance's `records` directory holds the organization's own rules, one section
per role, each rule with its source. Every session reads it after its profile. The repository ships
none; [`conductor init`](./initialise-the-records.md) writes an empty one with a section per role.
What belongs there is what holds for this organization only, such as where the operator's standing
grant for class-C decisions is written ([decision classes](../concepts/decisions-and-messages.md)).

Next: [initialise the records](./initialise-the-records.md), then
[start conductor](./start-conductor.md).
