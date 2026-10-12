---
title: Known limits
sidebar_position: 5
description: What the guard and the rest of conductor do not protect against today, what to do about each limit, and what is planned.
lede: The guard catches a session's mistakes. It is not a sandbox, and these are the gaps.
source: crates/conductor/src/guard.rs, crates/conductor/src/guard/rules.rs; crates/conductor/tests/adv_guard.rs; Taskfile.yml; .engineering/planning/story/single-conductor-lease.md, session-writes-bwrap.md, session-limits-systemd.md
---

Each limit below says what is not covered, what to do about it now, and whether a change is
planned. "Planned" means a story exists in the product's plan; it is not a date.

## No permission prompts; the guard is the only check

Every session runs with `--permission-mode bypassPermissions`. Nothing asks the operator before a
tool call. If the guard is not wired, nothing checks at all.

- **Do:** start sessions only through `task conductor:start` and the controller start form, both
  with the role's settings file ([confinement](/docs/security/confinement)). Provoke a denial after
  every change to the settings or the binary ([hardening](/docs/security/hardening)).
- **Planned:** no change to the permission mode; the stories below narrow what a session can do.

## Bash rules are a heuristic; they do not confine where a command writes

The shell rule reads the command line. It denies a `cd`, `pushd` or `git -C` into another
repository, a `gh` write, a `conductor` write, and a write form that names a protected path. Any
other Bash write passes: a controller can write, overwrite or delete any file its user can, other
checkouts and the `conductor` binary included, and conductor's records or config through a path
the command does not spell out (a relative path, a variable, a `cd` first). The guard is no
defence against a hostile prompt: a controller that reads an issue or pull request written by
someone else can be told to run such a command.

- **Do:** run sessions as a dedicated user that can reach only the instance's directories. Keep
  other work, keys and credentials out of that user's reach. Read the dirty-file counts of each
  snapshot.
- **Planned:** sessions in a box conductor controls, writing only their own places (a `bwrap`
  write boundary), drafted in the plan.

## `git push` and direct forge API calls are not checked

The guard denies `gh` writes. It does not look at `git push`, and it does not look at `curl` or
any other client calling the forge's API. A session with a token can push to any branch, and
call any endpoint, that the token allows.

- **Do:** protect `main` and release branches with required reviews and required status checks.
  Give sessions a token scoped to the instance's repositories, without admin, secrets or
  workflow-write rights where the forge allows that ([hardening](/docs/security/hardening)).
- **Planned:** nothing in the guard; the forge is the place for this control.

## WebFetch and the network are open

`WebFetch` and `WebSearch` are on the read-only list, and a shell command can open any network
connection. A URL can carry data out.

- **Do:** if that matters for an instance, restrict outbound traffic for the session user at the
  host or network level.
- **Planned:** no.

## Unguarded sessions

- **conductor-dev runs without the guard by default.** `task dev:start` passes `--settings` only
  when the `conductor-dev` role names a settings file.
- **A session started or resumed by hand** without `--settings <role settings file>` is not
  guarded.
- **Two conductors are refused only at session start, and only with the role's settings.** A
  conductor session started or resumed with the shipped `.claude/conductor-settings.json` claims
  the instance's conductor lease from its SessionStart hook (`conductor session claim`). While
  another live session holds the lease, the claim is refused and names that session's id and
  pid; once the holder's process is gone, the next claim takes the lease over and records whom
  from. The refusal is shown to the operator; it does not end the second session, and nothing
  denies that session's tool calls yet. A session started or resumed without those settings
  claims nothing and is not refused, and two conductors can then write the same records.

- **Do:** start and resume only through the tasks and the documented start form. Check
  `claude agents` for a live conductor session before starting or resuming one, and
  `conductor session leases` for the session that holds the lease.
- **Planned:** the guard denies the tool calls of a conductor session that does not hold the
  lease, and the watch reports two live conductor sessions, settings or not (drafted).

## No operating-system sandbox, no resource limits

Sessions run as an ordinary process of their user. There is no filesystem namespace, no CPU,
memory or process limit, and disk admission (asking for a build slot when disk is low) is a rule
of the profile.

- **Do:** use a dedicated user with a disk quota where the host supports it, and watch
  `conductor watch run`'s disk and usage reports.
- **Planned:** each instance and session under systemd limits, and the write boundary above
  (drafted).

## Each repository's own hooks run

Sessions load project settings, and `task trust` marks the records directory and every checkout
under `checkouts.root` as a trusted Claude Code workspace. Each repository's `.claude/` settings
and hooks therefore run in its controller's session without a prompt. A hook is not a tool call,
so the guard does not see it. A controller cannot write `.claude/` settings files, but a merged
pull request can change them.

- **Do:** review each repository's `.claude/` directory before you add it to an instance, and
  require review on changes to it (for example with a code-owners rule).
- **Planned:** no.

## Some rules exist only in the profiles

Starting controller sessions, one controller session per repository, the bot route for commits
and pushes, disk admission and build slots, message length and hand-overs are instructions to the
model, not code. A model can fail to follow them.
`conductor controller start-controller` records a controller; it does not start a session yet.

- **Do:** read the controller list (`conductor controller controllers`) against the live sessions (`claude agents`, or the dashboard) now and then.
- **Planned:** starting and stopping a controller's session from `conductor` (`spawn`).

## Smaller gaps

- **A program word the shell builds without naming `conductor`** (`"$C" …`, a `$(printf …)` that
  spells it) is judged only by the words that follow it. A `conductor` run whose group, leaf or
  flag word the shell expands, a substitution around `conductor`, and `xargs` handing it its words
  are denied to a controller; so is a quoted literal before the leaf, since the guard sees words
  after the shell's quote removal.
- **A script a shell reads from stdin, a here-document or a `watch` string is not read**, nor is an
  alias the session defines: a `conductor` run inside one is judged as no run at all.
- **A sub-agent is recognised by the shape of its id** in a controller's SendMessage, not by who
  started it.
- **Recording a denial can fail** (no state directory named, or the store's lock held for more
  than 2 seconds). The call is still denied, but there is no record.
- **The Gates secret scan in `task check` runs only when `B10X_GATES_POLICY` names a policy.**
  Without one, the scan is skipped with a line on standard error.
