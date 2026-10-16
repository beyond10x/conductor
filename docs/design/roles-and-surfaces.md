# Design: levels, roles and tool surfaces

Status: approved by the operator, 2026-10-16 ("regarding PR64, I agree with everything how you phrased it, framed it"). § 8 is answered as recommended: the names, the bridge as a session, `tool`, MCP with Bash denied, one maintainer per host.
It replaces the draft `agent-tools.md` (sections 3 to 4 of that draft become section 6 here) and does not depend on any concept from other projects.

## 1. What went wrong

Each point is measured on instance 1 between 2026-10-14 and 2026-10-16.

1. **Nothing sits above the instances.**
   - Each instance is a separate config file with its own `conductor` session.
   - The operator reaches an instance by finding its session.
   - Nothing reads across instances, so there are no metrics across instances and no single place to ask.
2. **conductor-dev is configured per instance** (`instances[0].roles[1]` in the instance's config). But it develops one product for every instance.
3. **Roles are overloaded and their names are confusing.**
   - "conductor" names the product, its CLI and one session.
   - The conductor profile is 393 lines and the controller profile 170. `rules.md` holds 143 items, most of them corrections of a role doing the wrong thing.
   - A controller plans, dispatches, reviews and releases. Its workers are generic sub-agents with the full tool set.
4. **The CLI is one mixed surface.**
   - `conductor --help` lists 26 top-level commands. Some are for the operator (`init`, `setup`, `config`, `trust`, `upgrade`, `doctor`, `dashboard`, `gc`), some for sessions (`message`, `dispatch`, `decision`, `build run`), and some for both.
   - Every session that can run `conductor` can run all of it, `init` and `config` included.
5. **Sessions run arbitrary Bash.**
   - In 48 h, 96 % of 24,647 tool calls' Bash calls were compound command lines.
   - The guard refused 212, and 78 of those were retried in the same form.
   - A sandbox does not help while a session holds the `conductor` binary and forge credentials. Through those, it acts outside the sandbox.

## 2. Levels and names

Four levels, each with one job. The names are proposals; the operator decides them (question 1).

| level | name (proposal) | today | one per | job | talks to |
|---|---|---|---|---|---|
| host | **bridge** | none | host | routes the operator's requests to an instance; shows every instance's state and metrics; starts and stops instances | the operator; each instance's director |
| host | **maintainer** | conductor-dev | host | develops conductor itself: code, spec, profiles, release, install | the operator; the bridge |
| instance | **director** | the `conductor` session | organization | north star, the plan across repositories, decisions within its grant, dispatch to leads | the bridge; its leads |
| repository | **lead** | repo-controller | repository | orchestrates only: picks work, starts workers with a grant, merges what passes review, escalates | its director; its workers |
| unit | **planner, implementer, reviewer, releaser** | generic sub-agents | unit of work | one job each; see section 3 | its lead; its team's channel |

Rules of the levels:
- **A message goes one level up or down.** A worker never messages a director, and a lead never messages the bridge. The only sideways channel is a team's, inside one repository (section 6).
- **The operator talks to the bridge by default and may address a director directly.** Section 4 covers how.
- **The maintainer is not part of any instance.** It reads every instance's records and transcripts as product evidence. What it writes into the product never names an instance (rules today, enforced by the scan).

Alternatives the operator asked about, for the lead: "product manager" suggests it owns direction, which stays with the director and the operator; "dispatcher" covers only half of its job. **Lead** says it leads a team in one repository. Alternatives for the bridge: "switchboard", "front desk".

## 3. Roles available by default

Every instance ships these worker roles. A lead starts one per unit.

| role | does | its default tools (section 6) | model tier |
|---|---|---|---|
| planner | scopes, decomposes and specifies; writes the planning store and `spec/` | read, search, plan, spec, note, ask | opus |
| implementer | failing test first, then the change, in its own tree | read, search, edit (harness), test, check, commit, note, ask | opus or sonnet by unit size |
| reviewer | adversarial and security review of a finished change; writes failing tests only | read, search, edit of test files, test, note, ask | opus |
| releaser | version, changelog, tag, Release, CI watch | read, release, pr, ci, note, ask | sonnet |

- **The lead itself codes nothing.** Its tools are read, search, plan read, `agent run`, merge, pr, ci and messages.
- **The director's tools** are the instance's records, decisions, dispatch, metrics and messages. It has no repository tools.

## 4. The bridge: one channel to every instance

- **Where the operator talks:** a channel the operator types into, from the terminal (`conductor ask "<text>"`) or from one input on the dashboard. The bridge runs as one session per host.
- **Routing:**
  - The bridge names its route in its first line: `→ <instance>/<repository>: <one line>`. A repository name or an instance's goal decides the route.
  - An ambiguous request is not guessed: the bridge asks one question with options.
- **Direct:** `conductor ask --to <instance> "<text>"`, or `@<instance>` as the first word, goes to that director with no routing. Every answer comes back through the same channel, labelled with its instance.
- **Across instances:**
  - `conductor status` and the dashboard's home page show every instance: health, open decisions, waves, KPIs and spend.
  - A decision asked by any director appears in one list, so the click-to-decide page covers all instances.
- **What the bridge never does:** it never plans for an instance, decides for one, or carries one instance's data into another. A request that needs two instances becomes one request to each, and the bridge joins the answers.

## 5. Instance isolation, re-reviewed

### 5a. What is shared today

From an inventory of every runtime path the binary, the Taskfile and the profiles construct (2026-10-16; file:line under `crates/conductor/src/`).

**Per instance today, and correct:**
- records, state (store, lease, watch, session settings), cache and trees (config.rs:2047-2054).

**Shared across instances, and wrong:**

| # | what | where | effect |
|---|---|---|---|
| I1 | session names without a prefix | config.rs:1188; confine/mod.rs:789 | a bare `conductor` in one config file matches the other file's session: a start is refused, or the wrong session is stopped |
| I2 | messages addressed by bare name | `repo-controller.md:82`; dashboard.rs:1419 | nothing checks that the receiving session belongs to the sender's instance |
| I3 | systemd units and slices named by prefix or session name only | Taskfile.yml:275; confine/systemd.rs:92, 110 | two bare instances share `conductor-dashboard`, and the second dashboard is silently not started |
| I4 | dashboard port by index within its file | config.rs:1115 | every file's first instance gets 7313 |
| I5 | `state` and `records` uniqueness not validated | config validation | two instances can point at one state directory |
| I6 | the host ledger beside the config file; `host.fall` names one instance | config.rs:991; host.rs:88 | one instance's watch can clear another's fall |
| I7 | profiles in `<config dir>/profiles` | Taskfile.yml:328 | every instance reads the one copy the last install wrote |
| I8 | `~/.claude.json` trust and `~/.claude` writable from the box | trust.rs:383; confine/places.rs:90 | any session can change another instance's trust |
| I9 | transcripts and the session list host-wide | usage.rs:83; collect/sessions.rs:205 | a foreign file's sessions are invisible to the prefix check, and shared cwds share transcripts |
| I10 | instance selection falls back to the file's `default` | config.rs:549 | a command run outside every instance's tree silently uses the default instance |
| I11 | default settings checkout-local; the pinned copy keyed by session name | session.rs:451, 590 | the same settings serve every instance |
| I12 | reclaim deletes shared caches and archives; scratch `~/.cache/<repo>-*` keyed by repository only | gc/reclaim.rs:104; places.rs:77 | two instances with the same repository name share scratch |
| I13 | one installed binary | upgrade.rs:235 | an upgrade switches every instance at once, with no pin per instance |

### 5b. The rules

- **Per instance:** records, state, cache, trees, dashboard port, session names, systemd units and transcripts.
  - Each session name carries the instance's prefix, so a bare name never reaches another instance's session.
  - Each instance has its own `CLAUDE_CONFIG_DIR`, so trust, transcripts and harness state are no longer shared through `~/.claude.json` and `~/.claude/projects`.
- **Per host, shared by design:** the host ledger (disk and CPU slots), the bridge and the maintainer, the installed binaries and the profiles of one release.
- **Nothing falls back to a default instance at runtime.** A session runs pinned to its instance (0.15.0); I10's fallback stays only for the operator's `conductor`. The broker of section 6 refuses a call whose session does not belong to the instance it names.
- **A host registry closes I1 to I5, I7 and I9.** The bridge keeps one registry per host of every instance across config files. Each instance gets a unique prefix (required, never bare), a unique port, its own state and records, and profiles copied per instance. Validation refuses a collision across files, not only within one file.
- **I8 and I11:** each instance gets its own harness configuration directory, and the box never makes another instance's harness state writable.
- **I6:** a fall is recorded per instance, and only the instance that owns it clears it.
- **I12:** scratch is keyed by instance and repository.
- **I13:** a binary per release lives at `<host>/bin/<version>`, and each instance pins its version. The bridge upgrades one instance at a time.

## 6. Tool surfaces

### 6.1 Two binaries

| binary | for | holds |
|---|---|---|
| `conductor` | the operator and the bridge | instances, config, setup, trust, upgrade, doctor, dashboard, metrics across instances, gc, host ledger |
| `tool` (name open, question 3) | every session of an instance | only the verbs of 6.3; no admin command is compiled in |

- **The split is by binary, not by subcommand.** A top-level split inside one binary (`conductor admin …` against `conductor agent …`) still puts the admin code in every workspace, and a worker can call it.
- **`tool` is a thin client.** It sends each call to the instance's **broker** over a socket mounted into the session's workspace.
- **The broker holds what the session must not have:** the forge credentials, the bot route, write access to the store, and the session's grant.

### 6.2 The broker and grants

- **Grant:** every session starts with one, a list of tools with their limits: repositories, paths, budgets and time to live.
- **Identity:** the broker identifies the caller by its session (the socket is the session's), not by a token the caller could pass on.
- **Checks:** each call is checked against the grant, run by the broker, and recorded. A refusal names the grant rule it broke.
- **No Bash:**
  - Sessions get the harness's file tools (Read, Edit, Write, Grep, Glob) and the `tool` surface, and nothing else.
  - On Claude Code the surface is offered as an MCP server backed by the broker, so no session needs Bash at all.
  - The CLI form serves harnesses without MCP. On those, Bash is allowed only for `tool *`, as plain argv.

### 6.3 Verbs

From the 48 h analysis (epic:agent-tools); every verb prints one line on success, and on failure at most 40 lines plus a log reference.

| group | verbs |
|---|---|
| read | `show <path> [--lines a:b \| --fn s] [--max n]` (bounded; "unchanged" on a re-read), `search <pattern> [--path p]`, `diff [--stat]` |
| build | `test [<crate>] [--filter f]`, `check`, `fmt`: through the host ledger, nice 19 |
| version control | `commit -m <msg> -- <paths>`, `branch`, `merge <branch>` (lead), `push` (through the gates) |
| forge | `pr open\|checks\|comment`, `ci rerun`, `release` (releaser) |
| plan | `plan show\|draft\|move` (planner; moves only for the lead) |
| team | `send <role-or-name> "<text>"` within the team; `report <status> "<line>"` to the lead; `escalate "<question>" --options …` to the lead, who raises it to the director |
| memory | `memo put <key> <text> [--ttl 4h]`, `memo get\|list`: team-scoped scratch that vanishes after its time to live |
| wait | `wait --ci <pr> \| --agent <id> \| --event <name> [--deadline d]`: resolves without a turn per poll |
| agents (lead) | `agent run --role <r> --brief <text> --tools <list> [--model m]` |
| tools | `tool define <name> '<steps>'`, `tool list`, `tool describe <name>` |

### 6.4 Grants compiled by the lead

- **`agent run` starts a worker with the tools the lead names.**
- **The broker grants the named tools minus anything outside the lead's own grant.** A child never gets more than its parent, so a grant only narrows.
- **A role's default tools (section 3) apply when the lead names none.**
- **A worker can `tool list` and `tool describe`** to explore what it was given.

### 6.5 `tool define`: aliases

- **An alias is a named sequence of verbs the caller already holds,** with parameters:

      tool define green-commit '{seq: [{test: {filter: $1}}, {commit: {m: $2, paths: $3}}]}'
      tool green-commit build_run "fix: …" crates/a.rs

- **Steps:**
  - Steps are verbs only, never a shell string.
  - `seq` stops at the first failure; `par` runs in parallel and reports every result.
  - The broker expands an alias and checks each step against the grant of whoever calls it, at call time. An alias never carries its author's rights.
- **Scope:**
  - An alias belongs to its session by default.
  - The lead may publish it to the repository's team, and the director to the instance.
  - Unused aliases expire.
- **Use is recorded.** An alias used often across instances becomes a draft story for a built-in verb, which closes the loop with the maintainer.

## 7. Migration, each step reversible

0. **Isolation fixes that need no new level,** done first: required unique prefixes, the collision check across files (I1 to I5), the per-instance fall (I6), scratch keyed by instance (I12).
1. **Names and levels:** rename the profiles and config roles, and add the bridge session as a profile over the existing CLI. Old names stay as aliases for one release.
2. **The bridge:** `conductor ask` and the dashboard home across instances (a registry of config files per host).
3. **The broker with `tool`:** it starts with read, test, commit, send and report. Implementers move to it first, with Bash denied. Measured before and after: context per turn, round trips, refusals.
4. **The remaining verbs and grants by `agent run`;** the lead moves to `tool`.
5. **Aliases.**
6. **The `conductor` binary loses its session commands,** once no session calls them.

## 8. Open questions for the operator

1. The names: bridge, maintainer, director, lead, planner, implementer, reviewer, releaser? (recommended)
2. Is the bridge a session (one more model in the loop, but it understands free text), or only the CLI and dashboard with explicit `--to`? (recommended: a session, with `--to` always available)
3. The agent binary's name: `tool`, `crew` or `kit`? (recommended: `tool`)
4. MCP as the surface on Claude Code, with Bash denied for every worker? (recommended)
5. Should the maintainer stay one per host and read every instance's records and transcripts? (recommended; the alternative is one per instance with no view across them)
