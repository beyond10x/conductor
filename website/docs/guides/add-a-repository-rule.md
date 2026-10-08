---
title: Add a repository rule
sidebar_position: 3
description: Write an activity rule for one repository in the config, and set its activity today with a mark that conductor's commands read.
lede: A repository's activity decides whether conductor processes its issues and pull requests; a mark sets it today, and a config rule records it for later.
source: docs/config.md, Instance; docs/design/conductor.md section 10, Repository activity; crates/conductor/src/repository.rs, crates/conductor/src/config.rs, crates/conductor/tests/repository_marks.rs; conductor repository --help
---

Conductor processes issues and pull requests only for **Active** repositories. Without anything
else, the newest complete snapshot decides: a repository with a real commit in 7 days is Active
([the loop](../concepts/the-loop.md#active-repositories)). Sometimes that is wrong: work sits
somewhere the snapshot does not count, or a busy repository should be left alone.

This guide continues the [getting-started](../getting-started.md) instance, where `alpha` is Active
and `beta` Inactive.

## 1. Write the rule in the config

Add a `repositories` list to the instance. Each rule names a repository and the activity it sets:

```yaml
version: conductor.config/1
instances:
  - name: demo
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
    "activity": "active"
  }
]
```

:::caution[Planned]

The binary accepts, checks and shows `repositories`, but nothing reads it yet: activity does not
follow it.

```console
$ conductor repository activity
repository  activity  decided_by  marked_by  reason
alpha       Active    Snapshot    null       null
beta        Inactive  Snapshot    null       null
```

Until it is read, set the activity with a mark (step 2).

:::

## 2. Mark the repository

A mark is a record in conductor's store, set by the operator or by conductor, always with a reason.
It wins over the snapshot and is never overwritten by one. `mark-repository` sets the first mark:

```console
$ conductor repository mark-repository --repository beta --activity Active --marked-by Operator --reason "release work sits on a branch"
$ conductor repository activity
repository  activity  decided_by  marked_by  reason
alpha       Active    Snapshot    null       null
beta        Active    Mark        Operator   release work sits on a branch
$ conductor repository repository-marks
repository  state   marked_by  reason
beta        Active  Operator   release work sits on a branch
```

`decided_by` says which one decided: `Mark` or `Snapshot`.

## 3. Change it later

`activate-repository` and `deactivate-repository` move an existing mark:

```console
$ conductor repository deactivate-repository --repository beta --marked-by Operator --reason "release done"
$ conductor repository activity
repository  activity  decided_by  marked_by  reason
alpha       Active    Snapshot    null       null
beta        Inactive  Mark        Operator   release done
```

Every command here writes to the store, so the guard denies it to a controller; conductor and the
operator set marks.
