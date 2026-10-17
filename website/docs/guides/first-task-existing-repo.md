---
title: One task in an existing repository
sidebar_position: 0
description: Set conductor up in a checkout you already have, give it one goal, let it dispatch one controller, review the pull request it opens, and know what to do when it stops.
lede: The shortest way to see conductor work is one task in a repository you already have, from conductor setup --apply in its checkout to a pull request you merge yourself.
source: crates/conductor/src/setup.rs, crates/conductor/src/doctor.rs, crates/conductor/src/spawn.rs; .agents/conductor.md, .agents/repo-controller.md; SETUP.md
---

This guide takes one task in a repository you already work in, here `alpha` of `example-org`
checked out at `~/src/alpha`, from nothing to a merged pull request. It needs the `conductor` CLI
([getting started](../getting-started.md) installs it), Claude Code (`claude`), and a checkout of
the conductor repository for the tasks that start sessions
([start conductor](./start-conductor.md)). The repository needs no planning store.

Output lines differ on your machine: host, versions, paths and ids are examples.

## 1. Set the checkout up

In the checkout, check it first, then apply what is safe:

```console
cd ~/src/alpha
conductor setup
conductor setup --apply
```

`conductor setup` prints the rows of `conductor doctor` (the machine, each program conductor
starts, each forge login, the worktree workspace), then four rows of the repository, and one
verdict as its last line. Before `--apply`, on a machine whose programs are ready, it ends:

```text
store: none
instance: alpha names ~/src/alpha
records: missing (~/.b10x/conductor/alpha/records); run conductor init --instance alpha
trust: not trusted (~/src/alpha); run conductor trust --instance alpha
not ready: records (missing), trust (not trusted)
```

`store: none` is no failure: conductor reads a planning store only where there is one.
`--apply` writes a config file with one instance naming the repository when there is no config
file (with one, it prints the instance to add and changes nothing), then runs `conductor init`
and `conductor trust`. It never logs in, installs a program, migrates a store or activates a
worktree workspace: each of those is a line for you to run, such as

```text
to run: gh auth login --hostname github.com
to run: worktree activate --profile <profile> --workspace <root>
```

The second line appears when the checkouts root is not inside an activated worktree workspace;
`<profile>` is your committed worktree profile template. `worktree activate` writes your own
configuration, so conductor only prints it. Run each `to run:` and `to install:` line, then
`conductor setup` again until its last line is `ready`.

## 2. Check the programs

```console
$ conductor doctor
host: workstation
system: Linux 6.18.0
architecture: x86_64
conductor 0.18.2
git: present, git version 2.55.0
gh: present, gh version 2.80.0
glab: not needed
claude: present, 2.1.296 (Claude Code)
jq: present, jq-1.8.2
task: present, 3.53.1
aep: present, aep 0.71.2
worktree: present, b10x-worktree-cli 0.18.1
ess: present, ess 0.59.0
gh: logged in to github.com as example-user
worktree workspace: ~/src activated
pins: none
aep: 0.71.2 meets the build's pin 0.71
worktree: 0.18.1 meets the build's pin 0.18.1
ess: 0.59.0 meets the build's pin 0.59
ready
```

While the workspace is not activated, doctor prints `worktree workspace: ~/src not activated`
with the command under it, `  activate: worktree activate --profile <profile> --workspace <root>`,
and ends `not ready: worktree workspace (not activated: ~/src)`. Doctor writes nothing.

## 3. Name the controller's profile and settings

A controller starts with the `controller` role's profile and settings file, and
`conductor spawn` refuses a role that names neither. In a checkout of the conductor repository,
`task install` writes the three profiles to `~/.b10x/conductor/profiles/`; add the roles to the
instance in the config file, then validate it:

```yaml
    roles:
      - role: conductor
        harness: claude
        model: opus
        profile: ~/.b10x/conductor/profiles/conductor.md
        settings: ~/example-org/conductor/.claude/conductor-settings.json
      - role: controller
        harness: claude
        model: opus
        profile: ~/.b10x/conductor/profiles/repo-controller.md
        settings: ~/example-org/conductor/.claude/controller-settings.json
```

```console
conductor config validate
conductor config show
```

The shipped settings files pin no instance, so `conductor config validate` prints a note for each:
`conductor session start` and `conductor spawn` start the role with a copy pinned to the instance.
`conductor config validate` names each problem by its YAML path and exits 1;
`conductor config show` prints the instance with its defaults, the `records` directory among them.

## 4. Give it one goal

A dispatch serves a goal. Record one, with what shows it is met, and confirm it:

```console
conductor goal propose-goal --goal-id first-task --title "Fix the flaky date test" --exit-evidence "the test passes 20 runs in a row on main"
conductor goal confirm-goal --goal-id first-task
conductor goal goals
```

```text
goal_id     state      title                    exit_evidence
first-task  Confirmed  Fix the flaky date test  the test passes 20 runs in a row on main
```

For a stricter gate, decide now who merges. A charter merges with `merge: controller` by default:
the controller merges its pull request once its full gate is green. With the line `merge: human`
in the charter, the controller opens the pull request and never merges it; you review and merge.
For a first task, use `merge: human`.

## 5. One dispatch

Start conductor in the instance's records directory, from the conductor checkout, and open it:

```console
task conductor:start
task conductor:attach
```

Tell it the task: the goal `first-task`, the repository `alpha`, and `merge: human`. Conductor
writes the charter `<records>/charters/alpha.md` (goal, scope, boundaries, the `merge:` line, the
current dispatch), records the dispatch with `conductor dispatch send-dispatch`, and starts the
controller with `conductor spawn --repository alpha`, which prints the controller session's id.
Read both back:

```console
$ conductor dispatch dispatches
dispatch_id      state  repository  goal_id     brief                    sent_at               priority  waits_on
DSP-20261013-01  Sent   alpha       first-task  Fix the flaky date test  2026-10-13T09:00:00Z  1
$ conductor controller controllers
```

`conductor spawn` refuses, and starts nothing, when the checkout is not under the checkouts root,
is not trusted, has no charter, or a controller of that repository runs already; the refusal
names which.

## 6. What the controller does

The controller works in `~/src/alpha` and reports only to conductor:

- It reports `started`. With no planning store, the report says so,
  `started; no planning store: /aep:init would add one`, and it goes on: the dispatch's brief
  is the plan, written into the pull request's description. It creates no store itself.
- It works in managed worktrees under the instance's trees root. When `worktree create` refuses
  with `workspace-not-activated`, it reports `worktree workspace not activated: <root>` to
  conductor and, until that is answered, works in a plain `git worktree add` tree it removes when
  done. Running the `to run:` line of step 1 answers it.
- It changes the repository's ESS specification first when it has one, implements, and runs the
  repository's full gate once on the integrated head.
- It pushes one branch and opens one pull request.

## 7. Review the pull request

With `merge: human`, the controller reports `review <url>` and stops there: it never merges and
never enables auto-merge. Open the link, review it as you would a colleague's, ask for changes in
the pull request or through conductor, and merge it yourself. The dispatch stays open until a
snapshot shows the pull request merged; conductor's next cycle takes one, then closes the
dispatch and reports the goal's state.

## When it stops

| what you see | what to do |
|---|---|
| `conductor setup` or `conductor doctor` ends `not ready:` | run the `to run:` or `to install:` line it names, then check again |
| the controller reports `blocked` | conductor answers what it can; a question only you can answer reaches you with the exact command |
| no report for a long time | `conductor watch run` names a silent session; `claude agents --json` shows its state |
| `conductor spawn` refused | the refusal names its cause; [troubleshoot](./troubleshoot.md) lists the fixes |
| the controller asks to merge under `merge: human` | conductor answers "a person merges here"; merge it yourself |

To stop a controller, `conductor controller stop-controller` ends the session `conductor spawn`
started, through the instance's confinement provider, and records it stopped;
`pause-controller` and `resume-controller` stop it and resume it from its transcript.
