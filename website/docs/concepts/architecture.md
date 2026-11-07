---
title: Architecture, today and next
sidebar_position: 7
description: How a conductor host is built today (units, one tmux server per session, the harness's agent list, the Taskfile start tasks, the operator's chat session, attach) and the design of the next architecture (a TUI over tmux, one controller and workers per instance, one supervisor across instances, no separate maintainer, global and instance settings, built-in and custom roles, self-improvement over settings).
lede: Today's host is three mechanisms side by side; the next one is one, a session started by conductor from its config, with a TUI to move between them.
source: docs/design/conductor.md sections 2, 3, 4, 9 and 12; docs/design/conductor-run.md; docs/design/roles-and-surfaces.md; docs/design/self-improvement.md; Taskfile.yml; crates/conductor/src/session; the operator's design request of 2026-11-07
---

Colours in the diagrams: blue is a host role, green a session with a model, grey a process without
one, amber a custom role or a decision, violet a store or a tree, pink a command. Every diagram has
a plain-text version under it.

## Status quo

### The host

```mermaid
flowchart TB
  classDef host fill:#dbeafe,stroke:#1d4ed8,color:#111827
  classDef model fill:#dcfce7,stroke:#15803d,color:#111827
  classDef nomodel fill:#e5e7eb,stroke:#4b5563,color:#111827
  classDef store fill:#ede9fe,stroke:#6d28d9,color:#111827
  op["operator: interactive harness session"]:::host
  sup["supervisor: claude --bg, reads profiles/conductor-supervisor.md"]:::host
  tea["teacher: claude --bg, reads profiles/conductor-teacher.md"]:::host
  op --- sup
  op -.-|names foreign sessions, answers his class| sup
  tea -.->|observes, edits the profile| sup
  subgraph slice["conductor_‹instance›.slice, one per instance"]
    run["‹instance›-run.service: conductor run, no model"]:::nomodel
    dash["‹instance›-dashboard.service: 127.0.0.1:‹dashboard_port›"]:::nomodel
    s1["‹prefix›-‹repo A›.service: tmux server, controller"]:::model
    s2["‹prefix›-‹repo B›.service: tmux server, controller"]:::model
    dev["‹prefix›-conductor-dev.service: tmux server, maintainer"]:::model
  end
  style slice fill:#f8fafc,stroke:#64748b,color:#111827
  sup -->|restarts, delivers, reads journals| run
  sup -->|defects, stories| dev
  run -->|spawn, dispatch, hand-over| s1
  run -->|spawn, dispatch, hand-over| s2
  s1 --> t1[("tree of repository A")]:::store
  s2 --> t2[("tree of repository B")]:::store
  dev --> t3[("tree of the conductor repository")]:::store
  run --> store[("records, state, cache")]:::store
  dash --> store
  s1 -->|reports, requests| store
```

<details>
<summary>plain text</summary>

```text
operator (chat session) ──── supervisor (claude --bg) ◄···· teacher (claude --bg)
                                  │ restarts, delivers, reads journals; defects to conductor-dev
  ┌─ conductor_<instance>.slice, one per instance ───────────────────────────────────┐
  │ <instance>-run.service         conductor run, no model                           │
  │ <instance>-dashboard.service   127.0.0.1:<dashboard_port>                        │
  │ <prefix>-<repo A>.service      tmux server ── controller ── tree of repository A │
  │ <prefix>-<repo B>.service      tmux server ── controller ── tree of repository B │
  │ <prefix>-conductor-dev.service tmux server ── maintainer ── conductor tree       │
  │                                                                                  │
  │ run ──spawn, dispatch, hand-over──► controllers ──reports, requests──┐           │
  │ run, dashboard ◄──────────────────────────────────► records, state, cache ◄──────┘           │
  └──────────────────────────────────────────────────────────────────────────────────┘
```

</details>

### Levels

| level | one per | what runs there | started by |
|---|---|---|---|
| host | machine | the supervisor session, the teacher session, the maintainer session (`conductor-dev`), the operator's own chat session | `task supervisor:start`, `task teacher:start`, `task maintainer:start`; the chat session by the operator |
| instance | organization | the loop (`conductor run`), the dashboard, the instance's controllers | `conductor run --install`, `task dashboard`, the loop |
| repository | repository of an instance | one controller session, in its own tree | the loop, through `conductor spawn` |
| unit of work | a story or task | the controller's sub-agents (planner, implementer, reviewer) | the controller, inside its own harness session |

The loop runs no model: it takes snapshots, picks work, sends dispatches, spawns controllers and asks
for hand-overs at the context threshold. Every judgement runs in a session with a model.

```mermaid
flowchart TB
  classDef nomodel fill:#e5e7eb,stroke:#4b5563,color:#111827
  classDef model fill:#dcfce7,stroke:#15803d,color:#111827
  classDef store fill:#ede9fe,stroke:#6d28d9,color:#111827
  watch["watch: every cadence.watch, and on a controller report"]:::nomodel --> sense["sense: one snapshot"]:::nomodel
  sense --> compare["compare: goals, servings, red signals"]:::nomodel
  compare --> decide["decide: answer requests, pick next work"]:::model
  decide --> dispatch["dispatch: start, steer or stop a controller"]:::nomodel
  dispatch --> record["record: one record per decision, dispatch, message"]:::store
  record --> report["report: the brief, deviations only"]:::nomodel
  report -.->|every cadence.cycle| sense
```

<details>
<summary>plain text</summary>

```text
sense     conductor snapshot start-snapshot   one snapshot: repositories, pull requests, main CI,
                                              releases, planning blockers, specifications, sessions
compare   goals and servings                  which goal each repository's work serves; idle goals;
                                              work serving no goal; red signals
decide    decision requests                   answer controller requests; pick next work per repository
dispatch  conductor dispatch send-dispatch    start, steer or stop a controller; one charter each
record    the store                           one record per decision, dispatch and message
report    operator brief                      once a day and on escalation, deviations only
          ◄── every cadence.cycle, and on a controller report, back to sense
```

</details>

### Processes and units

Each instance is one systemd user slice, `conductor_<instance>.slice`, and every long-running
process of the instance is one user unit inside it:

| unit | runs | model |
|---|---|---|
| `conductor_<instance>-run.service` | `conductor run`: the loop | no |
| `conductor_<instance>-dashboard.service` | the dashboard on `127.0.0.1:<dashboard_port>` | no |
| `conductor_<session>.service` | one tmux server, `tmux -D -S <state>/sessions/<session>.tmux`, whose only window runs the harness of a controller or the maintainer | yes |

#### One session, one tmux server

```mermaid
flowchart LR
  classDef model fill:#dcfce7,stroke:#15803d,color:#111827
  classDef nomodel fill:#e5e7eb,stroke:#4b5563,color:#111827
  classDef store fill:#ede9fe,stroke:#6d28d9,color:#111827
  classDef cmd fill:#fce7f3,stroke:#be185d,color:#111827
  subgraph unit["conductor_‹session›.service, in the instance's slice"]
    tmux["tmux -D -S ‹state›/sessions/‹session›.tmux -f ‹session›.tmux.conf"]:::nomodel
    harness["the harness, started from the role's profile"]:::model
    tmux -->|one window| harness
  end
  style unit fill:#f8fafc,stroke:#64748b,color:#111827
  files["‹session›.settings.json · ‹session›.envelope.json · ‹session›.typescript"]:::store
  tmux --- files
  attach["conductor session attach --name ‹session›"]:::cmd -->|opens the socket| tmux
  deliver["conductor session deliver --name ‹session›"]:::cmd -->|types a text, then Enter| tmux
  stop["systemctl --user stop conductor_‹session›.service"]:::cmd -->|ends this session only| unit
  watch["conductor session attach --watch"]:::cmd -->|follows| files
```

<details>
<summary>plain text</summary>

```text
conductor session attach --name S  ──opens the socket──┐
conductor session deliver --name S ──types a text──────┤
                                                       ▼
conductor_S.service:  tmux -D -S <state>/sessions/S.tmux -f S.tmux.conf
                        └─ one window: the harness, started from the role's profile
files beside the socket:  S.settings.json   S.envelope.json   S.typescript ◄── attach --watch
systemctl --user stop conductor_S.service   ends this session and nothing else
```

</details>

Why one server per session:

- A session is one unit: `systemctl --user stop conductor_<session>.service` ends that session and
  nothing else; its cgroup, cpu weight and tasks limit are measured and written to
  `<state>/sessions/<session>.envelope.json` (the box the instance's confinement asked for).
- The socket is the address: `conductor session attach --name <session>` and
  `conductor session deliver --name <session>` open that socket; nothing is shared between two
  sessions and a crashed server takes no other session down.
- The tmux config is per session too (`<session>.tmux.conf`: `exit-empty on`, prefix `C-]`, status
  off, `-x 160 -y 50`), so a session's terminal size and keys cannot be changed from another one.
- Everything typed and printed is kept: `<session>.typescript` is the record a watcher reads, and a
  session started before the tmux launcher can still be followed through it (`--watch`).
- A session's settings file (`<session>.settings.json`) is written at start from the role's
  `settings` and its `read_only_tools`, so the harness reads a file conductor owns, not a shared one.

### Three ways to see the sessions

| view | command | shows |
|---|---|---|
| harness | `claude agents` (`--json` for a script) | every background session of the harness on this host, any directory, any owner: the controllers, the maintainer, the supervisor, the teacher, and sessions nothing of conductor started |
| conductor | `conductor session list` | one line per live session of every instance whose state holds a launcher's files for it: instance, name, repository, attach or watch, state, seconds since the last line |
| dashboard | `http://127.0.0.1:<dashboard_port>` | the instance's repositories, dispatches, decisions, metrics and the plan board |

The harness view is the superset: a session the loop lost (a controller whose unit died, a session
started by hand with `claude --bg`) is in it and not in `conductor session list`. The supervisor
reconciles the two every pass.

### Starting things: the Taskfile

| task | does |
|---|---|
| `task install` | builds and installs the binary; `task profiles` installs `.agents/*.md` into `profiles/` next to the config file |
| `task maintainer:start` | `conductor session start --role maintainer`: the maintainer in its own tmux server and unit |
| `task supervisor:start` | `claude --bg --agent conductor-supervisor -n conductor-supervisor …` in the config directory |
| `task teacher:start` | the same for the teacher |
| `task dashboard`, `task dashboard:stop` | the dashboard unit |
| `task sessions`, `task conductor:attach`, `task maintainer:attach` | list sessions; attach to one |
| `task agents:link REPO=<path>` | links the role adapters into a repository's `.claude/agents/` |

The supervisor and the teacher are not units: they are the harness's own background sessions, read
their 10-line adapter from `~/.claude/agents/<name>.md` and their whole definition from
`profiles/<name>.md`. They have no tmux server, no envelope, no typescript, and `conductor session
list` does not show them.

### The operator's chat session

The operator works in an ordinary interactive harness session. It is not a role: it talks to the
supervisor (and, when he names one, to the teacher), and it is the place where decisions of his class
are given. A foreign session the operator names (a session he runs by hand in a repository of an
instance) is treated by the supervisor as that repository's owner: the loop starts no controller for
it, and the supervisor reads its MRs and pipelines instead.

### Attaching

```text
conductor session attach                  picker of every instance's sessions: Enter attaches, w watches, q quits
conductor session attach --name <session> one session, read-write
conductor session attach --role maintainer
conductor session attach --watch --name … read-only; also for a session started before the tmux launcher
ctrl-] then d                             detach; the session runs on
```

`claude agents` has no attach: a harness-only session (the supervisor, the teacher) is reached by
message or by `claude logs <id>`.

### What the status quo costs

| friction | measured on 2026-11-07 |
|---|---|
| three start mechanisms (unit, `claude --bg`, hand) | two sessions of one name (`<name>-2`) and a stale resumed id in one day; the rename needs `claude stop` and `claude --bg --resume` |
| the maintainer is a special role in config (`conductor-dev`) and in the host section | its hand-over left 52 local-only commits on 7 branches; no loop rule covered it until wave 45 |
| the supervisor is outside the config | its rules lived in a file under `~/.claude/agents/` until they became a shipped profile; four host scripts were written beside it |
| settings are per instance only | the same thresholds are copied into three instances; a host-wide change is three edits |
| one controller per repository | an instance with 6 active repositories runs 6 model sessions and 6 trees whether or not they have work |
| no custom role | a recurring job (a UI pass every day) has no place but a controller's charter |

## Next architecture

The design the operator asked for on 2026-11-07. Each change is a story for the maintainer; the
migration at the end orders them so that every step is reversible.

### One picture

The host and its instances:

```mermaid
flowchart TB
  classDef host fill:#dbeafe,stroke:#1d4ed8,color:#111827
  classDef model fill:#dcfce7,stroke:#15803d,color:#111827
  classDef cmd fill:#fce7f3,stroke:#be185d,color:#111827
  op["operator"]:::host
  tea["teacher, host, one: outside the system, reviews it"]:::host
  sup["supervisor, host, one"]:::host
  op --- tea
  op --- sup
  tea -.->|reviews decisions and outcomes, edits the profile| sup
  sup --> a["instance A: controller, workers, loop"]:::model
  sup --> b["instance B: controller, workers, loop; holds the conductor repository"]:::model
  sup --> c["instance C: controller, workers, loop"]:::model
  b -.->|tag, Release, announcement| si["conductor self-install ‹tag›, under the supervisor's grant: every loop, one instance at a time"]:::cmd
```

Inside one instance:

```mermaid
flowchart TB
  classDef model fill:#dcfce7,stroke:#15803d,color:#111827
  classDef nomodel fill:#e5e7eb,stroke:#4b5563,color:#111827
  classDef custom fill:#fef3c7,stroke:#b45309,color:#111827
  classDef store fill:#ede9fe,stroke:#6d28d9,color:#111827
  la["loop and dashboard, no model"]:::nomodel
  ca["controller, one per instance"]:::model
  la -.->|snapshots, schedules, hand-overs| ca
  ca --> w1["worker: implementer, repository 1"]:::model
  ca --> w2["worker: reviewer, repository 1"]:::model
  ca --> w3["worker: releaser, repository 2"]:::model
  ca --> x["custom role: ui-pass, daily 06:00"]:::custom
  w1 --> st[("records, state")]:::store
  w2 --> st
  w3 --> st
  x --> st
  st -.->|what the loop snapshots| la
```

<details>
<summary>plain text</summary>

```text
operator ── chat session ── supervisor (host, one)
                 │             │
              teacher (host, one; outside the system, reviews the supervisor and the outcomes)
                               │
            ┌──────────────────┼──────────────────┐
         instance A         instance B         instance C
            │                  │                  │
         controller         controller         controller        one per instance
         ├ worker            ├ worker            ├ worker         per repository or unit
         ├ worker            ├ worker            └ custom role    on a schedule
         └ custom role       └ worker (releaser) ──► conductor self-install <tag>
            │
         loop (no model), dashboard, records, state
```

</details>

| level | one per | role | what it does |
|---|---|---|---|
| host | machine | **teacher** | outside the system, above the supervisor: reviews what the supervisor decided and what the instances delivered, the way an adversarial reviewer reads a finished change; changes the supervisor's profile and files findings; never runs the supervisor's routine itself (operator, 2026-11-07) |
| host | machine | **supervisor** | keeps every instance's loop, controller and workers running; carries findings across instances; decides within its grant; reports to the operator |
| instance | organization | **controller** | the instance's judgement: picks work across the instance's repositories, starts and steers workers, answers requests, merges what passes, escalates |
| instance | organization | **loop** | `conductor run`: snapshots, timers, spawns, hand-overs, KPIs; no model |
| repository or unit | work | **worker** | one job in one tree: plan, implement, review, release; built-in roles |
| any | schedule | **custom role** | a role the config adds: a profile, a scope and a schedule; conductor starts it like any other |

The star stays: a worker talks to its controller, a controller to the supervisor, the supervisor to
the operator. A controller of one instance never messages another instance's sessions.

### No separate maintainer

conductor's own repository is a repository in one instance's `sources`. Its controller plans and
delivers conductor changes like any other repository's. What was special about `conductor-dev` becomes
two things:

- a built-in worker role `releaser` (version, changelog, tag, forge Release, announcement) that any
  repository can use;
- one host step, `conductor self-install <tag>` under the supervisor's grant: install the released
  binary, run `conductor doctor`, restart the loops one instance at a time, and roll back to the
  previous binary when a loop does not come up. The supervisor runs it after a release; the
  operator sees it in the daily brief.

The findings a session files about conductor (`conductor finding file-finding`) go to that
repository's planning store, as today.

### Every session is started by conductor

One start form for every role: `conductor session start --instance <i> --role <role>`. It writes the
settings file, starts one tmux server in the instance's slice (the host roles in `conductor_host.slice`),
measures the box and writes the envelope. `claude --bg`, `task supervisor:start` and the adapters
under `~/.claude/agents/` go away; the harness view (`claude agents`) stays a diagnostic.

```mermaid
sequenceDiagram
  participant L as caller
  participant C as session start
  participant S as systemd
  participant T as tmux
  participant H as harness
  L->>C: --instance i --role r
  rect rgb(252, 231, 243)
    C->>C: write settings.json, tmux.conf
  end
  rect rgb(229, 231, 235)
    C->>S: start the unit in the slice
    S->>T: tmux -D -S ‹session›.tmux
  end
  rect rgb(220, 252, 231)
    T->>H: one window, the role's profile
  end
  rect rgb(237, 233, 254)
    C->>C: measure, write envelope.json
  end
  C-->>L: name and socket
  H-->>L: reports, requests, hand-over
```

<details>
<summary>plain text</summary>

```text
loop, TUI or schedule ──► conductor session start --instance i --role r
                            1  write <session>.settings.json and <session>.tmux.conf
                            2  systemd: start conductor_<session>.service in conductor_<i>.slice
                            3  tmux -D -S <state>/sessions/<session>.tmux
                            4  one window: the harness with the role's profile
                            5  measure the box, write <session>.envelope.json
                      ◄──── session name and socket
harness ──► reports, requests, hand-over at the threshold
```

</details>

Rules carried over unchanged: one session per name, a successor only after its predecessor stopped;
hand-over at the context threshold with pushed branches; the typescript as the record.

### The TUI

`conductor` with no arguments at a terminal (and `conductor tui`) opens a full-screen view built on
tmux: the TUI itself is one tmux client, and entering a session switches that client to the session's
socket, so the session keeps running untouched when the operator leaves.

```text
┌ instances ─────────┬ sessions of <instance> ──────────────┬ settings ───────────────────┐
│ > A   2 dec  K1 ok │ controller   busy  412k  wave/46     │ cadence.cycle        2h     │
│   B   0 dec  K3 !  │ worker repo1 idle   38k  story:x     │ thresholds.disk_low  200G   │
│   C   1 dec  K1 ok │ worker repo2 busy  120k  story:y     │ controllers.max_working 8   │
│   host             │ ui-pass      sched 06:00 daily       │ roles[3].schedule    daily  │
├────────────────────┴──────────────────────────────────────┴─────────────────────────────┤
│ Enter attach  w watch  d deliver  s settings  r restart  l log  k kpis  ? keys  q quit    │
└───────────────────────────────────────────────────────────────────────────────────────────┘
```

```mermaid
flowchart LR
  classDef host fill:#dbeafe,stroke:#1d4ed8,color:#111827
  classDef model fill:#dcfce7,stroke:#15803d,color:#111827
  classDef store fill:#ede9fe,stroke:#6d28d9,color:#111827
  classDef cmd fill:#fce7f3,stroke:#be185d,color:#111827
  term["terminal"]:::host --> tui["conductor tui: one tmux client"]:::cmd
  tui -->|Enter: switch the client to the socket| sa["session socket, instance A"]:::model
  tui -->|Enter| sb["session socket, instance B"]:::model
  sa -.->|detach: back to the TUI| tui
  tui -->|s: validate, then write| cfg["conductor.yaml"]:::store
  tui -->|d, r, k: conductor commands| store[("the store: deliveries, restarts, readings")]:::store
  store -.->|what the dashboard reads| tui
```

<details>
<summary>plain text</summary>

```text
terminal ── conductor tui (one tmux client)
              ├─ Enter ──► session socket, instance A ── ctrl-] then d ──► back to the TUI
              ├─ Enter ──► session socket, instance B
              ├─ s     ──► validate, then write conductor.yaml
              └─ d r k ──► conductor commands ──► the store ◄── read by the TUI and the dashboard
```

</details>

| key | does |
|---|---|
| `Enter` | attach the selected session (`ctrl-]` then `d` returns to the TUI) |
| `w` | watch it read-only |
| `d` | deliver a text to it (recorded like `conductor session deliver`) |
| `s` | edit the selected instance's or role's settings; the TUI validates and writes the config file, and shows the effective value with the level it comes from |
| `r` | restart the session (hand-over first, then stop, then start) |
| `l` | its typescript, following |
| `k` | the instance's KPI readings of the last 7 days |

The TUI reads what the dashboard reads (the store and `<state>/sessions/`) and writes only through
conductor commands, so a thing done in the TUI is a record.

### Settings: global, instance, role

```mermaid
flowchart TB
  classDef store fill:#ede9fe,stroke:#6d28d9,color:#111827
  classDef model fill:#dcfce7,stroke:#15803d,color:#111827
  classDef cmd fill:#fce7f3,stroke:#be185d,color:#111827
  g["globals: the defaults"]:::store -->|overridden by| i["instances[]: one organization"]:::store
  i -->|overridden by| r["roles[]: one role"]:::store
  r --> e["effective value of one session"]:::model
  e --> show["conductor config show --effective --instance ‹i›"]:::cmd
```

<details>
<summary>plain text</summary>

```text
globals ──overridden by──► instances[] ──overridden by──► roles[] ──► effective value of one session
conductor config show --effective --instance <i>   every key with the level it came from
```

</details>

| level | file section | holds |
|---|---|---|
| global | `globals` | the defaults every instance starts from: harness, models per role, cadence, thresholds, disk, accounts, retention, the announcement hook |
| instance | `instances[].*` | overrides of any global key; sources, checkouts, records, state, cache, dashboard port, session prefix, the instance's roles |
| role | `roles[].*` (under `globals` or an instance) | profile, model, settings, tools, writable set, network, limits, schedule |

Precedence is role over instance over global. `conductor config show --effective --instance <i>`
prints every key with the level it came from. Today's `instances[].thresholds` keeps working: a key
set on an instance is an override.

### Built-in and custom roles

Built-in roles ship with a profile each: `supervisor`, `controller`, `planner`, `implementer`,
`reviewer`, `releaser`. A custom role is one entry in `roles[]`:

```yaml
roles:
  - role: ui-pass
    profile: ~/.b10x/conductor/profiles/ui-pass.md
    model: sonnet
    scope: repository
    repositories: [manager]
    schedule: daily 06:00
    handover: 150k
```

The loop starts it on its schedule (`daily HH:MM`, `every 6h`, `on: release`, `on: red-ci`), one
session at a time, in a tree of its scope, and ends it when its report arrives. A custom role reports
to the controller and can file findings and dispatch requests; it cannot merge. Its profile lives in
the instance's `profiles/`, outside the conductor repository, so a company-specific role never
enters the product.

### Self-improvement over settings

```mermaid
flowchart TB
  classDef nomodel fill:#e5e7eb,stroke:#4b5563,color:#111827
  classDef host fill:#dbeafe,stroke:#1d4ed8,color:#111827
  classDef store fill:#ede9fe,stroke:#6d28d9,color:#111827
  classDef decision fill:#fef3c7,stroke:#b45309,color:#111827
  classDef good fill:#dcfce7,stroke:#15803d,color:#111827
  classDef bad fill:#fee2e2,stroke:#b91c1c,color:#111827
  tick["daily tick: conductor metric compute"]:::nomodel --> kpi["KPI readings against targets"]:::store
  kpi --> tuner["tuner, host role"]:::host
  tuner --> prop["settings-proposal record: key, old, new, the reading it should move, the window"]:::store
  prop --> grant{"key inside globals.tuning, value inside its range?"}:::decision
  grant -->|yes| apply["apply the change, open the window"]:::nomodel
  apply --> measure["measure after the window"]:::nomodel
  measure -->|reading improved| keep["keep it, record"]:::good
  measure -->|same or worse| revert["revert it, record"]:::bad
  grant -->|no| finding["finding, then a story for the conductor repository's controller"]:::decision
  keep --> brief["the daily brief"]:::host
  revert --> brief
  finding --> brief
```

<details>
<summary>plain text</summary>

```text
daily tick ─► KPI readings vs targets ─► tuner ─► settings-proposal (key, old, new, reading, window)
   key inside globals.tuning?
     yes ─► apply, open the window ─► measure ─┬─ improved ──► keep, record ───┐
                                               └─ same/worse ► revert, record ─┼─► the daily brief
     no  ─► finding ─► story for the conductor repository's controller ────────┘
```

</details>

The daily tick computes the KPIs per instance. A built-in host role, `tuner`, runs after it:

1. reads the readings against the targets and the open findings;
2. proposes one settings change at a time as a record (`settings-proposal`: key, old value, new
   value, the reading it should move, the window to measure);
3. applies it when the key is inside the grant in `globals.tuning` (a list of keys with a bounded
   range, for example `cadence.watch: 60s..600s`, `controllers.max_working: 2..8`,
   `thresholds.context_handover: 120k..250k`);
4. measures after the window; keeps it when the reading improved, reverts it otherwise, and records
   both.

A key outside the grant, and anything that touches a profile, becomes a finding and a story for the
conductor repository's controller. The operator widens or narrows the grant in config; he never
sees a proposal inside it except in the brief.

### Migration, each step reversible

| step | change | reversible by |
|---|---|---|
| 1 | `conductor_host.slice`; the supervisor and teacher as roles in `globals.roles`, started by `conductor session start --role supervisor`; `task supervisor:start` calls it | the old task line |
| 2 | `conductor tui` read-only: instances, sessions, attach, watch, log | nothing to undo |
| 3 | `globals` with precedence; `config show --effective` | keys stay valid at the instance level |
| 4 | `roles[].schedule` and `scope`; the loop starts scheduled roles | remove the entry |
| 5 | one controller per instance; today's repository controllers become `implementer` workers it starts | `controllers.per_repository: true` keeps today's form |
| 6 | the conductor repository in an instance; `releaser` role; `conductor self-install`; `conductor-dev` retired | the `conductor-dev` role stays accepted for one release |
| 7 | TUI settings editor, `d`, `r`, `k` | read-only TUI |
| 8 | `tuner` with an empty grant; the operator fills `globals.tuning` | an empty grant |

### Decided

| question | answer |
|---|---|
| teacher: own role or part of the supervisor? | its own role, above the supervisor, outside the system like an adversarial review (operator, 2026-11-07: "the teacher should clearly be its own role, and it should just be above the supervisor. It's just like the concept of adversarial review. So we're stepping out of the system and then reviewing it from there") |

### Open questions for the operator

| question | default without an answer |
|---|---|
| Does one controller per instance keep a tree per repository, or does it work in the workers' trees only? | workers' trees only; the controller has no repository tools |
| Which keys go into the first tuning grant? | none; the tuner proposes and the brief shows the proposals |
