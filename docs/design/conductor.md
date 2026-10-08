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
| repositories | `git` in each checkout, `gh` for each `github` source | one row per repository: visibility, archived, open issues, `main` head, real commits in 7 days, behind `main`, dirty files, worktrees, latest release, unreleased commits, planning-store version, and `in_catalog` when the instance names a catalog |
| github | `gh pr list`, `gh run list` and releases per repository | open pull requests, the newest `main` run per workflow, releases, merged pull requests |
| blockers | `git fetch`, an export of `origin/main`, `aep plan workspace list` | open `decision-blocker` artifacts across the AEP stores |
| specifications | the same exports, `ess specify validate`, `ess verify conform synthesize` | per repository: specification present, opted out or missing; valid or refused; scenario count |
| sessions | `claude agents --json` | live sessions, their cwd, the repository each is bound to |

A `gitlab` source is not read by the binary. The session reads that forge through its own
integration and records the rows with the `snapshot record-*` commands.

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

**Class O: the operator decides.** Conductor prepares the options and a recommendation, and
escalates (`decision escalate-request`).

- New, renamed, split, merged or retired repositories.
- Architecture-level changes: a contract between repositories, a noun crossing repositories, the
  direction or an objective.
- Public-facing breakage, data loss, outages.
- Security trade-offs outside an approved design.
- Credentials, accounts and spending.
- Deleting another session's or person's work.
- Changes to the operator's own rules.

**Class H: needs the operator's hands, not judgement.** Setting a secret, a login, a paid account,
a permission only the operator holds. Conductor keeps one numbered list, with the exact command or
page for each item and the controllers waiting on it (`decision hands-todo`). Conductor never
records a class-H decision: `record-conductor-decision` refuses it as `ReservedForOperator`. The
operator records what was done with `record-operator-decision`, class H.

An escalated request is answered only by the operator (`decision answer-escalated-request`).
Conductor answers an open request (`decision answer-request`). Conductor reverses only its own
decisions (`decision reverse-conductor-decision`); the operator reverses any
(`decision reverse-operator-decision`).

When a class is unclear, conductor treats it as class O and records why. That record is how the
boundary moves over time. A request already answered is answered again from the record, never
asked of the operator a second time.

Decision ids and dispatch ids carry the prefixes the specification gives them; a decision id also
carries its UTC date and a number. Given no `--decision-id`, the record commands allocate the next
id for the UTC date of `--decided-at`.

Who decides class C and class O is also written in the instance's `authority` (§ 9).

### Authority

Conductor decides class C on the operator's standing grant (`authority.class_c`). The instance's
`rules.md` says where that grant is written and how the operator decides, so conductor can pick the
option he would. Every class-C decision conductor sends carries its decision id; one without an id
is a recommendation.

The controller profile tells a controller to act on a `[DECISION …]` that carries an id as the
operator's, within conductor's grant. Where the operator's own harness instructions would make a
controller ask him instead, they need the same grant; without it, controllers ask the operator and
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
`blocked`, `unblocked`, `done`, `failed`. A `progress` report is a message only.

A decision request also lands in the controller's own planning store as a `decision-blocker`, where
the repository has one. The controller clears the blocker stating what was decided, with no
reference to conductor; repositories carry no pointer to conductor. Conductor records each request
it receives (`decision raise-decision-request`, with the blocker reference). The `blockers`
collector is the cross-check: an open blocker with no recorded request is a controller that did not
report.

A `[RESOURCE]` request is granted or refused against the instance's thresholds
(`resource grant-resource`, `refuse-resource`, `release-resource`).

## 7. The guard

| rule | enforced by |
|---|---|
| a controller's cwd is its checkout | the session is started there |
| a controller's Edit, Write, NotebookEdit only inside its repository's checkout and managed worktrees, or in its scratch | the guard |
| a controller's SendMessage only to `conductor` or to a sub-agent it started | the guard |
| no `gh` writes; writes to the forge go through the repository's own route | the guard |
| no `cd`, `pushd` or `git -C` into another repository or into conductor's records | the guard (a heuristic) |
| no write to conductor's records or conductor's config | the guard: file tools; Bash only for the config |
| no session writes a `.claude/` settings file (`settings*.json`, `controller-settings.json`, `conductor-settings.json`) or a role's `settings` file, by a file tool or a Bash write form that names it | the guard |
| no `conductor` command that writes, only its views | the guard |
| conductor's Edit, Write, NotebookEdit only to its records, or in its scratch | the guard |
| a build over `thresholds.build_size` while free disk is under `thresholds.build_slot` asks for a slot | the profile and `[RESOURCE]` |

Scratch is a session's own temporary space: under `$TMPDIR` for every session, and for a
controller also under `~/.cache/<repo>-*/`, `<repo>` its repository with `/` written `-`. Scratch
is never a repository's checkout or managed worktree, conductor's records or its config, wherever
`$TMPDIR` points, and a `$TMPDIR` that is the home directory or holds it is no scratch at all.

The guard is a Claude Code PreToolUse hook. The repository ships it wired in
`.claude/conductor-settings.json` (conductor) and `.claude/controller-settings.json` (each
controller, named by the controller role's `settings`), once per matcher (`Edit`, `Write`,
`NotebookEdit`, `SendMessage`, `Bash`):

```console
conductor guard record-guard-decision --from-pre-tool-use || exit 2
```

It reads the hook payload on standard input and decides by the payload's `cwd`:

- a cwd in a repository's checkout or managed worktrees runs under the controller rules, unless a
  source of the instance excludes that repository;
- a cwd in conductor's records (the instance's `records` when a config file names it) runs under
  conductor's rules, which allow edits to the records (`decisions/`, `dispatches/`, `charters/`,
  `docs/handoff/`, `goals.jsonl`, `STATUS.md`, `NORTHSTAR.md`) and to scratch, and deny `gh`
  writes;
- a cwd in no repository is denied every tool that has a rule.

Exit 0 allows the call. Exit 2 blocks it, with one line of reason on standard error. Anything that
fails before a verdict exists is a denial, and the `|| exit 2` covers a missing binary.

Only denials are recorded, in the store under `--state-dir`, else under the instance's `state` when
a config file names it, else nowhere: the hook runs in any session's working directory and must
not plant a store there. `guard guard-decisions` reads them back. An allowed call is answered
without opening the store, because every tool call of every guarded session runs the hook.

The Bash rule is a heuristic over the command line, and it does not check where a command writes.
It denies a `cd`, `pushd` or `git -C` into another checkout or the records, a `gh` write, a
`conductor` write, and a write form that names a settings file or conductor's config. Any other
Bash write passes: a controller can write, overwrite or delete any file its user can, other
checkouts, conductor's records and the `conductor` binary included. Each snapshot reports dirty
files in every checkout, which turns such a write into a finding. The guard catches mistakes; it
is no defence against a hostile prompt (README, Security model). Only the five tools above reach it, `git push` is left to the
forge's branch protection, and conductor-dev runs without it. An operating-system sandbox for
Bash is story `bash-sandbox`; routing every tool to the guard is story `guard-all-tools`.

## 8. The watch

`conductor watch run` runs cheap probes between cycles and prints one line per change. Conductor
runs it with `--follow` under the harness `Monitor` tool.

| probe | source | line |
|---|---|---|
| sessions | `claude agents --json`, sessions in the instance's checkouts, trees or records | `session exited (blocked): …`, `session gone: …` |
| usage limit | the tail of each such session's transcript | `usage limit: …`; also a desktop notification (`notify-send`) |
| context | the newest usage line of each live session's transcript | `context: <repo> <n>k tokens` above `thresholds.context_handover` |
| `main` CI | `gh run list` for each Active repository of the newest complete snapshot | `CI red on main: …`, `CI green again on main: …` |
| free disk | `df` on `/` | `disk low: <n>G free on /` under `thresholds.disk_low`, again only after it was back at `disk_clear` |

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
`exclude` paths under its root. An instance's `catalog` names a repository under the checkouts
root and a directory in it; the repositories collector reads it (§ 4).

The file never holds a secret. A value that looks like a token is refused by its path and never
printed.

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
`dispatches/YYYY-MM.jsonl`, `charters/<repo>.md` and `docs/handoff/`. The CLI does not write them
yet.

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
| `controller` | start, pause, resume, stop a controller; `revise-charter` |
| `dispatch` | send, cancel and report on dispatches |
| `message` | receive, handle, reject and route messages |
| `resource` | request, grant, refuse and release resources; `usage` (token spend from transcripts) |
| `guard` | the hook and its verdicts |
| `config` | `validate`, `show` |
| `dashboard` | `serve`: a read-only page on 127.0.0.1 (default port 7313) |
| `store` | `migrate` |
| `watch` | `run` |

Not implemented yet; each answers exit 2:

- `snapshot publish-board` and `snapshot boards` (the generated `STATUS.md`);
- `--input-json -` on every command that offers it;
- `guard record-guard-decision` without `--from-pre-tool-use`;
- starting and ending the controller's session: `controller start-controller` and
  `stop-controller` record the controller only. Conductor starts the session itself, with the
  controller role's model, agent and settings.

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
| conductor | its records: `decisions/`, `dispatches/`, `charters/`, `docs/handoff/`, `goals.jsonl`, `STATUS.md`, `NORTHSTAR.md` |
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

**Restart.** Only when something conductor read at session start changed: its adapter, its
profile, the controller profile, the instance's `rules.md`, this design, or the operator's harness
instructions.

1. conductor-dev sends `[RESTART conductor] <reason>`.
2. conductor ends its cycle, writes a hand-over in `docs/handoff/` (open dispatches, pending
   requests, waits) and answers `[REPORT conductor -] ready <path>`.
3. conductor-dev runs `task conductor:restart`, which stops the background session and starts a
   new one that reads the hand-over first.

Both sessions run in the background (`claude --bg`). A session whose permission mode holds
incoming messages for approval stalls the loop, so both run in a mode that does not. The operator
reaches them with `task conductor:attach` and `task dev:attach`.

## 13. Risks

| risk | mitigation |
|---|---|
| conductor's context fills with every repository's traffic | the CLI aggregates; messages have fixed first lines and at most 20 lines; state lives in the store, so a restart loses nothing; the watch reports context above `context_handover` |
| conductor and the controllers share one usage quota | `controllers.max_working` and `controllers.max_subagents`; the usage view; the watch reports usage limits |
| a controller escapes through Bash | the guard heuristic, and snapshot detection of unexpected dirty files |
| a wrong class-C call | every decision has an id and its options; reversals are recorded; a reversed decision becomes a rule in a profile |
