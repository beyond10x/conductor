---
title: Run conductor on GitLab
sidebar_position: 7
description: Point an instance at a group on a self-hosted GitLab, log glab in to its host, check the programs, write the workspace file, and read what conductor collects and what the guard holds.
lede: A gitlab source names a group and the host it is on; conductor reads that forge through glab, so a team with no repository on GitHub needs no gh.
source: crates/conductor/src/config.rs, crates/conductor/tests/config.rs; crates/conductor/src/doctor.rs, crates/conductor/tests/doctor.rs; crates/conductor/src/collect/; crates/conductor/src/watch.rs; crates/conductor/src/guard/rules.rs, crates/conductor/tests/guard_rules.rs; spec/domains/config.yaml
---

This guide sets up an instance whose repositories are the projects of one group on a self-hosted
GitLab, here `example-group` on `gitlab.example.com`. Read
[configure an instance](./configure-an-instance.md) first; everything it says about checkouts,
records and roles holds here too.

## 1. Name the group and its host

A `gitlab` source names a group and the host it is on:

```yaml
version: conductor.config/1
default: platform
instances:
  - name: platform
    sources:
      - gitlab: example-group
        host: gitlab.example.com
    checkouts:
      root: ~/example-group
      trees: ~/.local/state/worktree/trees/example-group
    records: ~/example-group/records
```

| key | why |
|---|---|
| `gitlab` | the group whose projects, and its subgroups' projects, are the instance's repositories |
| `host` | the GitLab host name, without a scheme or a path; a port may follow as `:<port>`. Absent, it is `gitlab.com` |

`host` belongs to a `gitlab` source only; `conductor config validate` refuses it beside `github` or
`local`, and refuses a host written with a scheme or a path:

```console
$ conductor config validate
~/example-group/conductor.yaml: valid, 1 instance(s): platform
$ conductor config show --format json | jq -c '.instances[0].sources'
[{"gitlab":"example-group","host":"gitlab.example.com"}]
```

A `gitlab` source may also carry `exclude`, as a `local` source does. Every key is in the
[config reference](/docs/reference/config).

## 2. Log glab in to the host

Conductor reads GitLab through `glab`, GitLab's own CLI, on the host the source names. Log it in
once, as the user conductor's sessions run as:

```console
$ glab auth login --hostname gitlab.example.com
$ glab auth status --hostname gitlab.example.com
```

`glab` keeps its token in its own config; conductor never reads or writes it.

## 3. Check the programs

`conductor doctor` asks for a forge's CLI only when the instance has a source on that forge: `glab`
for a `gitlab` source, `gh` for a `github` source. For this instance `gh` is not needed, and
`glab` is:

```console
$ conductor doctor
git: present, git version 2.55.0
gh: not needed
glab: present, glab 1.73.0
claude: present, 2.1.296 (Claude Code)
jq: present, jq-1.8.2
task: present, 3.53.1+ff3372fc
aep: present, aep 0.71.2
worktree: present, b10x-worktree-cli 0.18.0
ess: present, ess 0.59.0
pins: none
aep: 0.71.2 meets the build's pin 0.71
worktree: 0.18.0 meets the build's pin 0.18
ess: 0.59.0 meets the build's pin 0.59
ready
```

Without `glab` on `PATH` the `glab` line reads `glab: missing`, the last line
`not ready: glab (missing)`, and the command exits 1. A snapshot of this instance is refused up
front for the same reason, naming `glab`, rather than failing in a collector
([troubleshoot](./troubleshoot.md)).

## 4. Write the workspace file

```console
$ conductor config workspace
```

For a `gitlab` source, the workspace file has one member per git checkout under `checkouts.root`
whose `origin` remote is on the source's host and under its group, but those its `exclude` names.
It reads the checkouts only, not the forge: a checkout of the same group path on another host, or
of another group on this host, is not a member. `conductor config workspace --check` writes
nothing and names each member that differs from the file.

## What conductor reads from GitLab

Each snapshot reads the group with `glab api --hostname <host>` and records:

- **repositories**: one row per project of the group and its subgroups, archived projects marked
  archived, with whether a checkout of it is under `checkouts.root`;
- **issues**: the number of open issues of each project;
- **releases**: each project's newest release, its tag and date, and the commits on the default
  branch after it; and each release published within the collector's window;
- **merge requests**: each open merge request, failing when its head pipeline failed, and each
  merge request merged within the collector's window;
- **pipelines**: the newest pipeline of each project's default branch, its status read as the
  same conclusions a GitHub workflow run has;
- **specifications**: for each project, whether its checkout holds a specification.

The board, the dashboard and `conductor repository shipped` read these rows as they read a GitHub
instance's ([the loop](../concepts/the-loop.md)).

The watch reports default-branch pipelines: for the active repositories of the newest complete
snapshot, `conductor watch run` prints the red line when a project's default-branch pipeline turns
from success to failed, and the green line when it turns back.

## The guard holds glab writes like gh writes

The guard reads a `glab` call as it reads a `gh` call: a read is allowed, and anything else is a
forge write, denied in every guarded session as the same `gh` write would be. A `glab` read is
`mr view|list|diff`, `issue view|list`, `release view|list|download`, `repo view|list`,
`ci view|list|status|trace|get`, `api` with GET and no request fields, `auth status`, any call
with `--help`, and `--version`. So `glab mr merge` is denied where `gh pr merge` is, and
`glab mr view` allowed where `gh pr view` is. Writes go through the repository's own route.

A write in `glab variable`, `auth`, `ssh-key`, `gpg-key`, `deploy-key`, `token`, `config` or `securefile` is a forge
credential, class H, like a `gh secret` or `gh auth` write: it needs the operator's hands
([the guard](../security/the-guard.md)).

Next: [initialise the records](./initialise-the-records.md), then
[start conductor](./start-conductor.md).
