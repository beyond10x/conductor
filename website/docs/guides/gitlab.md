---
title: Run conductor on GitLab
sidebar_position: 2
description: Point an instance at a group on a self-hosted GitLab, log glab in to its host, check the programs, write the workspace file, and read what conductor collects from GitLab.
lede: A gitlab source names a group and the host it is on. Conductor reads that forge through glab, so a team with no repository on GitHub needs no gh.
source: crates/conductor/src/config.rs, crates/conductor/tests/config.rs; crates/conductor/src/doctor.rs; crates/conductor/src/collect/gitlab.rs, collect/mod.rs; crates/conductor/src/watch.rs; crates/conductor/src/guard/rules.rs; spec/domains/config.yaml
---

This guide sets up an instance whose repositories are the projects of one group on a self-hosted
GitLab, here `example-group` on `gitlab.example.com`. Read
[configure an instance](./configure-an-instance.md) first: what it says about the file, checkouts,
records and roles holds here too.

## 1. Name the group and its host

```yaml
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/store/conductor.sqlite
default: platform
instances:
  - name: platform
    placement: linux
    session_prefix: platform
    sources:
      - gitlab: example-group
        host: gitlab.example.com
    checkouts:
      root: ~/example-group
      trees: ~/.local/state/worktree/trees/example-group
    records: ~/records/example-group
    forge:
      route: direct
```

| key | what it sets |
|---|---|
| `gitlab` | the group whose projects, and its subgroups' projects, are the instance's repositories |
| `host` | the GitLab host name, without a scheme or a path. A port may follow as `:<port>`. Absent, it is `gitlab.com` |
| `forge.route` | `direct`: a controller writes its own merge requests with `glab`. The bot route of the default, `gates`, writes to GitHub only, so `conductor doctor` is not ready for a `gitlab` source on it |

`conductor config validate` refuses `host` beside a `github` or `local` source, and a host written
with a scheme or a path:

```console
$ conductor config validate
~/example-group/conductor.yaml: valid, 1 instance(s): platform
$ conductor config show --format json | jq -c '.instances[0].sources'
[{"gitlab":"example-group","host":"gitlab.example.com"}]
```

A `gitlab` source may also carry `exclude`, as a `local` source does. An entry names a project
that is checked out by its checkout's path under `checkouts.root` (`checks`) or by the project's
path under the group (`example-checks`); either leaves it out.

A source may instead name the projects it stands for with `include`: each entry is a project's
path under the group (`alpha`), or a subgroup's (`sub`), which takes every project under it. Every
other project of the group is left out as an excluded one is, a project the group gains later
included, until an entry names it:

```yaml
sources:
  - gitlab: example-group
    host: gitlab.example.com
    include: [alpha, sub]
```

`include` and `exclude` do not go on one source, and an empty `include` does not validate. An
entry no project of the group answers to does not stop a snapshot: it is listed under
`conductor snapshot member-failures`, the entry as its member. Every key is in the
[config reference](../reference/config.md).

## 2. Log glab in to the host

Conductor reads GitLab through `glab`, GitLab's own CLI, on the host the source names. Log it in
once, as the user conductor runs as:

```console
$ glab auth login --hostname gitlab.example.com
$ glab auth status --hostname gitlab.example.com
```

`glab` keeps its token in its own config. Conductor does not read or write it.

## 3. Check the programs

`conductor doctor` asks for a forge's CLI only when an instance of the file has a source on that
forge: `glab` for a `gitlab` source, `gh` for a `github` source. For this instance the forge lines
read:

```console
$ conductor doctor
gh: not needed
glab: present, glab 1.73.0
glab: logged in to gitlab.example.com as example-user
```

Without `glab` on `PATH` the line reads `glab: missing`, the last line `not ready: glab (missing)`,
and the command exits 1. A snapshot of this instance is refused at its start for the same reason,
with a message that names `glab`. The other lines of doctor's output are in
[getting started](../getting-started.md), and each `not ready` reason in
[troubleshoot](./troubleshoot.md).

## 4. Write the workspace file

```console
$ conductor config workspace
```

For a `gitlab` source, the workspace file has one member per git checkout under `checkouts.root`
whose `origin` remote is on the source's host and under its group, except those its `exclude`
lists, or with `include` those it does not list. It reads the checkouts only, not the forge. The
repository's name is the checkout's path under `checkouts.root`, not the project's name: a
checkout `checks` of the project
`example-checks` is `checks` in the workspace file, the snapshots, `conductor repository activity`,
its charter `charters/checks.md` and its dispatches. A checkout of the same group path on another
host, or of another group on this host, is not a member. The records directory is a member too
([initialise the records](./initialise-the-records.md#3-keep-the-workspace-file-in-step)).

## What conductor reads from GitLab

Each snapshot reads the group with `glab api --hostname <host>` and records:

| record | what |
|---|---|
| repositories | one row per checkout under `checkouts.root` of a project of the group and its subgroups, named by the checkout's path under the root, and one row per project without a checkout, named by its path under the group; archived projects marked archived. A project with no commit yet is left out |
| issues | the number of open issues of each repository |
| releases | each repository's newest release, its tag and date, and the commits on the default branch after it. Each release published within the collector's window. A `v<major>.<minor>.<patch>` tag counts as a release |
| merge requests | each open merge request, failing when its head pipeline failed, and each merge request merged within the collector's window, once per repository: a project checked out under two directory names has each of its merge requests, releases and pipelines recorded twice, once under each name |
| pipelines | the newest pipeline of each repository's default branch, its status read as the conclusions a GitHub workflow run has |
| specifications | for each repository, whether its checkout holds a specification |

The board, the dashboard and `conductor repository shipped` read these rows as they read a GitHub
instance's. For the active repositories of the newest complete snapshot, `conductor watch run`
prints `CI red on main: <repo> / pipeline (run <id>)` when a project's default-branch pipeline
turns from success to failed, and `CI green again on main: <repo> / pipeline` when it turns back.

## glab under the guard

The guard reads a `glab` call as it reads a `gh` call. A read is allowed, and any other call is a
forge write, denied in every guarded session unless `forge.route: direct` carries it. A write
to a forge credential (`glab variable`, `auth`, `ssh-key`, `gpg-key`, `deploy-key`, `token`,
`config` or `securefile`) is class H, for the operator to run. Which `glab` calls read, and the writes `forge.route: direct` lets a controller
make, are in [the guard](../security/the-guard.md).

Next: [initialise the records](./initialise-the-records.md).
