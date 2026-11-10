---
title: Add a repository rule
sidebar_position: 4
description: Write a rule for one repository in the config, and set its activity today with a mark that conductor's commands read.
lede: A repository's activity decides whether conductor processes its issues and pull requests. A mark sets it today, and a config rule records it for later.
source: docs/config.md, Instance; crates/conductor/src/repository.rs, crates/conductor/src/config.rs, crates/conductor/src/metric.rs, crates/conductor/src/cli.rs (LEAVES), crates/conductor/tests/repository_marks.rs; conductor repository --help
---

Conductor processes issues and pull requests only for **Active** repositories. Without a mark,
the newest complete snapshot decides: a repository that is not archived and has a real commit in
the last 7 days is Active. That is wrong when work sits somewhere the snapshot does not count, or
when a busy repository should be left alone.

The instance here has a `local: ~/work` source with two repositories: `alpha` is Active and
`beta` Inactive.

## 1. Write the rule in the config

Add a `repositories` list to the instance. Each rule matches a repository and sets its activity:

```yaml
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/store/conductor.sqlite
instances:
  - name: demo
    placement: linux
    session_prefix: demo
    sources:
      - local: ~/work
    checkouts:
      root: ~/work
      trees: ~/.local/state/worktree/trees/work
    repositories:
      - {match: beta, activity: active}
```

```console
$ conductor config validate
~/.b10x/conductor/conductor.yaml: valid, 1 instance(s): demo
$ conductor config show --format json | jq '.instances[0].repositories'
[
  {
    "match": "beta",
    "activity": "active",
    "no_releases": null
  }
]
```

A rule can also mark a repository that ships without releases, with the reason:
`{match: docs-site, no_releases: "published on merge"}`. `metric compute` leaves such a
repository out of every K5 reading and lists it as skipped once. A repository that releases by
semver tag (`v1.2.3`) needs no rule: the collectors record such a tag as its release.

:::caution[Planned]

The binary accepts, checks and shows `repositories`, but only `no_releases` is read. Activity
does not follow `activity` yet:

```console
$ conductor repository activity
repository  activity  decided_by  marked_by  reason
alpha       Active    Snapshot    null       null
beta        Inactive  Snapshot    null       null
```

Until it is read, set the activity with a mark (step 2).

:::

## 2. Mark the repository

A mark is a record in conductor's store, set by the operator or by conductor, always with a
reason. It wins over the snapshot, and no snapshot overwrites it. `mark-repository` sets the first
mark:

```console
$ conductor repository mark-repository --repository beta --activity Active --marked-by Operator --reason "release work sits on a branch"
$ conductor repository activity
repository  activity  decided_by  marked_by  reason
alpha       Active    Snapshot    null       null
beta        Active    Mark        Operator   release work sits on a branch
$ conductor repository repository-marks
repository  state   marked_by  session  reason                         actor
beta        Active  Operator   null     release work sits on a branch  human:example-user
```

`decided_by` is `Mark` or `Snapshot`. `--session` records the session that set the mark.
`actor` is who ran the command: `human:<operator>` for you, `agent:<instance>/<role>/<name>` for
a session.

## 3. Change it later

`activate-repository` and `deactivate-repository` move an existing mark:

```console
$ conductor repository deactivate-repository --repository beta --marked-by Operator --reason "release done"
$ conductor repository activity
repository  activity  decided_by  marked_by  reason
alpha       Active    Snapshot    null       null
beta        Inactive  Mark        Operator   release done
```

Every command here writes to the store, so the guard denies it to a controller's session.
Conductor and the operator set marks.
