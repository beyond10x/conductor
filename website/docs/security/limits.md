---
title: Known limits
sidebar_position: 6
description: What conductor does not protect against today, what to do about each gap, and which rules are code and which are asked of the model.
lede: The guard catches a session's mistakes. It is not a sandbox, and these are the gaps.
source: crates/conductor/src/guard/rules.rs, confine/mod.rs, actor.rs, agent/claude_code.rs, gc.rs; crates/conductor/tests/adv_guard.rs; Taskfile.yml; .engineering/planning/story/
---

Each limit says what is not covered, what to do now, and whether a change is planned. "Planned"
means a story exists in the product's plan. It is not a date.

## No permission prompts, so the guard is the only check

Every session runs with `--permission-mode bypassPermissions`. Nothing asks the operator before a
tool call. Where the guard is not wired, nothing checks at all.

- **Do:** start controllers only through `conductor spawn`, which `conductor run` calls, and other
  sessions through `conductor session start`
  ([sessions](/docs/security/sessions#who-starts-a-session)). Provoke a denial after every change
  to the settings or the binary ([hardening](/docs/security/hardening#verify-the-guard)).
- **Planned:** no change to the permission mode.

## Bash rules are a heuristic and do not confine where a command writes

The shell rule denies the forms [the guard](/docs/security/the-guard#bash-and-monitor) lists.
Any other Bash write passes. A controller can write, overwrite or delete any file its user can,
other checkouts and the `conductor` binary included, and the records or config through a path the
command does not spell out: a relative path, a variable, `env -C <dir>`, a `cd` first. A
controller that reads an issue written by someone else can be told to run such a command.

- **Do:** run sessions as a dedicated user that reaches only the instance's directories, with no
  other work, keys or credentials. Read the dirty-file counts of each snapshot.
- **Planned:** no. Agent sessions run unboxed ([below](#agent-sessions-run-unboxed)).

## `git push` and direct forge API calls are not checked

The guard holds `gh` and `glab`, not `git push`, `curl` or another client of the forge's API. A
session can push to any branch, and call any endpoint, its token allows.

- **Do:** protect `main` and release branches with required reviews and status checks. Give
  sessions a token scoped to the instance's repositories, without admin, secrets or
  workflow-write rights ([hardening](/docs/security/hardening#forge)).
- **Planned:** no. The forge is the place for this control.

## WebFetch and the network are open

`WebFetch` and `WebSearch` are on the read-only list, and a shell command can open any
connection. A URL can carry data out.

- **Do:** restrict the session user's outbound traffic at the host or network where that matters.
- **Planned:** no.

## Unguarded sessions

- **The maintainer, the host roles and custom roles** start with empty settings when their role
  names no `settings`, so no guard runs ([the settings file](/docs/security/sessions#the-settings-file)).
- **A session started or resumed by hand** without `--settings <role settings file>` is not
  guarded and claims no lease.
- **Two conductors** are kept apart only by the conductor settings' lease
  ([the lease](/docs/security/the-guard#one-conductor-the-lease)). It lets a write through when
  `<state>/conductor.lease` does not read, and misses a Bash write it does not recognise, such as
  `git commit`. A pid reused after the holder ends denies the others their writes.

- **Do:** start and resume only through the documented commands. Check `claude agents` and
  `conductor session leases` before starting a conductor session. `conductor watch` reports
  `conductors: <n> live sessions named <name>: <ids>`. Never resume the running conductor's id in
  a second process. If no conductor runs and writes are denied as a second conductor's, remove
  `<state>/conductor.lease`.
- **Planned:** no.

## Agent sessions run unboxed

Every session is `claude --bg`, an ordinary process of its user, with no filesystem namespace. A
role's `limits`, `writable` and `network` are neither enforced nor measured. Only an instance
runtime runs in a slice with limits or in a substrate box
([sessions](/docs/security/sessions#boxes-slices-and-limits)). Disk admission is code for a
controller's builds, which run under `conductor build run`, and a profile rule for the rest.

- **Do:** use a dedicated user with a disk quota where the host supports it, and read the watch's
  disk and usage lines.
- **Planned:** no. The stories that put controller sessions in a box are archived.

## Each repository's own hooks run

Sessions load project settings, and `conductor trust` marks the records directory and every
checkout under `checkouts.root` as trusted. Each repository's `.claude/` settings and hooks run in
its controller's session without a prompt, unseen by the guard. A controller cannot write them,
but a merged pull request can.

- **Do:** review each repository's `.claude/` directory before you add it to an instance, and
  require review on changes to it, for example with a code-owners rule.
- **Planned:** no.

## A session's commands record as the operator

No v1 start path sets `CONDUCTOR_SESSION`, so a `conductor` command in a `claude --bg` session
records the Unix user as its actor, and `ActorNotHuman` does not refuse it
([actors](/docs/security/operators-and-credentials#actors)). The guard still denies a controller
every writing leaf. A conductor session started by hand can run `decision
record-operator-decision`.

- **Do:** do not start a conductor session by hand. Read the decision log for operator decisions
  you did not give, and reverse them with `conductor decision reverse-operator-decision`.
- **Planned:** no.

## Enforced in code, and asked of the model

| enforced in code | asked of the model (profiles only) |
|---|---|
| the guard decides every tool call of a session started with its settings | starting a session only through `spawn` and `session start` |
| unknown and MCP tools are denied unless a role's `read_only_tools` names them | keeping `read_only_tools` to tools that only read |
| file tools stay in the session's place; settings, profiles, config and records are protected | Bash writes outside the session's own repository |
| `cd`, `pushd`, `git -C` and git writes outside the session's places are denied (a heuristic) | `git push` only through the repository's own route |
| `gh` and `glab` write only through the bot route or `forge.route: direct`; credential writes are class H | commits and pushes through the bot route |
| a controller runs only the `conductor` leaves declared for it, and builds only under `build run` | message format and length |
| a controller messages only its conductor and its own sub-agents | handing a session over before its context runs out |
| `spawn` refuses a second live session of a name and a second running controller | not starting a conductor session by hand |
| the store refuses class O and H decisions recorded by conductor; the operator's commands refuse an agent actor | relaying only the operator's own words with `answer-escalated-as-operator` |
| a daemon write needs its own origin and token, or a person's bearer holding the grant | |
| the config file refuses a value that looks like a token, and takes secrets only as `secret:<name>` | |
| `conductor run` holds `<state>/run/lock`, so a second loop of the instance refuses | |

## Smaller gaps

- **A program word the shell builds without `conductor`** (`"$C" …`, a `$(printf …)` that spells
  it) is judged only by the words after it. A quoted literal before the leaf is denied, since the
  guard reads words after quote removal.
- **A script a shell reads from a here-document or a here-string is not read**, nor one after a
  flag cluster with `c` not last (`bash -ce '…'`).
- **A sub-agent is recognised by the shape of its id** in a controller's SendMessage.
- **Recording a denial can fail**: no state directory, or the store's lock held over 2 seconds.
  The call is still denied, with no record.
- **The Gates secret scan in conductor's own gate runs only when `B10X_GATES_POLICY` names a
  policy**; without one, it prints `B10X_GATES_POLICY is not set; the Gates scan is skipped`
  ([developer onboarding](../develop/onboarding.md#clone-and-run-the-gate)).
- **`conductor gc` keeps a checkout's `target/` only where a live session works.** A build started
  from a parent directory with `--manifest-path` is not seen, and its target can be removed while
  it runs.
