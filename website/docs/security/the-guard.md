---
title: The guard
sidebar_position: 2
description: The PreToolUse hook that decides every tool call of conductor and its controllers; its tool table, its rules per role, how it fails, and what it records.
lede: One hook reads each tool call, finds the place the session runs in, applies that place's rule and answers with an exit code.
source: crates/conductor/src/guard.rs, crates/conductor/src/guard/rules.rs, crates/conductor/src/guard/shell.rs, crates/conductor/src/session.rs; .claude/conductor-settings.json, .claude/controller-settings.json; crates/conductor/tests/guard_rules.rs, crates/conductor/tests/guard_hook.rs, crates/conductor/tests/adv_guard.rs
---

The guard is a Claude Code PreToolUse hook. The repository ships it wired in two settings files,
`.claude/conductor-settings.json` for conductor and `.claude/controller-settings.json` for each
controller, on the matcher `*`:

```console
conductor guard record-guard-decision --from-pre-tool-use || exit 2
```

Conductor's file also passes the session's process, as its SessionStart claim does, so the lease
check can tell the holder's process from another ([the lease](#one-conductor-the-lease)):

```console
conductor guard record-guard-decision --from-pre-tool-use --pid "$PPID" || exit 2
```

Every tool call of a session started with one of these files, and of that session's sub-agents,
runs this command before the tool does.

## How it decides

1. **Payload.** The hook reads the payload on standard input: the tool's name, its input, the
   session's id and its working directory (`cwd`).
2. **Place.** The `cwd`, resolved through every link, decides the rule set:
   - in a repository's checkout (`<checkouts root>/<repo>/` or `<root>/<group>/<repo>/`) or its
     managed worktrees (`<trees>/<repo>/`): the controller rules for that repository, unless a
     source of the instance excludes it. A controller's workers run in its managed worktrees, so
     their calls are decided as the controller's own.
   - in conductor's records (the instance's `records`): conductor's rules. Without a config file,
     the checkout of the repository named `conductor` is the records.
   - anywhere else: no rule set applies, and every ruled tool is denied.
3. **Rule.** The tool table below, then the rule for that tool and that role.
4. **Exit code.** 0 allows the call and prints nothing. 2 blocks it, with one line on standard
   error, `conductor guard: denied: <reason>`, which the session reads.

Places come from the instance the process runs: its config file (`$CONDUCTOR_CONFIG`, else the
default file) or, without one, the built-in instance. Paths are compared where they land: `~` is
the home directory, a relative path is taken from the `cwd`, and every symbolic link is followed,
dangling or not.

## The tool table

| tool | verdict |
|---|---|
| Edit, Write, NotebookEdit | decided by the file rule of the session's role |
| SendMessage | decided by the message rule of the session's role |
| Bash, Monitor | decided by the shell rule of the session's role (Monitor runs a shell command, so the Bash rule decides it) |
| Agent, Task (sub-agents) | denied when the call sets `cwd`, which would put the sub-agent under another place's rule, or `isolation: "remote"`, whose calls never reach this hook; allowed otherwise, wherever the session runs |
| the 24 read-only harness tools | allowed: Read, Glob, Grep, LS, WebFetch, WebSearch, TodoWrite, TaskCreate, TaskGet, TaskList, TaskUpdate, TaskOutput, BashOutput, TaskStop, KillShell, ToolSearch, ListAgents, ListMcpResourcesTool, ReadMcpResourceTool, Skill, CronCreate, CronList, CronDelete, ScheduleWakeup |
| a tool in the role's `read_only_tools` | allowed, for that role's sessions only |
| any other tool, MCP tools included | denied |

The read-only tools write no file, run no command and reach no other session. A role's
`read_only_tools` in the config ([config reference](/docs/reference/config)) extends that list
for the role: `conductor` for conductor's rules, `controller` for a controller's. A tool with a
rule of its own is decided by its rule whatever that list says. A session whose `cwd` is in no
place gets the built-in list only.

## File tools: Edit, Write, NotebookEdit

The target path is resolved first, `~` and links included. Then:

| a controller may write | conductor may write |
|---|---|
| inside its repository's checkout and its managed worktrees | its records: `decisions/`, `dispatches/`, `charters/`, `docs/handoff/`, `docs/analysis/`, `goals.jsonl`, `STATUS.md`, `NORTHSTAR.md` |
| its scratch: under `$TMPDIR`, or under `~/.cache/<repo>-*/` (`<repo>` its repository, `/` written `-`) | its scratch: under `$TMPDIR` |

Both are denied, wherever the file is:

- a `.claude/` settings file: a `.json` file whose name holds `settings`, in a directory named
  `.claude` (`settings.json`, `settings.local.json`, `controller-settings.json`,
  `conductor-settings.json`);
- the `settings` file of any role of the instance, the file that wires the guard;
- the `profile` file of any role, the system prompt its sessions start with.

A controller is denied, in addition:

- conductor's config: the config file the process resolves, the default file, the directory of
  the default file itself, and the instance's `state` directory and everything under it. A
  controller that could rewrite the config could give itself conductor's rules or a wider
  checkout.
- conductor's records, even where a config file puts them inside its checkout.

Scratch is never a repository's checkout or managed worktree, the records, the config or a
protected file, wherever `$TMPDIR` points. A `$TMPDIR` that is the home directory, or holds it, is
no scratch at all.

## SendMessage

| role | may message |
|---|---|
| controller | its instance's conductor session (`<session_prefix>-conductor`, or `conductor` without a prefix, by name or as `<name> [<ref>]`); a sub-agent it started; a socket address `uds:<path>/<pid>.sock` whose pid the session list (`claude agents --json`) gives a live session of that conductor |
| conductor, with a `session_prefix` `<p>` | sessions named `<p>-<name>`, sub-agents it started, and the sessions of the instances it serves |
| conductor, without a prefix | anyone, when its config file holds one instance; otherwise its own sessions and those of the instances it serves, never a session of an instance that names its own conductor role |

A sub-agent is recognised by the shape of its id, not by who started it. A `recipient` field
that names another session than `to` is denied. A denial for another instance's session names
both instances. The session list is read at most once per call and given 5 seconds; a list that
cannot be read denies a socket address.

## Bash and Monitor

The shell rule reads the command line. It is a heuristic: it catches the common forms of the
commands below, not every way to write them, and it does not confine where a command writes.

Denied in every guarded session:

- a `gh` call that is not a read. A read is `view`, `list`, `status`, `diff`, `checks`, `search`
  or `download` after any group; `gh run view|list|watch|download`; `gh api` with GET and no
  request fields; `gh api graphql` without a mutation; `gh auth status`; `gh search`; `gh status`;
  `gh help`; any call with `--help`; `--version` or `-h` with no group. Writes go through the
  repository's own route.
- a `glab` call that is not a read, held exactly as a `gh` write. A read is `glab mr
  view|list|diff`, `glab issue view|list`, `glab release view|list|download`, `glab repo
  view|list`, `glab ci view|list|status|trace|get`; `glab api` with GET and no request fields
  (`glab api graphql` reads only as any other endpoint does); `glab auth status`; any call with
  `--help`; `--version` with no group. No verb reads after any other group: a `glab` group not
  named here makes only writes.
- a command that names a `.claude/` settings file or a role's settings file and holds a write
  form;
- a command that names a role's profile file and holds a write form.

Denied in a controller's session, in addition:

- a `cd`, `pushd`, or `git -C`, `--git-dir` or `--work-tree` that reaches another repository's
  checkout or managed worktrees, or conductor's records;
- a command that names conductor's config (`$CONDUCTOR_CONFIG`, the config file, the default
  file, its directory, the `state` directory) and holds a write form;
- a command that names conductor's records, or a path under them, and holds a write form;
- a `conductor` command that is not a read: `conductor trust`, `conductor init`, and every leaf
  of the command groups except the views, `decision show`, `config show`, `config validate` and
  the hook's own `guard record-guard-decision`. `conductor doctor` reads, and is allowed.

A **write form** is a redirection to a file outside quotes (`2>&1`, `>&2` and `>/dev/null` are
none), one of `sed -i`, `sed --in-place`, `yq -i`, `yq --inplace`, `perl -i`, `perl -pi` or
` of=` anywhere, or one of `tee`, `cp`, `mv`, `rm`, `ln`, `install`, `truncate`, `touch`,
`rsync`, `patch` as a word. A path is "named" by a substring test on its spellings (absolute,
`~/…`, `$HOME/…`, `${HOME}/…`, quoted `$HOME`), not by a parse. So a command that names a
protected path while it writes another file is denied, and a command that writes a protected
file by a relative path, a variable or after a `cd` is not caught.

A `gh` or `glab` call hidden inside an interpreter string (`python -c "…"`, `node -e "…"`) is
not caught by the guard's parsing, which follows `sh -c` scripts but not another language's; only
the session's sandbox bounds such a call, and there is no operating-system sandbox yet
([limits](/docs/security/limits)).

### Forge credentials are class H

A `gh secret`, `gh auth`, `gh ssh-key` or `gh gpg-key` write, and a `glab variable`, `glab auth`,
`glab ssh-key`, `glab gpg-key`, `glab deploy-key` or `glab token` write, is denied as class H: it
needs the operator's hands, whoever authorized it. The denial says so and hands the operator the exact
command to run, prefixed with `!`, so it can be passed on as is.

### The read-flag exemption

Starting or resuming a session names its settings and profile files:
`claude … --settings <file>` or `--append-system-prompt-file <file>`. Claude Code only reads
those. So a protected file named **only** as the value of `--settings`,
`--append-system-prompt-file` or `--system-prompt-file` does not count as written, unless a write
form targets it: a redirection to it, or a writing program (`tee`, `cp`, `sed`, `dd` and the
others above) with it as a word.

The exemption does not apply when the shell could reach that value without naming it again: a
parameter other than `$HOME` (`$f`, `$1`, `${…}`), a backquote, a here-document or here-string, or
one of `sh`, `bash`, `zsh`, `python`, `node`, `perl`, `awk`, `xargs`, `eval`, `source`, `exec`,
`set`, `for`, `while`, `read`, `find`, `parallel` and the like in the command. Such a command is
decided as if the exemption did not exist.

## One conductor: the lease

Conductor's settings claim the instance's conductor lease when a session starts
(`conductor session claim`, from the SessionStart hook). The guard holds the lease on every write
of a session decided by conductor's rules. When the rule table allows such a session a write, the
guard reads the lease of the instance it runs before it answers. A write here is:

- Edit, Write or NotebookEdit, wherever the file is;
- SendMessage;
- a Bash or Monitor command that holds a write form (below), or that runs a `conductor` leaf the
  shell rule denies a controller;
- a Bash or Monitor command that names the lease file `conductor.lease`, or names the state
  directory or a path under it, whatever program it runs (`chmod`, `unlink`, `find -delete`,
  `python3 -c`). Such a command could remove the lease file or make it unreadable, and with no
  readable file the guard would let every write through.

The guard knows the calling session by the payload's `session_id` and, when the settings pass
it, by its process: conductor's settings pass `--pid "$PPID"`, the hook shell's parent. Claude
Code runs every hook of a session from the session's own process, so the session's own calls,
its sub-agents' and its Monitor commands all pass the pid the lease records. A second process
that resumes the holder's session id (`claude --resume <id>` by hand) passes its own.

While the holder's process lives, a write is denied if the lease names another session than the
payload's `session_id`, or if `--pid` names another process than the lease's. The reason:

```text
<tool> is denied: session <id> (pid <pid>) holds the conductor lease of instance <name> and its process lives: this session is a second conductor of the instance and must stop; it may read, and writes nothing
```

When the session id is the holder's and only the process differs, `this session` reads
`this process (pid <pid>), which carries the holder's session id,`.

The denial is recorded like any other. Reads stay allowed, so the second session
can still read `conductor session leases`.

The guard reads the lease from one small file, `conductor.lease` in the state directory (the one
denials are recorded in), and never opens the store for it. `conductor session claim` writes
that file once the store has kept a claim that takes, renews or takes over the lease. It writes a
staging file in the same directory and renames it into place, so a reader never sees half a
file. A refused claim leaves the file as it is. The store stays the record:
`conductor session leases` reads the store, not the file.

The verdict stays as the rule table gave it in these cases:

- the lease names this session, and `--pid` is not given or names the lease's process;
- the holder's process is gone, so the next SessionStart claim takes the lease;
- there is no state directory: without `--state-dir` and without a config file the guard reads no
  lease, so the lease is not enforced;
- there is no lease file;
- the file is not a regular file (a link is not followed), cannot be read, does not hold the
  instance, session id and pid, or names another instance. On these errors the lease check lets
  the call through.

A controller's calls and every read never read the file.

What the lease check does not cover:

- **Without `--pid` the holder is known by session id only.** The process check needs the
  settings to pass `--pid "$PPID"`, as the shipped conductor settings do. A settings file without
  it (one copied before this check, or a role's own `settings`) leaves a second process that
  resumes the holder's session id free to write while the holder runs.
- **The pid comes from `$PPID`.** The claim records the hook shell's parent as the session's
  process, and the guard compares the same value. If Claude Code ran hooks under one more process
  layer, the recorded pid would end as soon as the hook did. The lease would then never hold, and
  the check would let every write through (fail open).
- **A pid that names another process keeps a dead lease alive.** After the holder ends, its pid
  can be reused by an unrelated process, or name a process of another user. Either way it counts
  as alive, and every other conductor session is denied its writes (fail closed). To recover, the
  operator removes `<state>/conductor.lease`; the next session that starts claims the lease.

## Fail closed

Anything that goes wrong before a verdict exists is a denial:

- a payload that cannot be read or is not JSON;
- a payload with no `tool_name`; a ruled tool with no `cwd`, or a `cwd` that is not absolute;
- a `HOME` that is not set or not absolute;
- a panic inside the guard, which is caught and answered as a denial.

Claude Code treats any exit status other than 0 and 2 as "proceed". The wired command ends in
`|| exit 2`, so a missing `conductor` binary, or a guard killed before it answered, still blocks
the call.

A config file that does not load leaves the hook on the built-in instance's places. What is
denied without a file stays denied; the places the broken file names grant nothing.

## What is recorded

Only denials. Each one is recorded in conductor's store with the repository, the session id, the
tool, the target (the file, the recipient or the command), the verdict, the reason and the time.
The store is the one under `--state-dir`, else the instance's `state` directory when a config file
names one, else none: the hook runs in any session's working directory and must not create a
store there.

An allowed call is answered without opening the store. For a write of a conductor session the
guard reads one small file first, its copy of the lease ([the lease](#one-conductor-the-lease)).
Deciding a call took about 1.5 ms in the
repository's latency test; that is a measurement, not a promise. Recording a denial is given at
most 2 seconds, since another session's hook may hold the store's writer lock. The denial stands
whether or not it was recorded; a failed recording adds one line on standard error.

`conductor guard guard-decisions` lists the recorded denials
([audit trail](/docs/security/audit-trail), [CLI reference](/docs/reference/cli#conductor-guard-guard-decisions)).

## Read next

- [Confinement](/docs/security/confinement): how sessions are started so that the guard runs.
- [Known limits](/docs/security/limits): what the guard does not check.
