---
title: One task in an existing repository
sidebar_position: 0
description: Set conductor up in a checkout you already have, give it one goal and a charter, let the loop start one controller, review the pull request it opens, and know what to do when it stops.
lede: The shortest way to see conductor work is one task in a repository you already have, from conductor setup --apply in its checkout to a pull request you merge yourself.
source: crates/conductor/src/setup.rs, doctor.rs, spawn.rs, run.rs, run/dispatch.rs; crates/conductor/roles/controller.md; SETUP.md
---

This guide takes one task in a repository you already work in, here `alpha` of `example-org`
checked out at `~/src/alpha`, to a merged pull request. It needs the `conductor` CLI and the
programs `conductor doctor` checks ([getting started](../getting-started.md)). The repository needs
no planning store.

Output lines differ on your machine: versions, paths and ids are examples.

## 1. Set the checkout up

In the checkout, check first, then apply what is safe:

```console
cd ~/src/alpha
conductor setup
conductor setup --apply
```

`conductor setup` prints the rows of `conductor doctor`, then four rows of the repository, the
forge route of its instance, then one verdict. Before `--apply`, on a host with no config file,
it ends:

```text
store: none
instance: none names ~/src/alpha, which --apply names with the source {github: example-org}; run conductor setup --apply
records: missing (no instance names the repository)
trust: not trusted (~/src/alpha)
forge route alpha: gates without a gates_policy: set instances[0].gates_policy, or instances[0].forge.route: direct
not ready: instance (none), records (missing), trust (not trusted)
```

`store: none` is no failure: conductor reads a planning store only where there is one.

With no config file, `--apply` writes a `conductor.config/2` file with one instance, `alpha`. It
names `daemon.store`, `placement: systemd`, `session_prefix: alpha`, the source from the
repository's `origin`, and `~/src` as the checkouts root. Then it runs `conductor init` and
`conductor trust`. With a config file whose instances do not name the repository, it prints the
instance to add as `to write:` lines and changes nothing.

`--apply` chooses no forge route, and the `forge route alpha:` line stays until the instance
writes one; doctor ends `not ready: forge route alpha (no gates_policy)` until then.
Write `forge: {route: direct}` under the instance, and a controller writes its pull requests
with your own forge login; or write a `gates_policy`, the policy file of the bot route
([forge calls](../security/the-guard.md#forge-calls)). With `B10X_GATES_POLICY` naming a
readable policy file in its environment, `--apply` writes that path as the `gates_policy` of
the instance it creates.

`--apply` then checks each box as conductord does before it starts one. When conductord would refuse
an instance's box, a fifth row, `box:`, names the refusal and the verdict reads
`not ready: box (refused)`. With a database already at `~/.b10x/conductor/conductor.sqlite`, the
file `--apply` writes names that store, which a box cannot write
(`placement.writable-root-shared`): it prints `to write: daemon.store:
sqlite:~/.b10x/conductor/store/conductor.sqlite` and moves nothing. Stop conductord, move the
database there and write that value, or give the instance a placement without a box.

When doctor's row reads `worktree workspace: ~/src not activated`, `--apply` also activates the
checkouts root as a worktree workspace. It writes a profile for the instance beside the config
file, `~/.b10x/conductor/worktree/alpha.toml`, unless one is there, and runs
`worktree activate --profile ~/.b10x/conductor/worktree/alpha.toml --workspace ~/src`. You need
no profile of your own.

It never logs in, installs a program or migrates a store. Each of those is a line for you to run:

```text
to run: gh auth login --hostname github.com
to run: worktree activate --profile ~/.b10x/conductor/worktree/alpha.toml --workspace ~/src
```

The second line appears only when `worktree` refused the activation; the reason is on standard
error. Run each `to run:` and `to install:` line, then `conductor setup` again until its last line
is `ready`.

## 2. Start the instance

An instance whose checkouts root holds the checkout, such as the `home` instance of
[getting started](../getting-started.md), already runs: go on.

For the instance `--apply` wrote, do steps 5 and 6 of
[getting started](../getting-started.md#5-start-the-substrate-daemon): start `substrate-daemon`
with this instance's writable roots, `--writable-root state-alpha=<state>`,
`--writable-root records-alpha=<records>` and `--writable-root cache-alpha=<cache>`, then
`conductor daemon serve --install`. `substrate-daemon` reads its writable roots at start, so a
running one is restarted with the new root. A running conductord starts the new runtime at its
next pass after the config file changed.

## 3. Add the lead role

`conductor spawn` refuses an instance with no `lead` role (`NoControllerRole`). Add the roles to
the instance in the config file:

```yaml
    roles:
      - role: director
        harness: claude
        model: opus
      - role: lead
        harness: claude
        model: opus
```

```console
conductor config validate
```

A lead starts from the profile and settings the binary carries, which wire the guard
([sessions](/docs/security/sessions#profile-or-agent)). Claude Code refuses a background session
in `bypassPermissions` until your OS user has accepted its disclaimer once:
run `claude --dangerously-skip-permissions` in a checkout and accept it
([bypass permissions](/docs/security/sessions#bypass-permissions)).

## 4. Give it one goal

A dispatch serves a goal. Record one with what shows it is met, and confirm it:

```console
conductor goal propose-goal --goal-id first-task --title "Fix the flaky date test" --exit-evidence "the test passes 20 runs in a row on main"
conductor goal confirm-goal --goal-id first-task
conductor goal goals
```

```text
goal_id     state      title                    exit_evidence
first-task  Confirmed  Fix the flaky date test  the test passes 20 runs in a row on main
```

## 5. Write the charter and the dispatch

Write `<records>/charters/alpha.md`, where `<records>` is the `records` value of
`conductor config show`: the goal `first-task`, the scope, the boundaries, and the line
`merge: human`. With it the controller opens the pull request and never merges it. Then record the
dispatch:

```console
conductor dispatch send-dispatch --repository alpha --goal-id first-task --brief "Fix the flaky date test" --sent-at 2026-11-09T09:00:00Z
conductor dispatch dispatches
```

The loop starts a controller for each Active repository that has a charter and no running
controller. It gives the controller its charter and the repository's open dispatch. It does so at
each cycle tick: at the runtime's start, then every `cadence.cycle` (2 h).

```console
conductor repository activity
conductor controller controllers
```

A repository with no real commit in 7 days is Inactive, and the loop passes it by. Mark it Active:
`conductor repository activate-repository --repository alpha --marked-by Operator --reason "first task"`.

## 6. What the controller does

The controller works in `~/src/alpha` and reports only to conductor, with
`conductor message send`:

- It reports `started`. With no planning store the report says so,
  `started; no planning store: /aep:init would add one`, and it goes on: the dispatch's brief is
  the plan, written into the pull request's description. It creates no store itself.
- It works in managed worktrees under the instance's trees root. When `worktree create` refuses
  with `workspace-not-activated`, it reports `worktree workspace not activated: <root>` and, until
  that is answered, works in a plain `git worktree add` tree it removes when done. Running the
  `to run:` line of step 1 answers it.
- It changes the repository's ESS specification first when there is one, implements, and runs the
  repository's full gate once on the integrated head.
- It pushes one branch and opens one pull request.

## 7. Review the pull request

With `merge: human` the controller reports `review <url>` and stops there: it never merges and
never enables auto-merge. Review it as you would a colleague's, ask for changes in the pull request
or through conductor, and merge it yourself. The dispatch stays open until a snapshot shows the
pull request merged.

## When it stops

| what you see | what to do |
|---|---|
| `conductor setup` or `conductor doctor` ends `not ready:` | run the `to run:` or `to install:` line it names, then check again |
| the instance is `PlacementRefused` | [a runtime is refused](./troubleshoot.md#a-runtime-is-refused) |
| the controller reports `blocked` | conductor answers what it can. A question only you can answer reaches you with the exact command |
| no report for a long time | the loop delivers a stalled or silent controller one check-in and restarts it from its charter at the second. `conductor session list` shows its state |
| `conductor spawn` refused | the refusal names its cause ([refusals](/docs/security/sessions#refusals)) |

`conductor controller stop-controller --controller-id <id> --reason <why>` ends the controller's
session and records it stopped with the reason. `pause-controller` and `resume-controller` stop it
and resume it from its transcript ([run conductor](./start-conductor.md#controllers)).
