---
title: The guard
sidebar_position: 2
description: The PreToolUse hook on every tool call of a guarded session; places, tool table, rules per role, the lease, failure and records.
lede: One hook reads each tool call, finds the place the session runs in, applies that place's rule and answers with an exit code.
source: crates/conductor/src/guard.rs, crates/conductor/src/guard/rules.rs, crates/conductor/src/guard/shell.rs, crates/conductor/src/session.rs, crates/conductor/src/cli.rs; crates/conductor/kinds/claude-code/; crates/conductor/tests/guard_rules.rs, guard_hook.rs, adv_guard.rs
---

The guard is a Claude Code PreToolUse hook on the matcher `*`, wired by the two settings files the
binary carries in `crates/conductor/kinds/claude-code/`. `controller-settings.json`, for a
controller whose role names no `settings`:

```console
conductor guard record-guard-decision --from-pre-tool-use || exit 2
```

`conductor-settings.json`, for a conductor session, also passes the session's process and claims
the [lease](#one-conductor-the-lease) at SessionStart:

```console
conductor guard record-guard-decision --from-pre-tool-use --pid "$PPID" || exit 2
conductor session claim --from-session-start --pid "$PPID" || exit 2
```

Every tool call of such a session and its sub-agents runs the hook first. Any other role's
session, the maintainer's included, is unguarded unless its role names a `settings` file that
wires the hook ([sessions](/docs/security/sessions#the-settings-file)).

## How it decides

1. **Payload.** Standard input holds the tool name, input, session id and working
   directory (`cwd`).
2. **Place.** The `cwd`, resolved through every link, dangling or not, picks the rule set:
   - a repository's checkout (`<checkouts.root>/<repo>/` or `<checkouts.root>/<group>/<repo>/`)
     or managed worktrees (`<checkouts.trees>/<repo>/`): the controller rules for that
     repository, unless a source's `exclude` names it. Workers in those worktrees are decided as
     the controller.
   - the instance's `records`: conductor's rules. Without a config file, the checkout of the
     repository named `conductor` is the records.
   - anywhere else: every ruled tool is denied.

   The places are those of the config file (`$CONDUCTOR_CONFIG`, else
   `~/.b10x/conductor/conductor.yaml`), else the built-in instance's.
3. **Exit code.** 0 allows the call silently. 2 blocks it with one line on standard
   error, `conductor guard: denied: <reason>`, which the session reads.

## The tool table

| tool | verdict |
|---|---|
| Edit, Write, NotebookEdit | the role's file rule |
| SendMessage | the role's message rule |
| Bash, Monitor | the role's shell rule |
| Agent, Task | denied when the call sets `cwd` or `isolation: "remote"` |
| 24 read-only harness tools | allowed: Read, Glob, Grep, LS, WebFetch, WebSearch, TodoWrite, TaskCreate, TaskGet, TaskList, TaskUpdate, TaskOutput, BashOutput, TaskStop, KillShell, ToolSearch, ListAgents, ListMcpResourcesTool, ReadMcpResourceTool, Skill, CronCreate, CronList, CronDelete, ScheduleWakeup |
| a tool in the role's `read_only_tools` | allowed to that role's sessions |
| any other tool, MCP tools included | denied |

The `conductor` role's `read_only_tools` extend conductor's rules, the `controller` role's a
controller's ([config reference](/docs/reference/config#role)). A session in no place gets the
built-in list only.

## File tools: Edit, Write, NotebookEdit

| a controller may write | conductor may write |
|---|---|
| its repository's checkout and managed worktrees | its records: `decisions/`, `dispatches/`, `charters/`, `docs/handoff/`, `docs/analysis/`, `goals.jsonl`, `STATUS.md`, `NORTHSTAR.md`, `REPORT.md` |
| scratch: under `$TMPDIR` or `<cache>/scratch/<repo>/`, `/` in `<repo>` written `-` | scratch: under `$TMPDIR` |

Denied to every session: a `.claude/` settings file (a `.json` whose name holds `settings`, in a
directory named `.claude`), and any role's `settings` or `profile` file. Denied to a controller
as well: the records, even inside its checkout, and conductor's config: the file the
process resolves, the default file, its directory, and the instance's `state` directory
and everything under it.

Scratch is never a checkout, a worktree, the records, the config or a protected file. A `$TMPDIR`
that is or holds the home directory is no scratch.

## SendMessage

| role | may message |
|---|---|
| controller | its conductor (`<session_prefix>-conductor`, `<session_prefix>-director` for this release, `conductor` without a prefix), as `<name>` or `<name> [<ref>]`; its sub-agents; a socket `uds:<path>/<pid>.sock` whose pid `claude agents --json` gives a live session of that conductor |
| conductor, prefix `<p>` | sessions named `<p>-<name>`, its sub-agents, and the sessions of the instances it serves |
| conductor, no prefix | anyone, when the file holds one instance; else its own sessions and those of the instances it serves, never one of an instance with its own conductor role |

A sub-agent is known by the shape of its id. A `recipient` other than `to` is denied. A session
list unread within 5 seconds denies a socket address.

## Bash and Monitor

The shell rule reads the command line: a heuristic that does not confine where a command writes
([limits](/docs/security/limits)).

| denied | to |
|---|---|
| `pkill`, `killall`, `killall5`, `skill`, a `kill` whose operands are not all numeric pids or `$!` | every session |
| a `gh` or `glab` call that is not a read, but for the [write routes](#forge-calls) | every session |
| a write to a `.claude/` settings file, a role's settings or profile ([write targets](#write-targets)) | every session |
| a post to the dashboard ([dashboard answers](#dashboard-answers)) | every session |
| `timeout` around `conductor snapshot start-snapshot`, which kills it mid-write | every session |
| `cd`, `pushd`, `git -C`, `--git-dir`, `--work-tree`, `GIT_DIR=`, `GIT_WORK_TREE=` into another repository or the records | a controller |
| a git write whose target is outside its checkout, worktrees and scratch | a controller |
| a command naming conductor's config with a [write form](#write-forms) | a controller |
| a write to the records | a controller |
| a `conductor` leaf a controller may not run | a controller |
| a build, test or gate outside `conductor build run` | a controller |

**Git.** The target is where `-C` leaves the command (else the `cwd`), and each `--git-dir`,
`--work-tree`, `GIT_DIR`, `GIT_WORK_TREE`, an earlier `export` included. These read anywhere:
`status`, `log`, `diff`, `show`, `rev-parse`, `ls-files`, `ls-tree`, `cat-file` and the other
plumbing reads, `config --get` and `--list`, `remote` alone or `show`, `branch` and `tag`
listing, `stash list`, `worktree list`. Every other subcommand writes, an alias too. A target in a
variable or substitution is not decided.

**Conductor leaves.** `LEAVES` in `crates/conductor/src/cli.rs` declares each leaf's access. A
controller runs the reads, `doctor` included, plus `guard record-guard-decision`, `build run` and
`message send`. The reads kept for conductor (`repository shipped`, `resource usage`, `records
check`, `agent attach`, `tui`), every other leaf, and a leaf the shell expands or `xargs` supplies
are denied.

**Builds.** `cargo build|test|check|clippy|nextest|install|run|bench|doc|rustc|fix` (and `b`,
`t`, `c`, `r`), `task check|gate|install|regen|docs-generate|ui`, `npm ci`, `npm test` and `npm run
build|test` are held behind launchers, in `sh -c` and in substitutions. The denial names
`conductor build run --reserve auto -- <command>` ([disk](/docs/concepts/disk)). `cargo fmt`,
`rustfmt`, `cargo metadata` and `cargo tree` are allowed.

### Forge calls

| forge | reads |
|---|---|
| `gh` | `view`, `list`, `status`, `diff`, `checks`, `search` or `download` after any group; `gh run` `view`, `list`, `watch`, `download`; `gh api` GET with no request fields; `gh api graphql` without a mutation; `gh auth status`, `gh search`, `gh status`, `gh help`; `--help`; `--version` or `-h` alone |
| `glab` | `mr` `view`, `list`, `diff`; `issue` `view`, `list`; `release` `view`, `list`, `download`; `repo` `view`, `list`; `ci` `view`, `list`, `status`, `trace`, `get`; `glab api` GET with no request fields; `glab auth status`; `--help`; `--version` alone. Every other group only writes |

Two routes write for the session's own repository, the one its checkout's `origin` names
(conductor's: its records' `origin`):

| route | allowed |
|---|---|
| bot route, `b10x-gates gh` | `release` `create`, `upload`, `edit`, `view`; `run rerun`; `workflow run`; with `--repository` naming the own repository |
| `forge.route: direct` | `gh pr` `create`, `edit`, `comment`, `merge`, `ready` and `glab mr` `create`, `update`, `note`, `merge` with `-R` naming it; `glab ci retry` or `run`; `glab api -X POST` to its pipeline and job retry endpoints |

A credential write is class H under every route: `gh secret`, `auth`, `ssh-key`, `gpg-key`, and
`glab variable`, `auth`, `ssh-key`, `gpg-key`, `deploy-key`, `token`, `config`, `securefile`.
The denial hands the operator the command, prefixed with `!`. A call inside `python -c "…"` or
`node -e "…"` is not caught: the parser follows `sh -c` only.

### Dashboard answers

Denied: a command naming `/v1/answers` or `/v1/instances/<name>/…/answers`. Also one naming an
instance's `api.sock`, or `:<daemon.port>` after any loopback spelling (`127.0.0.1`, `localhost`,
`[::1]`, `0x7f000001`, none), with a POST or a body. That is `-X POST`, `--request POST`, `--method=POST`,
httpie's `POST` or a request item, a body flag (`-d…`, `--data…`, `--json`, `-F`, `--form…`, `-T`,
`--upload-file`, `--post-data`, `--post-file`), or `data=`, `json=`, `.post(`, `'POST'`, `body:`.
`tr -d`, `cut -d`, `date -d` are no body. The denial names `conductor decision
answer-escalated-as-operator`. A script, a variable or a percent-encoded path gets past this
substring test ([dashboard writes](/docs/security/operators-and-credentials#dashboard-and-api-writes)).

### Write forms

A write form is a redirection to a file outside quotes, other than `2>&1`, `>&2` and
`>/dev/null`. It is also `sed -i`, `sed --in-place`, `yq -i`, `yq --inplace`, `perl -i`, `perl
-pi`, ` of=`, or `tee`, `cp`, `mv`, `rm`, `ln`, `install`, `truncate`, `touch`, `rsync` or `patch`
as a word. A path is named by substring on
its spellings: absolute, `~/…`, `$HOME/…`, `${HOME}/…`, quoted `$HOME`. The config rule needs only
the name and a write form. A relative path, or one after `cd`, is not caught.

### Write targets

The records, settings and profile rules deny a command naming the path only when one of its
commands:

- redirects to it (`>`, `>>`, `2>`);
- has it as an operand of `tee`, `cp`, `mv`, `rm`, `ln`, `install`, `truncate`, `touch`, `rsync`,
  `patch`, `dd` or in-place `sed` or `yq`;
- writes to a target the guard cannot read: a variable other than `$HOME` and `$TMPDIR`, a
  substitution, a backquote, `{}` after `find -exec` or `xargs`, or `$HOME`, `$TMPDIR` or `~` the
  command sets;
- runs `sh`, `bash`, `zsh`, `python`, `python3`, `node`, `ruby`, `perl`, `xargs`, `parallel`,
  `eval` or `source`.

A read operand, a quoted program or a here-document body is no write:
`awk '/^## Controller/,/^## Worker/' <records>/rules.md > $TMPDIR/brief.md` passes, `sed -i …
<records>/rules.md` does not.

## One conductor: the lease

The guard holds the lease on every write under conductor's rules: Edit, Write, NotebookEdit,
SendMessage, a Bash or Monitor write form or `conductor` leaf denied to a controller, and any
command naming `conductor.lease` or the state directory (`chmod`, `unlink`, `find -delete`).

The caller is known by `session_id` and `--pid "$PPID"`. Claude Code runs a session's hooks from
its own process, so its calls, its sub-agents' and Monitor's pass the pid the lease records. A
second process resuming the holder's id (`claude --resume <id>`) passes its own. While the
holder's process lives, a write is denied if the lease names another session id or `--pid`
another process:

```text
<tool> is denied: session <id> (pid <pid>) holds the conductor lease of instance <name> and its process lives: this session is a second conductor of the instance and must stop; it may read, and writes nothing
```

When only the process differs, `this session` reads `this process (pid <pid>), which carries the
holder's session id,`. Reads, `conductor session leases` among them, stay allowed.

The guard reads `<state>/conductor.lease`, not the store. `session claim` writes it, by rename,
after the store keeps the claim. The rule table's verdict stands when:

- the lease names this session and `--pid` is absent or matches;
- the holder's process is gone;
- there is no state directory or no lease file;
- the file is no regular file, does not read, lacks instance, session id or pid, or names another
  instance.

A pid reused by another process keeps a dead lease alive. Removing `<state>/conductor.lease`
recovers.

## Fail closed

Denied before any rule: an unreadable or non-JSON payload, no `tool_name`, a ruled tool with no
`cwd` or a relative one, `HOME` unset or relative, a panic in the guard. Claude Code proceeds on
any exit status but 0 and 2, so the wired command ends in `|| exit 2`: a missing binary or a killed
guard blocks the call. A config file that does not load denies every call as `ConfigUnloaded`,
with no fallback to the built-in places.

A hook acts for the instance its settings pin ([pinning](/docs/security/sessions#the-settings-file)).

## What is recorded

Only denials: repository, session id, tool, target, verdict, reason, time, in the store of
`--state-dir`, else of the instance's `state` under a config file, else nowhere
([audit trail](/docs/security/audit-trail)). An allowed call opens no store.
`tests/guard_latency.rs` measured about 1.5 ms per decision. Recording gets at most 2 seconds
(`RECORD_DEADLINE`). The denial stands either way, and a failed recording prints one line on
standard error.
