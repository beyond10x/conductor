---
title: Initialise the records
sidebar_position: 2
description: Write the instance's records directory with conductor init, fill in rules.md, keep the workspace file in step with the config, and take the first snapshot.
lede: Conductor starts in the instance's records directory and reads its hand-over, rules and goals from there, so the directory exists, is a git repository and has an origin before the first session.
source: crates/conductor/src/init.rs, crates/conductor/tests/init.rs; crates/conductor/src/config.rs, config workspace; crates/conductor/src/collect/blockers.rs; conductor init --help, conductor config workspace --help
---

This guide continues the instance of [configure an instance](./configure-an-instance.md): a
`github: example-org` source checked out under `~/example-org`, with `records:
~/example-org/records` and `$CONDUCTOR_CONFIG` pointing at its config file. The CLI prints
absolute paths; the output here writes the home directory as `~`.

## 1. Write the skeleton

```console
$ conductor init
wrote: ~/example-org/records/README.md
wrote: ~/example-org/records/NORTHSTAR.md
wrote: ~/example-org/records/rules.md
wrote: ~/example-org/records/docs/handoff/2026-10-12.md
wrote: ~/example-org/records/.engineering/workspace.yaml
wrote: ~/example-org/records/decisions
wrote: ~/example-org/records/dispatches
wrote: ~/example-org/records/charters
committed: README.md, NORTHSTAR.md, rules.md, docs/handoff/2026-10-12.md, .engineering/workspace.yaml
```

`conductor init` creates the instance's `records` directory, makes it a git repository, writes the
files below and commits them, and only them. `--instance` selects another instance of the file.

| path | holds |
|---|---|
| `README.md` | what the directory is and who writes each file |
| `NORTHSTAR.md` | what the organization works towards; a placeholder until the operator writes the goals |
| `rules.md` | the organization's rules, with an empty section per role: `Conductor`, `Repository controller`, `conductor-dev` |
| `docs/handoff/<date>.md` | the first hand-over: its first task is to fill in `rules.md` and `NORTHSTAR.md` with the operator, run `conductor doctor` and one cycle |
| `.engineering/workspace.yaml` | the workspace file, as `conductor config workspace` writes it (step 3) |
| `decisions/`, `dispatches/`, `charters/` | empty directories for the session's decision log, briefs and charters |

It overwrites nothing. When any of those files is there, it writes nothing, makes no repository and
exits 1:

```console
$ conductor init
conductor: init: ~/example-org/records/README.md, ~/example-org/records/NORTHSTAR.md, ~/example-org/records/rules.md, ~/example-org/records/docs/handoff/2026-10-12.md, ~/example-org/records/.engineering/workspace.yaml are there; init overwrites nothing and wrote nothing
```

It also refuses without a config file: the records directory is the one an instance of a file
names. The texts it writes name no organization.

## 2. Write the rules

Fill in `rules.md` with the organization's own rules, one section per role, each rule with its
source: a decision, a message or a document. Every session reads it at start, after its profile, and
follows its role's section; conductor passes the other sections on in the charters and briefs it
writes. Write where the operator's standing grant for class-C decisions is
([decision classes](../concepts/decisions-and-messages.md#authority)), and how the operator decides,
so conductor can pick the option the operator would. Commit it in the records directory.

The repository ships no rules: what is generic is in the profiles, what holds for one organization
is here.

## 3. Keep the workspace file in step

The blockers collector reads the instance's repositories from `.engineering/workspace.yaml` in the
records, and the specifications collector reads a `github` source's repositories through it. `conductor config workspace` rewrites it from the
config, with no network read: each checkout directly under `checkouts.root` (or a `local` source's
path) that has an `origin` remote, and the records directory itself. With checkouts `api`, `web`
and `conductor` under `~/example-org`:

```console
$ conductor config workspace
member: api ../../api
member: conductor ../../conductor
member: records ../../records
member: web ../../web
wrote: ~/example-org/records/.engineering/workspace.yaml
```

`--check` writes nothing. It exits 0 with one line when the file matches, and 1 naming each member
that differs:

```console
$ conductor config workspace --check
conductor: config workspace: ~/example-org/records/.engineering/workspace.yaml: member old-repo: source ../../../old-repo, not in the config
conductor: config workspace: ~/example-org/records/.engineering/workspace.yaml differs from the config; `conductor config workspace` rewrites it
```

Run it after a repository is cloned under the checkouts root or removed, and commit the file.
`config workspace` refuses when the records directory is not there yet, and points at
`conductor init`.

## 4. Give the records an origin

The collectors read every member at `origin/main`, the records directory included: each snapshot
runs `git fetch origin` in it and reads its `origin/main`. A records directory without an `origin`
fails the snapshot:

```console
$ conductor snapshot start-snapshot
01a13061-f655-7369-a638-51343d24b5c1
conductor: snapshot start-snapshot: snapshot 01a13061-f655-7369-a638-51343d24b5c1 failed: blockers: repository records, tried twice: `git fetch --quiet origin` exited 128: fatal: 'origin' does not appear to be a git repository fatal: Could not read from remote repository. Please make sure you have the correct access rights and the repository exists.
```

Create a private repository for the records on the forge, add it as `origin` and push `main`:

```console
git -C ~/example-org/records remote add origin <url of the private repository>
git -C ~/example-org/records push -u origin main
```

`git init` names the first branch after your `init.defaultBranch`; the collectors read `main`, so
rename the branch first if it is called something else. Keep the records private: they hold the
organization's decisions, rules and briefs.

## 5. Take the first snapshot

```console
conductor doctor
conductor snapshot start-snapshot
conductor snapshot snapshots
```

`start-snapshot` prints the snapshot's id and exits 0 when every collector answered. When it fails,
[troubleshoot](./troubleshoot.md#a-failed-snapshot) reads the reason.

Next: [start conductor](./start-conductor.md). Its first session reads the hand-over `conductor init`
committed and works through its first task.
