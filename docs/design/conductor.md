# Conductor: design

## 1. What conductor is

Conductor coordinates agent sessions across the repositories of one organization. It has two
parts:

- **A session.** One coding-agent session, named `conductor`, running in the directory that holds
  conductor's records.
- **A CLI.** The `conductor` binary records what the session decides and dispatches, reads the
  state of the repositories, and guards the conductor and controller sessions.

The session does four things:

1. It holds the organization's direction: goals, and which repositories serve them. It checks that
   direction against what happens in the repositories.
2. It tracks activity, planning state, pull requests, CI, releases and live sessions across the
   repositories of its instance (§ 9).
3. It dispatches one controller session per repository checkout. Each controller plans locally,
   dispatches its own workers, and reports only to conductor.
4. It takes the decisions the controllers raise. The architectural ones go to the operator.

Conductor keeps only its own records: decisions, dispatches, goals, controllers, messages, guard
verdicts and snapshots. Every other fact stays with the tool that owns it:

| fact | where it lives |
|---|---|
| the work | each repository's AEP planning store |
| code, issues, pull requests, CI, releases | the repositories and their forge |
| the shape of a repository's system | its ESS specification |

Conductor never edits another repository. A controller does that.

## 2. Roles

| role | who | owns | never |
|---|---|---|---|
| operator | the person running the instance | direction, architecture, new or retired repositories, credentials and spending, public-facing risk | routine approvals |
| conductor | one session, `conductor` | the cross-repository plan, decisions within its grant, dispatch, the loop's records, the operator brief | edits outside its records; anything that defines or runs conductor (§ 12) |
| conductor-dev | one session, `conductor-dev`, in the conductor repository | conductor's code, specification, plan, design, profiles, adapters and tasks; the instance's `rules.md`; observing conductor and turning what it sees into stories; installing the binary; restarting conductor | dispatching to controllers; writing conductor's records |
| repository controller | one session per repository, cwd = the checkout | its repository's plan, waves, reviews, releases and workers | messages to anyone but conductor; edits outside its repository |
| worker | a sub-agent of a controller | one unit of work | anything outside the unit |

Each role runs on the harness and model the instance's `roles` name, and starts with the harness
agent and settings file the role names, if any (§ 9, [`../config.md`](../config.md)). Every session
follows a generic profile (§ 11) and then the instance's `rules.md` in its records directory: the
rules of that one organization, one section per role, each rule with its source.

## 3. Topology

A star. Controllers send to `conductor` and receive only from it. A controller that needs something
from another repository raises a `[NEED]`; conductor routes it to that repository's controller as a
dispatch (`conductor message route-need`), or queues it.

A session outside the instance that needs something from one of its repositories talks to
conductor, not to a controller. Its request becomes a dispatch with an owner and a place in the
order.

conductor-dev sits outside the star. It talks to the operator and to conductor, never to a
controller.

**One controller per repository.** `conductor controller start-controller` refuses a second
controller for a repository that already has a running or paused one (`ControllerAlreadyRunning`).

**The channel.** Claude Code sessions reach each other with SendMessage. The specification also
models a harness-neutral mailbox (`Transport: Mailbox`) for controllers on a harness that cannot
receive SendMessage. No command writes or reads the mailbox yet.

## 4. The loop

```text
sense     conductor snapshot start-snapshot     one snapshot: repositories, pull requests, main CI,
                                                releases, planning blockers, specifications, sessions
compare   goals and servings                    which goal each repository's work serves; idle goals;
                                                work serving no goal; red signals
decide    decision requests                     answer controller requests; pick next work per
                                                repository; escalate class O and H (§ 5)
dispatch  conductor dispatch send-dispatch      start, steer or stop a controller; one charter each
record    the store, and the session's logs     one record per decision, dispatch and message
report    operator brief                        once a day and on escalation, deviations only
```

A cycle runs on the instance's `cadence.cycle`, and a daily cycle at `cadence.daily` carries the
operator brief. The session schedules both. A cycle also runs when a controller reports.

Between cycles the watch (§ 8) wakes conductor on a change. Session starts do not wake it.

Snapshots are records, so a restarted conductor resumes from the newest complete snapshot, not
from its context. A snapshot a collector did not answer is failed with the collector's name
(`snapshot fail-snapshot`). A failed snapshot is kept and is never read as the current state.

Steering includes cancelling a dispatch (`dispatch cancel-dispatch`) when its brief no longer
serves its goal.

### Collectors

`conductor snapshot start-snapshot` runs five collectors in order and completes the snapshot only
when all five answered.

| collector | reads | records |
|---|---|---|
| repositories | `git` in each checkout, `gh` for each `github` source, `glab api --hostname <host>` for each `gitlab` source's group and subgroups | one row per repository: visibility, archived, open issues, `main` head, real commits in 7 days, behind `main`, dirty files, worktrees, latest release, unreleased commits, planning-store version, and `in_catalog` when the instance names a catalog |
| github | per repository, `gh pr list`, `gh run list` and releases for a `github` source; merge requests, default-branch pipelines and releases through `glab api --hostname <host>` for a `gitlab` source | open pull requests (a GitLab merge request is one), the newest `main` run per workflow (a GitLab pipeline is one), releases, merged pull requests |
| blockers | `git fetch`, an export of `origin/main`, `aep plan workspace list` | open `decision-blocker` artifacts across the AEP stores |
| specifications | the same exports, `ess specify validate`, `ess verify conform synthesize` | per repository: specification present, opted out or missing; valid or refused; scenario count |
| sessions | `claude agents --json` | live sessions, their cwd, the repository each is bound to |

A `gitlab` source names its `host` (default `gitlab.com`) and is read through `glab api --hostname
<host>`: repositories and specifications, merge requests as pull requests, pipelines as workflow
runs, releases. `gh` is needed only with a `github` source, `glab` only with a `gitlab` source.

The catalog is optional. An instance that names one (`catalog`: a repository under the checkouts
root, a directory in it, and how a file's stem spells a name) gets `in_catalog` on every
repository row: `true` when a `.json` entry in that directory on the catalog repository's
`origin/main` names the repository, `false` when none does. Without a catalog, or without a
checkout of the catalog repository, the field is absent (`null` in JSON).

The views (`snapshot repositories`, `pull-requests`, `workflow-runs`, `blockers`,
`specifications`, `sessions`, `releases`, `merged-pull-requests`, `snapshots`) list every kept
snapshot's rows. Filter on the newest `snapshot_id`.

## 5. Decision classes

**Class C: conductor decides.** It records the decision (`decision record-conductor-decision`) and
tells the operator only by exception.

- Wave approvals; commit, push, pull request and merge words.
- Releases through each repository's own process.
- Dependency upgrades, CI repair, baseline moves of the repository's gates.
- Disk and build scheduling. Cleanup of merged worktrees.
- Order and priority inside an accepted epic or initiative. Design choices inside an approved
  design.
- Issue triage, which profile or model a dispatch uses, and which repository waits for which.
- Everything else that is not class O or H below: conductor takes the recommended option that can
  be undone, records it and reports it as taken.
  The operator asks for nothing to be held for them ("there should be no 'Waiting on you' — that's
  the point of conductor", 2026-10-12).

**Class O: the operator decides.** Conductor prepares the options and a recommendation, and
escalates (`decision escalate-request`).

- Data loss: deleting another session's or person's work, an archive that is the only copy, a
  repository.
- Rewriting published history.
- A new, renamed or retired repository.
- An architecture decision: a change an architecture decision record would carry (the system's
  structure, a component's responsibility, a cross-repository contract or technology choice).

**Class H: needs the operator's hands, not judgement.** Setting a secret, a login, a paid account,
a permission only the operator holds. One exception: a command the instance's `rules.md` names as
granted by the operator for unattended runs, with their words and its scope (for example publishing
a committed file to the secrets that carry it); conductor runs exactly that command and reports
only its failure. The grant is the operator's alone, given directly, never inferred from a general
instruction.

Conductor keeps one numbered list, with the exact command or
page for each item and the controllers waiting on it (`decision hands-todo`). Conductor never
records a class-H decision: `record-conductor-decision` refuses it as `ReservedForOperator`. The
operator records what was done with `record-operator-decision`, class H.

An escalated request is answered only by the operator (`decision answer-escalated-request`).
Conductor answers an open request (`decision answer-request`). Conductor reverses only its own
decisions (`decision reverse-conductor-decision`); the operator reverses any
(`decision reverse-operator-decision`).

When a class is unclear, conductor treats it as class C, takes the option that can be undone, and
records why. That record is how the
boundary moves over time. A request already answered is answered again from the record, never
asked of the operator a second time.

Decision ids and dispatch ids carry the prefixes the specification gives them; a decision id also
carries its UTC date and a number. Given no `--decision-id`, the record commands allocate the next
id for the UTC date of `--decided-at`.

Who decides class C and class O is also written in the instance's `authority` (§ 9).

### Authority

Conductor decides class C on the operator's standing grant (`authority.class_c`). The instance's
`rules.md` says where that grant is written and how the operator decides, so conductor can pick the
option they would. Every class-C decision conductor sends carries its decision id; one without an id
is a recommendation.

The controller profile tells a controller to act on a `[DECISION …]` that carries an id as the
operator's, within conductor's grant. Where the operator's own harness instructions would make a
controller ask them instead, they need the same grant; without it, controllers ask the operator and
the loop stalls on routine approvals.

## 6. Messages

A message's first line is machine-readable. The rest is free text, at most 20 lines. Detail goes in
a file in the sender's own repository, and the message carries its path.

| first line | from → to | meaning |
|---|---|---|
| `[DISPATCH <repo> <dispatch-id>] <goal id> <what to do>` | conductor → controller | a brief |
| `[REPORT <repo> <dispatch-id>] <started\|progress\|blocked\|unblocked\|done\|failed> <one line>` | controller → conductor | progress on a dispatch |
| `[DECISION-REQUEST <repo> <request-id>] <question>` | controller → conductor | options A/B/C and a recommendation below |
| `[DECISION <request-id>] <option> <one-line reason> (<decision-id>)` | conductor → controller | the answer |
| `[NEED <repo> <request-id>] <other-repo> <what is needed>` | controller → conductor | a cross-repository request |
| `[RESOURCE <repo>] <build-slot\|disk\|usage> <amount>` | controller → conductor | asked before a large build |
| `[ESCALATION <request-id>] <O\|H> <question>` | conductor → operator | class O or H |
| `[HANDOVER <repo>] context <n>k` | conductor → controller | write a hand-over and stop, above `thresholds.context_handover`; design only, not in the specification |
| `[RESTART conductor] <reason>` | conductor-dev → conductor | § 12; design only, not in the specification |

conductor records every message it receives (`message receive-message`), and rejects one whose
first line does not parse (`message reject-message`) by answering with the format.

Each REPORT verb but one moves the dispatch through its `dispatch report-*` command: `started`,
`blocked`, `unblocked`, `done`, `failed`. A `progress` report is a message only. A `done` closes a
dispatch that is sent, started or blocked: a controller that never reported `started`, or finished
while blocked, needs no `started` or `unblocked` recorded first.

A decision request also lands in the controller's own planning store as a `decision-blocker`, where
the repository has one. The controller clears the blocker stating what was decided, with no
reference to conductor; repositories carry no pointer to conductor. Conductor records each request
it receives (`decision raise-decision-request`, with the blocker reference). The `blockers`
collector is the cross-check: an open blocker with no recorded request is a controller that did not
report.

A `[RESOURCE]` request is granted or refused against the instance's thresholds
(`resource grant-resource`, `refuse-resource`, `release-resource`), except disk and build slots:
those come from the host ledger (§ 9, Resources), and conductor answers a `[RESOURCE <repo>]
disk` or `build` with "use conductor build run".

## 7. The guard

| rule | enforced by |
|---|---|
| a controller's cwd is its checkout | the session is started there |
| a controller's Edit, Write, NotebookEdit only inside its repository's checkout and managed worktrees, or in its scratch | the guard |
| a controller's SendMessage only to `conductor` or to a sub-agent it started | the guard; a sub-agent by the shape of its id, not by who started it |
| no `gh` or `glab` writes; writes to the forge go through the repository's own route; a forge write of a credential is class H, handed to the operator, unless `rules.md` grants that one command (§ 5) | the guard |
| no `cd`, `pushd` or `git -C` into another repository or into conductor's records | the guard (a heuristic) |
| no write to conductor's records or conductor's config | the guard: file tools; Bash a write target in the records, or a write form in a command that names the config |
| no session writes a `.claude/` settings file (`settings*.json`, `controller-settings.json`, `conductor-settings.json`), a role's `settings` file or a role's `profile` file, by a file tool or a Bash write target | the guard |
| no `conductor` command that writes, only its views | the guard |
| conductor's Edit, Write, NotebookEdit only to its records, or in its scratch | the guard |
| a controller's build, test or gate (`cargo build\|test\|check\|clippy\|nextest\|install\|run`, `task check\|gate\|install\|ui\|regen\|docs-generate`, `npm run build\|test`, `npm ci`) runs under `conductor build run`, which reserves its disk in the host ledger | the guard: the denial names `conductor build run --reserve <size> -- <command>` |

Scratch is a session's own temporary space: under `$TMPDIR` for every session, and for a
controller also under `<cache>/scratch/<repo>/`, `<cache>` its instance's own `cache` and `<repo>`
its repository with `/` written `-`: two instances never share a scratch directory. Scratch
is never a repository's checkout or managed worktree, conductor's records or its config, wherever
`$TMPDIR` points, and a `$TMPDIR` that is the home directory or holds it is no scratch at all.

Conductor's config is the config file the process resolves, the default file
`~/.b10x/conductor/conductor.yaml` and the directory `~/.b10x/conductor/` itself, and, with a
config file, the instance's `state` directory and everything under it. The rest of
`~/.b10x/conductor/` is not the config: an instance's `records` are there by default, and keep
their own rule (conductor writes them, a controller does not).

The guard is a Claude Code PreToolUse hook. The repository ships it wired in
`.claude/conductor-settings.json` (conductor) and `.claude/controller-settings.json` (each
controller, named by the controller role's `settings`), on the matcher `*`, so every tool call
reaches it; a tool without a rule of its own is allowed only from the read-only allowlist and the
role's `read_only_tools`, and denied otherwise:

```console
conductor guard record-guard-decision --from-pre-tool-use || exit 2
```

It reads the hook payload on standard input and decides by the payload's `cwd`:

- a cwd in a repository's checkout or managed worktrees runs under the controller rules, unless a
  source of the instance excludes that repository;
- a cwd in conductor's records (the instance's `records` when a config file names it) runs under
  conductor's rules, which allow edits to the records (`decisions/`, `dispatches/`, `charters/`,
  `docs/handoff/`, `goals.jsonl`, `STATUS.md`, `NORTHSTAR.md`, `REPORT.md`) and to scratch, and deny `gh` and
  `glab` writes;
- a cwd in no repository is denied every tool that has a rule.

Exit 0 allows the call. Exit 2 blocks it, with one line of reason on standard error. Anything that
fails before a verdict exists is a denial, and the `|| exit 2` covers a missing binary.

Only denials are recorded, in the store under `--state-dir`, else under the instance's `state` when
a config file names it, else nowhere: the hook runs in any session's working directory and must
not plant a store there. `guard guard-decisions` reads them back. An allowed call is answered
without opening the store, because every tool call of every guarded session runs the hook.

The Bash rule is a heuristic over the command line, and it does not check where a command writes.
It denies a `cd`, `pushd` or `git -C` into another checkout or the records, a `gh` write, a
`glab` call that is not a read (`mr view|list|diff`, `issue view|list`, `release
view|list|download`, `repo view|list`, `ci view|list|status|trace|get`, `api` with GET and no
request fields, `auth status`, `--help`, `--version`), held exactly as a `gh` write, a `conductor`
write, a command that writes a settings file, a role's profile file or (a controller's)
conductor's records, a command that names conductor's config and holds a write form anywhere
in it, and (a controller's) build, test or gate that does not run under `conductor build run`:
its denial names `conductor build run --reserve <size> -- <command>`, 8G for cargo and `task`,
2G for npm and `task ui`, and `cargo fmt`, `rustfmt`, `cargo metadata` and `cargo tree` stay
allowed. A write form is a redirection to a file outside quotes (`2>&1`, `>&2`, `>/dev/null` and a
`>` inside a quoted program such as `awk 'NR>=490'` are none), or a write command or flag (`tee`,
`cp`, `mv`, `rm`, `sed -i` and the like). A Bash command is decided on its write targets: for the
records, the settings and the profile files the guard reads the targets of the command's
redirections and the operands of its writing programs (`tee`, `cp`, `mv`, `install`, `sed -i`,
`perl -i` and the like), so a path that appears only as a read operand or inside a quoted or
here-document body is not written, and a controller may copy part of the records' `rules.md`
into its scratch. A target held in a variable or a substitution, or a shell or interpreter on the
line, is decided on the name and the write form, as before. A denied write names the session's
scratch, and a denied `cd` says to read by absolute path instead. Any other Bash write passes: a controller can write,
overwrite or delete any file its user can, other checkouts and the `conductor` binary included,
and conductor's records or config through a path the command does not spell out. Each
snapshot reports dirty files in every checkout, which turns such a write into a finding. The guard
catches mistakes; it is no defence against a hostile prompt (README, Security model). `git push` is left to the
forge's branch protection, and conductor-dev runs without it. An operating-system sandbox for
Bash is story `bash-sandbox`.

## 8. The watch

`conductor watch run` runs cheap probes between cycles and prints one line per change. Conductor
runs it with `--follow` under the harness `Monitor` tool.

| probe | source | line |
|---|---|---|
| sessions | `claude agents --json`, sessions in the instance's checkouts, trees or records | `session exited (blocked): …`, `session gone: …`; `conductors: <n> live sessions named <name>: …` when two or more live sessions carry the conductor role's name |
| usage limit | the tail of each such session's transcript | `usage limit: …`; also a desktop notification (`notify-send`) |
| context | the newest usage line of each live session's transcript | `context: <repo> <n>k tokens` above `thresholds.context_handover` |
| silent controllers | the modification times of each live controller's transcript and its sub-agents' | `session silent: <name> <id> <m>m` when a controller `working` or `waiting` and all its sub-agents have written nothing for longer than `thresholds.silent_session`; again only after it wrote and fell silent anew |
| stalled sessions | the dispatch log's `sent` and report lines of each live controller a dispatch was sent to, and the newest commit in its checkout and its managed trees | `session stalled: <name> <id> <m>m` when such a controller, `working` or `waiting`, has had neither a report conductor received nor a commit for longer than `thresholds.stalled_session`, however its transcript moves; once per stall |
| queued builds | the open grants of the host ledger | `build queued: <grant> <repository\|session> <m>m` for a grant of the instance `Requested` longer than `thresholds.build_queued`, once per grant |
| `main` CI | for each Active repository of the newest complete snapshot, `gh run list` for a `github` source, the default-branch pipeline through `glab api --hostname <host>` for a `gitlab` source | `CI red on main: …`, `CI green again on main: …` |
| free disk | `statvfs` on `/` | each floor once per fall: `disk low: <n>G free on /` under `thresholds.disk_low`, again only after it was back at `disk_clear`; `disk admit: <n>G free on / (builds wait)` under `thresholds.disk_admit`, again only after it was back at `disk_admit`; `disk gate stop: <n>G free on / (gates stop)` under `thresholds.gate_stop`, again only after it was back at `gate_resume` |

A pass runs every `cadence.watch` (`--every <SECONDS>`), and CI is read every `cadence.ci`
(`--ci-every <SECONDS>`). A session exit that a dispatch log line explains is not reported. The
watch keeps its state under `<state>/watch/`.

## 9. Config and instances

One file, `conductor.config/1`, names conductor's instances. An instance is one organization:
where its repositories come from, where their checkouts are, where conductor keeps its records,
state and cache, which harness, model, agent and settings file each role runs with, how many
controllers and sub-agents work at once, its cadence, thresholds, retention, reports, authority,
the operator's name and an optional catalog. The specification is `spec/domains/config.yaml`;
every field is in [`../config.md`](../config.md).

The binary reads some fields; the sessions read the rest. A profile writes a config value as
`section.key`, such as `thresholds.gate_stop`, meaning that field of
`conductor config show --format json`. So one generic profile serves every instance, and a value
that differs per organization is a config key or a rule in the instance's `rules.md`, never an
edit to the profile.

The file is `--config <FILE>`, else `$CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml`.
The instance is `--instance <NAME>` (on `config validate` and `config show`), else
`$CONDUCTOR_INSTANCE`, else the file's `default`, else its only instance. Without a file, a
built-in instance applies.

Every command resolves the instance before it runs, and a file that does not validate stops the
command with its problems. `conductor config validate` names each problem by its YAML path.
`conductor config show` prints the effective instance after defaults (`--format text|json|yaml`).

A repository is the directory holding `.git` at depth 1 or 2 under a root (`<root>/<repo>` or
`<root>/<group>/<repo>`), named by its path under the root. A `local` or `gitlab` source may
`exclude` paths under its root. A `gitlab` source names its `host` (default `gitlab.com`);
`conductor config workspace` writes a member for each checkout under the checkouts root whose
`origin` remote is on that host and under the source's group, but those it excludes. An instance's `catalog` names a repository under the checkouts
root and a directory in it; the repositories collector reads it (§ 4).

The file never holds a secret. A value that looks like a token is refused by its path and never
printed.

### Resources

Build slots come from the host ledger, not from messages. Every build, test or gate a session
runs is `conductor build run --reserve <size> [--repository <repo>] -- <command>`: the binary
grants the reservation at once when free disk minus the live reservations minus the floor covers
it; when it does not, it reclaims what needs no decision (`conductor host reclaim`: build output
of idle managed trees and of worktree archives, caches past their age, snapshots past retention)
and grants, or refuses with the shortfall, exit 2, and records one operator item
(`decision hands-todo`) per fall naming what is left by path and size. The command runs in its own
process group under a disk watchdog, paused under `thresholds.gate_stop` and resumed over
`thresholds.gate_resume`; its peak is recorded with the reservation, which is released when the
command exits or is killed. `conductor host grants` shows the ledger, shared by every instance on
the host (`spec/domains/host.yaml`). The guard denies a controller's build that bypasses it (§ 7),
and no profile grants a slot by message. The concept page is `website/docs/concepts/disk.md`.

## 10. The store

The records live under the state directory: `--state-dir <DIR>`, else the instance's `state` when
a config file names it, else `state/` under the working directory. Each record is one stream of an
eventlog tree store: one immutable file per event, read through an index built when the store
opens.

| directory | holds |
|---|---|
| `<state>/tree/` | every record but the observations: snapshots, goals, servings, marks, decisions, requests, controllers, dispatches, messages, resource requests, guard verdicts |
| `<state>/observations/` | the observations of each snapshot, one tenant per snapshot |
| `<state>/watch/` | the watch's state files |

When a snapshot completes, the store keeps the observations of the newest `retention.snapshots`
complete snapshots and drops the tenants of the others. The `Snapshot` records stay. A command that
touches no observation, the guard among them, never opens `observations/`.

A view opens only an existing store and creates nothing. A command creates the store where none
is. `conductor store migrate` copies an older eventlog-file store, or a `tree/` that still holds
observations, into this layout and keeps the old one beside it.

The session also keeps human-readable logs in its records directory: `decisions/YYYY-MM.jsonl`,
`dispatches/YYYY-MM.jsonl`, `charters/<repo>.md` and `docs/handoff/`. The CLI writes the two
logs: once the store has kept a record, the command that recorded it appends exactly one line, a
JSON object with `at`, `event`, `repo` and the fields its event declares
(`conductor.dispatch.DispatchLogLine`, `conductor.decision.DecisionLogLine`), to the file of the
line's month:

| log | commands, and the `event` each appends |
|---|---|
| `dispatches/` | `dispatch send-dispatch` and `conductor spawn` (`sent`, spawn's with the session id), `dispatch report-started`, `report-blocked`, `report-unblocked`, `report-done`, `report-failed` (`started`, `blocked`, `unblocked`, `done`, `failed`), `dispatch cancel-dispatch` (`cancelled`), `message receive-message` (`message-received`), `resource grant-resource`, `refuse-resource` (`resource-granted`, `resource-refused`), `controller revise-charter` (`charter-revised`) |
| `decisions/` | `decision record-conductor-decision`, `record-operator-decision` (`decided`), `raise-decision-request` (`requested`), `answer-request`, `answer-escalated-request` (`answered`), `escalate-request` (`escalated`), `withdraw-request` (`withdrawn`), `reverse-conductor-decision`, `reverse-operator-decision` (`reversed`) |

A line for an event one of these commands records is never written by hand.
`conductor records check` reads both logs and the store's decisions and exits 1 naming each
decision id a dispatch line cites that nothing holds, each line whose keys its log does not
declare, and each decision line in the dispatch log. `charters/` and `docs/handoff/` are
written by the session.

### Repository activity

Conductor processes issues and pull requests only for repositories that are **Active**.
`conductor repository activity` decides each one:

- A `RepositoryMark` (`repository mark-repository`, `activate-repository`,
  `deactivate-repository`), set by the operator or conductor with a reason, wins and is never
  overwritten.
- Without a mark, a repository is Active when the newest complete snapshot counts at least one real
  commit on any `origin` branch in 7 days and it is not archived.

A commit is not real when it is a merge, an organization-wide sweep (the same normalised subject in
at least 3 repositories in the window), a maintenance subject, or touches only docs, CI, planning
or lock files. Every branch counts, not `main` alone, so a repository whose work sits on branches
is still Active.

`conductor repository shipped --since <RFC 3339>` lists releases and merged pull requests since an
instant, from the newest complete snapshot.

## 11. The CLI

`conductor` is a Rust CLI with clap derive. It holds no planning model of its own and calls the
tools that own each fact.

**Spec first.** Its nouns and commands are an ESS specification in `spec/`. The command tree is
the `cli:` block of `spec/components.yaml` plus the local commands of `crates/conductor/cli.yaml`.
`crates/conductor-model` is generated from `spec/` and never edited by hand.

| group | does |
|---|---|
| `snapshot` | start, complete or fail a sense run; record observations; the views of what it saw |
| `goal` | propose, confirm, drop or meet goals; which repositories serve each |
| `repository` | activity marks; `activity`; `shipped` |
| `decision` | requests, escalations, decisions, reversals; `hands-todo`; `show` |
| `controller` | start, pause, resume, stop a controller; pause and stop end the session `conductor spawn` started, and resume resumes it, through the instance's confinement provider; `revise-charter` |
| `dispatch` | send, cancel and report on dispatches |
| `message` | receive, handle, reject and route messages |
| `resource` | request, grant, refuse and release resources; `usage` (token spend from transcripts) |
| `guard` | the hook and its verdicts |
| `config` | `validate`, `show`; `workspace` (write `<records>/.engineering/workspace.yaml` from the config; `--check` only compares) |
| `dashboard` | `serve`: a read-only page on 127.0.0.1 (default port 7313) |
| `doctor` | each program conductor starts, present with its version or missing, and each one older than its `b10x.toml` pin; exit 1 on either; `gh` is needed only with a `github` source and `glab` only with a `gitlab` source, otherwise `not needed` |
| `init` | write and commit the instance's records directory skeleton; refuse to overwrite any file |
| `store` | `migrate` |
| `trust` | mark every git checkout under `checkouts.root`, and the records directory, as trusted Claude Code workspaces in `~/.claude.json` (`task trust`) |
| `watch` | `run` |

Not implemented yet; each answers exit 2:

- `snapshot publish-board` and `snapshot boards` (the generated `STATUS.md`);
- `--input-json -` on every command that offers it;
- `guard record-guard-decision` without `--from-pre-tool-use`;
- starting the controller's session from `controller start-controller`, which records the
  controller only: `conductor spawn` starts the session and records it.

### Profiles

| profile | used by |
|---|---|
| `.agents/conductor.md` | the conductor session |
| `.agents/conductor-dev.md` | the conductor-dev session |
| `.agents/repo-controller.md` | every repository controller |
| `rules.md` (in the instance's records) | every session, after its profile: the instance's own rules, one section per role, each with its source |
| `charters/<repo>.md` (in the records) | the per-repository brief: goal, boundaries, current dispatch, standing decisions, the paths of the controller profile and `rules.md` |

The profiles are generic: they name no organization, and a value that differs per instance is
written `section.key` and read from the config (§ 9). What holds for one organization only goes in
its `rules.md`, which this repository does not ship. There is one controller profile.
Per-repository differences go in the charter, so a rule changes in one place.

A profile's rules live in `.agents/`, readable by any harness. For Claude Code, `.claude/agents/`
holds one adapter per profile (`conductor.md`, `conductor-dev.md`, `repo-controller.md`), each
carrying only the agent frontmatter and a pointer at its `.agents/` file. A role's `agent` in the
config names the adapter its session starts with, and its `settings` the settings file:
`.claude/controller-settings.json` for a controller, which wires the guard (§ 7).
`task conductor:start` starts conductor with `.claude/conductor-settings.json`.

A controller's `charter_revision` counts the revisions sent to that running controller. It starts
at 1 and `controller revise-charter` raises it.

## 12. Developing conductor: conductor-dev

Conductor runs the organization. It does not build itself. A second session, `conductor-dev`,
owns conductor's development, watches how conductor behaves, and rebuilds and restarts it.

| who | writes |
|---|---|
| conductor | its records: `decisions/`, `dispatches/`, `charters/`, `docs/handoff/`, `goals.jsonl`, `STATUS.md`, `NORTHSTAR.md`, `REPORT.md`; `.engineering/workspace.yaml`, through `conductor config workspace` |
| conductor-dev | `crates/`, `spec/`, `.engineering/`, `docs/`, `.agents/`, `.claude/`, `Taskfile.yml`, `Cargo.*`, `AGENTS.md`, `README.md`; the instance's `rules.md` |

Conductor sends its faults and requests to conductor-dev as `[NEED conductor <id>] conductor-dev
<what>`, instead of editing its own profile or design.

conductor-dev reads, to find improvements:

- decisions later reversed, and escalations that turned out to be class C;
- the operator's corrections in conductor's transcripts;
- conductor's failed tool calls, and commands it invented;
- refusals from other sessions, and messages held for the operator;
- `[NEED conductor …]` messages from conductor.

Each finding becomes a draft story in the AEP store in `.engineering/`, spec first, delivered in
waves.

**Rebuild.** After a wave merges and `task check` is green, conductor-dev runs `task install`. The
CLI keeps no state outside the state directory, so a new binary needs no restart.

**Restart.** A `rules.md` change reaches a running conductor as a relayed diff, like the
controllers', and a restart is only for the profile, the adapter, the controller profile or the
design. conductor-dev commits the change and sends `[REPORT conductor -] progress rules.md
<commit> items <n>,…` with the diff; conductor reads the changed items with `conductor rules show
--item <n>,…` and relays them to every running controller. Restarts are batched with the next
wave, at most one an hour unless one corrects a live fault. A start reads little: conductor's
profile, its own section of `rules.md` through `conductor rules show --section conductor`, §§ 4–8
of this design, the north star and status, the tail of its logs and the newest hand-over.

1. conductor-dev sends `[RESTART conductor] <reason>`.
2. conductor ends its cycle, writes a hand-over in `docs/handoff/` (open dispatches, pending
   requests, waits) and answers `[REPORT conductor -] ready <path>`.
3. conductor-dev runs `task conductor:restart`, which stops the background session and starts a
   new one that reads the hand-over first.

When a restart conductor needs (its context over `thresholds.context_handover`) cannot be answered
because conductor-dev is unreachable, conductor writes its hand-over and runs `task
conductor:restart` itself, from its records directory and detached from the session the task
stops.

Both sessions start in the background with `conductor session start`, in the box the instance's
`confinement` asks for. A session whose permission mode holds
incoming messages for approval stalls the loop, so both run in a mode that does not. The operator
reaches them with `task conductor:attach` and `task dev:attach`.

## 13. Risks

| risk | mitigation |
|---|---|
| conductor's context fills with every repository's traffic | the CLI aggregates; messages have fixed first lines and at most 20 lines; state lives in the store, so a restart loses nothing; the watch reports context above `context_handover` |
| conductor and the controllers share one usage quota | `controllers.max_working` and `controllers.max_subagents`; the usage view; the watch reports usage limits |
| a controller escapes through Bash | the guard heuristic, and snapshot detection of unexpected dirty files |
| a wrong class-C call | every decision has an id and its options; reversals are recorded; a reversed decision becomes a rule in a profile |
