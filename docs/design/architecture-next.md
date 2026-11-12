# Design: conductor v1 — a daemon, confined instances, abstract agents, one store

Status: proposed 2026-11-07; revised 2026-11-08 on the operator's decisions (§13). Store record:
`design:architecture-next`, epic `epic:architecture-next`. The public page of the host as it
shipped is `website/docs/concepts/the-host.md`; the design's own public page was retired into this
document. The work lands on the integration branch `v1` (§14); the shipping line, v0, is untouched
until the cutover.

The rule this design is built under (operator, 2026-11-08): "don't shy away from removing baggage,
the more we remove the better" and "we work spec-first ESS": the whole system as it is written here
is modelled in `spec/` before a handler is written (§3).

## 1. What the operator asked for

Quoted on 2026-11-07, in order:

1. "not every instance should come with its own dashboard, aka web UI. Each instance comes with an
   HTTP API ... but not with its own UI deployment."
2. "something like conductor daemon ... making sure that instances are started and stopped and so on
   based on the configuration. It's, so to say, reconciling the instance state. And this conductor
   daemon itself has an API to list the instances ... some API federation ... a global API on the
   conductor level itself, and there's an instance level API. And then we have various UIs, so one of
   them being the dashboard, another one being a TUI."
3. "the actual instance ... should run in a confined way ... look at substrate from B10X or mantle
   ... the instances themselves, they should be capable of running remotely ... my daemon I would run
   locally here on my machine. Also the config would live here ... it's not only ... about remotely,
   it's about the confinement ... Docker, or ... Firecracker ... Substrate should support all of that."
4. "the state store should somewhat be abstracted, because we might want to put that into a database,
   which is long-lived in some way. I don't know, is config also state?"
5. "a regular design document in the conductor repo, with the planning store ... There was already
   some version 2 planned. It might be superseded ... And at the same time, I want to see the public
   documentation about this future architecture."

Earlier the same day, also in force: one controller and several workers per instance; one supervisor
across instances; the teacher its own role above the supervisor ("adversarial review"); no separate
maintainer; global settings plus instance settings; built-in roles plus custom roles on a schedule;
self-improvement that tweaks settings.

On 2026-11-08, answering the open questions (§13): no Agency; Substrate for every confinement, local
and remote; agents abstracted with today's Claude Code behaviour kept; one shared database;
the dashboard served by the daemon and deployable remotely at the same time; secrets through the
`secrets` crates; everything built now, on one integration branch, v0 to v1.

What the v0 host cost, measured on 2026-11-07:

| friction | measured |
|---|---|
| three start mechanisms (unit, `claude --bg`, hand) | two sessions of one name (`<name>-2`) and a stale resumed id in one day; the rename needs `claude stop` and `claude --bg --resume` |
| the maintainer is a special role in config (`conductor-dev`) and in the host section | its hand-over left 52 local-only commits on 7 branches; no loop rule covered it until wave 45 |
| the supervisor is outside the config | its rules lived in a file under `~/.claude/agents/` until they became a shipped profile; four host scripts were written beside it |
| settings are per instance only | the same thresholds are copied into three instances; a host-wide change is three edits |
| one controller per repository | an instance with 6 active repositories runs 6 model sessions and 6 trees whether or not they have work |
| no custom role | a recurring job (a UI pass every day) has no place but a controller's charter |
| a dashboard per instance | three dashboard units on three ports, one Vue build each, no view across instances but the registry list |
| the store is files in the instance's state directory | an instance can only run where its files are; a remote or boxed instance has no way to keep its records at home |
| instances are installed by hand | `conductor init` and `conductor run --install` per instance; nothing compares the config with what runs |

## 2. The shape

```text
control host                                      placements (substrate: local or remote daemon)
┌────────────────────────────────────────┐        ┌─ instance A, a substrate box ───────────────┐
│ conductord                             │        │ conductor instance serve                    │
│  reconciles instances from config      │───────►│   loop (no model)                           │
│  global API  /v1/...                   │◄───────│   instance API /v1/... (socket)             │
│  federates instance APIs               │        │   agents: controller, workers, custom roles │
│  serves the dashboard app              │        │   store: the one shared database            │
│  hosts supervisor and teacher          │        └─────────────────────────────────────────────┘
│  the shared store: sqlite or postgres  │        ┌─ instance B, a box on a remote substrate ───┐
│  conductor.yaml (desired state)        │───────►│ …                                           │
│  secrets: keychain (local)             │        └─────────────────────────────────────────────┘
└────────────────────────────────────────┘
   ▲ dashboard, local      ▲ dashboard, deployed remotely for colleagues      ▲ conductor tui   ▲ CLI
```

The control host is wherever conductord runs: the operator's workstation, or a remote box (a cloud
instance) that several engineers reach through the daemon's API (operator, 2026-11-08: "conductor
D, the daemon itself, is running also on the remote host, say some EC2 instance. Then we can connect
via API. So this must of course be secured somehow"). The config lives with the daemon, in git.

Five concepts, each with one job:

| concept | one per | job | replaces |
|---|---|---|---|
| **conductord** | control host | reads the config, reconciles instances (desired = config, actual = what runs), opens and closes each instance's box at its placement, owns the global API, federates the instance APIs, serves the dashboard app, starts the host roles (supervisor, teacher), opens the one shared store and hands its URL to every runtime | `host/instances.json`, `conductor_<i>-run.service`, `conductor_<i>-dashboard.service`, the bridge session of `roles-and-surfaces.md` §4 |
| **instance runtime** (`conductor instance serve`) | instance | the loop (`conductor-run.md`), the instance API, the starter of the instance's agents; writes the instance's store | `conductor run`, the per-instance dashboard server (its routes become the instance API) |
| **placement** | instance, overridable per role | a Substrate box, from a daemon on this host or a remote one, with an adapter (bubblewrap, a container, a pod) and a profile | the `confinement.provider` enum, `confine/systemd.rs`, `confine/none.rs`, the host ledger's slices |
| **agent kind** | role slot | what program fills a role and how conductor starts, feeds, reads and stops it; `claude-code` in v1, with today's behaviour | the one hard-wired `claude --bg` launcher |
| **UIs** | any number | the dashboard (one web app over the global API, served by the daemon and deployable elsewhere), the TUI (`conductor tui`), the CLI (`conductor <cmd>` as an API client), the operator's chat session (through the supervisor) | the per-instance dashboards; `conductor ask` becomes `/v1/ask`, routed by the supervisor |

The roles: supervisor (host, one), teacher (host, one, above the supervisor and outside the
system), controller (one per instance), workers (built-in roles planner, implementer, reviewer,
releaser), custom roles on a schedule. The maintainer is retired: conductor's repository is a
repository of one instance.

| level | one per | role | what it does |
|---|---|---|---|
| host | machine | teacher | outside the system, above the supervisor: reviews what the supervisor decided and what the instances delivered, the way an adversarial reviewer reads a finished change; changes the supervisor's profile and files findings; never runs the supervisor's routine itself |
| host | machine | supervisor | keeps every instance's loop, controller and workers running; carries findings across instances; decides within its grant; reports to the operator |
| instance | organization | controller | the instance's judgement: picks work across the instance's repositories, starts and steers workers, answers requests, merges what passes, escalates |
| instance | organization | loop | `conductor instance serve`: snapshots, timers, agent starts, hand-overs, KPIs, the instance API; no model |
| repository or unit | work | worker | one job in one tree: plan, implement, review, release; built-in roles |
| any | schedule | custom role | a role the config adds: a profile, a scope and a schedule; conductor starts it like any other (§10) |

The star stays: a worker talks to its controller, a controller to the supervisor, the supervisor to
the operator. A controller of one instance never messages another instance's sessions.

## 3. Specification first: the v1 domains

Operator, 2026-11-08: "the whole system as visible in the design/stories/pub docs as proposed needs to
be modelled in ESS". The specification (`spec/`, `ess 0.93`) is the source of truth for every noun,
command, view and error below; `crates/conductor-model` is generated from it; a handler is written
against the generated types (`AGENTS.md` rule 1). The first story on `v1` is the model
(`story:spec-v1`); every other story depends on it and starts by reading its domain.

| domain | status in v1 | holds |
|---|---|---|
| `conductor.instance` | new | `Instance` (desired: name, placement, store, roles, settings; actual: state, binary version, last seen, box), `Reconciliation` (one record per action: start, stop, restart, upgrade, nothing), commands `Reconcile`, `StartInstance`, `StopInstance`, `RestartInstance`, `UpgradeInstance`, `PurgeInstance` (class H), views `Instances`, `Reconciliations`. Replaces the registry types of `conductor.host` (`InstanceRegistry`, `RegisteredInstance`, `RegisteredFile`, `DashboardInstance`, `InstanceCollision`) |
| `conductor.placement` | new | `Placement` (substrate daemon: local socket or URL with a bearer; adapter; profile: read-only roots, writable workspace, limits, egress, secret slots), `Box` (opened, closed, refused with the substrate reason), commands `OpenBox`, `CloseBox`, errors `PlacementRefused` (named: `exec.sandbox-unavailable` and the others substrate declares). Replaces `confinement.provider`, `confinement.mode` and the systemd provider's places |
| `conductor.agent` | new | `AgentKind` (`claude-code` in v1), `Capabilities` (`interactive`: PTY, attach, deliver, hand-over; `run-to-exit`: an exit code and an events file), `Agent` (a role slot filled: kind, placement, profile, limits, state), commands `StartAgent`, `DeliverToAgent`, `AttachAgent`, `StopAgent`, events `AgentStarted`, `AgentReported`, `AgentStopped`. `conductor.session` becomes the `claude-code` kind's behaviour, kept scenario for scenario |
| `conductor.store` | new | `Backend` (`sqlite` for one host, `postgres` for a deployment), `Store` (the one URL the daemon and every instance runtime open; the password a `secret:<name>`), command `MigrateStore` (today's tree into the database, once), view `StoreStatus` (backend, reachable, last event id per instance). One database per deployment, shared by every operator, instance and UI (operator, 2026-11-08); no copy per instance, no sync. The records directory stays a projection |
| `conductor.secret` | new | `SecretName` (what the config names), `Source` (`keychain`, `remote`), `Slot` (a sealed slot handed to a box), command `ResolveSecrets` at box open, view `Secrets` (names and sources, never values) |
| `conductor.operator` | new | `Actor`, a kind and the fields of that kind: `human` (an `Operator`, an Identity principal), `agent` (an `Agent` of `conductor.agent`: instance, role, name, kind) or `daemon` (conductord or an instance runtime on its own account); `Operator`, `Grant` (answer decisions, change settings, start, stop and upgrade instances, purge, join); every command, event and record carries its `Actor`; `JoinInstance`, `LeaveInstance`; views `Operators`, `Actors`. The instance cluster has several operators (operator, 2026-11-08); a human and an agent are told apart by type, never by name (operator, 2026-11-08: "we have the actors, and we kind of need to distinguish between who is an agent actor and who is a human actor") |
| `conductor.token` | new | `TokenPool` (per instance), `PoolToken` (owner, seat, state `ok` or `limited until`), commands `ContributeToken`, `WithdrawToken`, `LeaseToken`, `ReleaseToken`, `MarkTokenLimited`, view `Tokens` (owners, seats, states, never values); the `claude-code` kind's lease per agent start (§5.3) |
| `conductor.config` | changed | `Level` (global, instance, role) and precedence, `EffectiveSettings` view, command `PutConfig` (validated, written, committed, reconciled) with event `ConfigChanged`; `dashboard_port` and `confinement.*` removed; `conductor.config/2` |
| `conductor.improvement` | changed | `TuningGrant` (keys with bounded ranges; empty by default), a proposal applied only inside the grant through `PutConfig` |
| `conductor.host` | shrunk | keeps grants, disk reservations, build runs and reclaim (the host ledger for boxes on this host); loses the registry, the dashboard instance and the host-roles config |
| `conductor.observation`, `conductor.direction`, `conductor.decision`, `conductor.dispatch`, `conductor.tool` | unchanged | the loop's facts, goals, decisions, dispatches and tools |

**Actors.** Every command the specification accepts names who issued it. An actor is a record with
a kind, human, agent or daemon, and the fields of that kind. A human actor is an operator with
grants, named by the operator's name, so an answer, a config change, a join and a purge each name
the person. An agent actor is the agent the runtime started, named by its instance, role, name and
kind, so a decision a controller took, a report a worker wrote, a message the supervisor sent and a
guard denial each name the program and its role. A daemon actor is conductord or an instance
runtime acting on its own account, when it reconciles, observes an instance, opens or closes a box,
resolves secrets or records a stop it observed; it carries no other field. A grant belongs to an
operator only; an agent and the daemon hold none. A grant is checked on the kind, so a command that
needs one refuses every actor whose kind is not human by name (`ActorNotHuman`), and that refusal
is a predicate in the specification that synthesis tests. Every event and every record carries its
actor, every view shows it, and the records projection writes it as `human:<name>`,
`agent:<instance>/<role>/<name>` or `daemon`; the `human:` prefix is the one AEP's transitions
already write. The design's class H decision is the one only a human actor may answer; class C is
the one an agent may take and record (§ 5 of the conductor design).

Components (`spec/components.yaml`): `conductord` (owns instance, placement, store, secret, config),
`conductor-instance` (owns observation, direction, decision, dispatch, agent, improvement, tool),
`conductor-cli`, `conductor-dashboard` and `conductor-tui` as clients that accept views and the few
writes the UIs make (answers, settings inside the grant). The two APIs are the components' accepted
commands and views, projected to OpenAPI by `ess`.

## 4. conductord: reconciling instances

The daemon is `conductor daemon serve`, installed as the user unit `conductord.service` (or a system
unit on a server), or as a StatefulSet of one in a Kubernetes namespace, where `daemon.kubernetes`
makes it run each instance's runtime as a pod of its own, beside the unit and the child process
and not as a placement (guide: run in Kubernetes). It owns the config file. Its loop:

```text
read config ─► desired instances (name, placement, store, roles, settings)
observe     ─► actual instances (what answers at each placement; what the store last recorded)
reconcile   ─► open a box and start what is desired and absent; stop what runs and is gone from config;
               restart what changed placement or binary; carry on what matches
record      ─► one Reconciliation per action in the daemon's own store
```

- Reconciliation runs on a config change (the daemon watches the file and takes `PUT /v1/config`),
  on a placement event (an instance runtime stops answering), and every `cadence.reconcile`
  (default 60 s). It is idempotent: a second pass with nothing changed does nothing (a test counts
  actions).
- The daemon never runs a model. A judgement it needs (an instance that fails to start three times)
  is a finding for the supervisor, which it starts and keeps running the way the loop keeps a
  controller today.
- Removing an instance from the config stops its runtime and keeps its records in the shared store;
  deleting data is `conductor instance purge <name>`, class H, the operator's yes.
- The daemon's own records (reconciliations, config changes) are in the same shared store (§8).
- **The daemon on a remote box.** The same binary, installed as a system unit on the box, with the
  config repository cloned there. Its API is the TLS listener of §5.2 only: every client (the
  dashboard in a browser, `conductor tui`, `conductor <cmd>`) presents an Identity bearer;
  `conductor login <daemon url>` obtains one from the organization's Identity service and keeps it in
  the client's keychain; `--daemon <url>` or `CONDUCTOR_DAEMON` names the daemon a command talks to,
  default the local socket. The loopback listener is off on a box that has no local operator. The
  secrets source on a box is the Secrets service (§9); the OS keychain is for a workstation.

## 5. APIs and federation

### 5.1 Instance API

Every instance runtime serves `/v1/` on a Unix socket inside its box (`<runtime>/api.sock`), carried
to the daemon by substrate's session transport when the box is remote. The routes are today's
dashboard routes, renamed and with the same semantics:

| today (`/api/...`) | next (`/v1/...`) | writes |
|---|---|---|
| `board`, `repositories`, `pull-requests`, `workflow-runs`, `releases`, `reviews` | the same | no |
| `events?since=`, `unanswered`, `decisions`, `series`, `kpis`, `waiting-on`, `sessions`, `blockers`, `specifications`, `health` | the same | no |
| `plan` GET/POST, `plan/story` | the same | POST |
| `config` GET/POST | `settings` GET/PUT: the instance's effective settings and their level | PUT |
| `answers` POST | the same | POST |
| (none) | `agents/{name}/attach` (WebSocket, a PTY stream, interactive kinds only), `agents/{name}/deliver` POST, `agents/{name}/restart` POST | yes |

Writes carry the token the daemon minted for the instance at start, the same rule as today's dashboard
token. The instance API never reaches another instance.

### 5.2 Global API

`conductord` serves `/v1/` on `127.0.0.1:<daemon_port>` (default 7300) and, when configured, on a TLS
listener with Identity bearer tokens for a dashboard deployed elsewhere. Three kinds of routes:

| kind | routes | how |
|---|---|---|
| the daemon's own | `instances` (list with placement, state, binary version, last seen), `instances/{name}` (start, stop, restart, upgrade), `config` GET/PUT (validated, written, committed, reconciled), `secrets` (names and sources), `reconciliations`, `health` | the daemon's store and the config |
| proxied | `instances/{name}/*` → the instance's `/v1/*` | one connection per instance, held by the daemon; the dashboard token is translated so a browser holds one token, the daemon's |
| federated | `sessions`, `decisions`, `unanswered`, `kpis`, `events` (SSE), `ask` | the union of every instance's answer, each row carrying `instance`; `ask` is delivered to the supervisor, whose answer names its route (`→ <instance>/<repository>`, the rule of `roles-and-surfaces.md` §4) |

An instance that does not answer is a row with `state: unreachable` and the time it last answered, never an
error for the whole view.

### 5.3 UIs over the APIs

The dashboard is one web app that reads the global API. The daemon serves it at `/` on the control
host, where the operator controls conductord through it (config, instances, answers). The same app is
deployed elsewhere, in a cluster, against the daemon's TLS listener, where colleagues read it; both
run at the same time (operator, 2026-11-08: "not either or - both should be possible at the same
time"). What a remote reader may write is the bearer's grant: read-only by default. The TUI
(`conductor tui`) and the CLI are clients of the same API; the TUI's first release is read-only with
attach, its settings editor comes with the tuner's grant. Against a daemon on a remote box (§4), the
dashboard is where the team approves stories and follows progress (operator, 2026-11-08): a story
approval is a request the controller raises and an operator with the grant answers on the Decisions
page, after which the controller moves the story in its repository's planning store (the store stays
the owner, `AGENTS.md` rule 11); progress is the federated `kpis`, `series`, `sessions` and
`decisions` views.

**The TUI.** `conductor` with no arguments at a terminal, and `conductor tui`, opens a
full-screen view over the global API, the one the dashboard reads. It reads the daemon `--daemon
<url>` names, else `CONDUCTOR_DAEMON`, else `127.0.0.1:<daemon.port>` of the config file, so it
works from any machine that reaches the daemon. It reads `GET /v1/instances`, `decisions`, `kpis`,
`sessions` and `instances/{name}/settings`, and writes nothing.

```text
┌ instances on <daemon> ─────────┬ sessions of A ─────────────────────────┬ settings of A (read-only) ─┐
│ > A   running      2 dec  K ok │ > controller   busy   412k  wave/46    │ cadence.cycle        2h    │
│   B   running      0 dec  K1 ! │   controller   idle    38k  story:x    │ thresholds.disk_low  200G  │
│   C   unreachable  ? dec  K ?  │   conductor    busy      -  -          │ controllers.max_working 8  │
├────────────────────────────────┴────────────────────────────────────────┴────────────────────────────┤
│ Enter attach  w watch  l log  ctrl-] d back  Tab pane  j/k move  r refresh  q quit                   │
└──────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| key | does |
|---|---|
| `Enter` | attach the selected session; `ctrl-]` then `d` returns to the TUI, the session untouched |
| `w` | watch it read-only |
| `l` | follow its terminal log (`<session>.terminal.log`) |
| `Tab`, `j`, `k` | switch between the instances and the sessions pane, and move in it |
| `r` | read every pane again; they are also read again every 5 seconds |
| `q` | quit |

The instances pane shows each instance's state, its open decision requests and its KPI flag (`K
ok`, or `K<n> !` for `n` KPIs off their target); the sessions pane each session's role, state,
context and current work; the settings pane the selected instance's effective settings. An
instance that does not answer is shown `unreachable`, never an error for the whole view. Attach
hands the terminal to `conductor session attach` when the daemon is on this host. Against a
daemon on another host the TUI asks `GET /v1/instances/{name}/agents/{agent}/attach` through the
daemon; a PTY stream over HTTP is not served yet, and the TUI shows that answer on its status line.

**Several operators.** The instance cluster has operators, not one operator (operator, 2026-11-08:
"multiple operators possible of the instance cluster"). An operator is an Identity principal with
grants (`conductor.operator`, §3); every write on the global API carries the operator's bearer and
every record it causes (an answer, a config change, a reconciliation, a purge) names the operator.
The supervisor's "the operator" is the operator a request is routed to by grant; a class H decision
is answered by any operator whose grant names it, and the record says which. On a single host with
no Identity, one operator is implied by the Unix user, named in the config.

**Joining an instance, and the token pool.** An engineer joins an instance with `conductor join
<daemon url> --instance <i>`: they log in through Identity, the daemon records them as an operator
of that instance with the grants the config gives their role, and they contribute their Claude
token to the instance's **token pool** (operator, 2026-11-08: "some engineer of us is basically
joining ... an instance ... and by doing so, they would basically hand out their token to the pool
of Claude tokens. And the agents of kind Claude Code, they would make use of these tokens"). The
pool is a set of secrets in the instance's secrets namespace, each with its owner, its seat and its
state (`ok`, or `limited until <time>`); values never leave the secrets source except as a sealed
slot. The `claude-code` kind leases one token per agent start (the owner's own token first for an
agent the owner asked for, otherwise the token with the most headroom), runs the agent on it, and on
a usage-limit stop releases it as `limited` and resumes the agent on another (today's `auto-resume`,
over the pool). Every agent record names the token's owner, so usage is attributable per seat.
`conductor leave` withdraws the token and the agents on it are resumed on others; `GET
/v1/instances/{name}/tokens` lists owners, seats and states, never values. A Remote Control session
belongs to the login whose token started it, so an agent on an engineer's token is the one that can
appear in that engineer's Claude app (§17).

## 6. Placements: Substrate, local and remote

Conductor implements no sandbox and keeps no confinement provider of its own. A placement is a
Substrate daemon plus an adapter plus a profile (operator, 2026-11-08: "rely on substrate for local
and remote confinement (bwrap, docker, k8s)"; ADR 0109: confinement is one Substrate contract,
`ConfinementProfile` with one adapter per mechanism, and conductor's `systemd-run` moves onto
Substrate, `adr/0109:84-85`).

| `placement.daemon` | runs where | adapters (substrate 0.16.1, read 2026-11-07) | transport |
|---|---|---|---|
| `local` (default) | this host, `substrate-daemon --socket`, kept running by conductord | `systemd` (a unit in a slice with cpu, memory and task limits: today's box), `linux` (bubblewrap plus a delegated cgroup: read-only roots, writable workspace, default-deny egress with named capabilities, `~/beyond10x/substrate/README.md:225-226`, `ROADMAP.md:27`), `oci` (a container; Docker and Firecracker drivers when substrate ships them, `ROADMAP.md:30-31`) | substrate's Unix socket |
| `<url>` | a remote host or cluster running `substrate-daemon --tcp-listen --tcp-bearer-file` | the same, plus `pod` (one pod per box in a cluster) | HTTPS/WSS with an Identity bearer (`ROADMAP.md:28`) |

Of the adapters, `linux` is the default. `systemd` is today's box, a unit in a slice with cpu
weight, memory and tasks and no write fence, kept for an instance that writes it on purpose.

What the daemon passes to substrate for one instance (the `ConfinementProfile` admitted through
`substrate_host::confinement::open(AdapterRequest)`, `crates/substrate-host/src/confinement/mod.rs:142`,
or the SDK's `client.workspace()` builder):

- argv and cwd: `conductor instance serve --instance <name>`, the instance's records directory;
- read-only roots: the conductor binary and its profiles, the agent kind's binary (the harness), the
  instance's checkouts root when `checkouts.trees` is elsewhere;
- writable places: today's six (`confine/places.rs:84-101`: checkout, trees, build cache, scratch,
  harness state, records), as one substrate workspace with subdirectories;
- limits: cpu, memory, pids, storage, from the role's `limits` and the instance's `limits`;
- egress: the forge hosts of the instance's sources, the model API host, and nothing else;
- secrets: every credential the roles need (forge token, harness token, the dashboard token) as a
  sealed memfd slot (`README.md:375-378`), never in argv or env; the daemon resolves them from its
  secrets source (§9) at box open.

Rules:

- A placement that cannot give what the profile asks for is a refusal with substrate's name
  (`exec.sandbox-unavailable`), recorded as `PlacementRefused`: the instance does not start and the
  daemon records why. Nothing falls back: ADR 0109 says "`local` is a configuration, not an absence
  … Nothing falls back to it" (`adr/0109:63-65`). Substrate's `local` adapter (no isolation) is a
  configuration an instance writes on purpose, with a reason, or not at all.
- Every agent inside the box is a substrate exec or PTY session (`get_pipe_session`), one per role
  slot, started by the instance runtime through the same SDK. tmux stays the terminal multiplexer
  inside the box for the `claude-code` kind, so attach, deliver and the terminal log keep working
  unchanged.
- The daemon keeps `substrate-daemon` running on the control host, and treats a remote substrate like
  any unreachable instance: a row, not an error.
- A host policy (substrate's) may refuse an adapter; conductor passes the refusal through with its
  name.

## 7. Agent kinds

Operator, 2026-11-08: "currently we use ClaudeCode agents. but at some point a controller could also
be a takt protocol, and the workers could also be various different 'executions' - we just need to
abstract it. any combination possible - so abstract it - but keep exact behaviour now
(claudecode-based)".

An agent kind is the program that fills a role slot and the way conductor starts, feeds, reads and
stops it. A role slot is `(role, kind, placement, profile, limits)`; any kind may fill any role, at any
placement, when its capabilities cover what the role needs.

| capability | means | needed by |
|---|---|---|
| `interactive` | a PTY the operator can attach to, a mailbox the runtime delivers into, a hand-over at a context threshold, a terminal log | controller, supervisor, teacher, planner, releaser, custom roles as written today |
| `run-to-exit` | one unit of work in, an exit code and an events file out; a message to it is a new generation | implementer, reviewer, any worker whose unit is one story |

| kind | capabilities | in v1 |
|---|---|---|
| `claude-code` | interactive | the only kind. Its behaviour is today's, kept exactly: the settings file and `tmux.conf` written, one tmux server in the box, the harness with the role's profile appended, the envelope, the terminal log, the hand-over, the guard as a PreToolUse hook. `conductor.session`'s scenarios and the tests `confine_systemd`, `session_attach`, `session_start` are the pin; they pass unchanged against the kind. Its token is leased from the instance's token pool (§5.3); with a pool of one, that is today's one token |
| `takt` | run-to-exit | a draft story after v1: `takt run --repo <tree> --work aep:<story>`, exits 0 to 9, `events.jsonl` (takt 0.34.0, §12); as a controller, a takt protocol |
| `codex`, a `loom.agent/1` turn | interactive, run-to-exit | named here so the trait is not shaped for one program; no story |

The trait in `crates/conductor` (`agent::Kind`): `start(slot) -> Handle`, `deliver(handle, text)`,
`attach(handle) -> PtyStream` (interactive only), `reports(handle) -> Stream<Report>`, `stop(handle)`,
`capabilities()`. The instance runtime holds the handles; the instance API's `agents/*` routes are the
trait over HTTP. The `[REPORT]`, `[NEED]` and `[DECISION-REQUEST]` lines of design §6 stay the report
vocabulary; a run-to-exit kind's runtime adapter writes them from the exit code and the events file.

**Every agent is started by conductor.** One start form for every role: `conductor agent start
--instance <i> --role <role>`, answered by the instance's runtime at its placement, and the host
roles by the daemon, in their own box. `claude --bg`, `task supervisor:start` and the adapters
under `~/.claude/agents/` go away; the harness view (`claude agents`) stays a diagnostic. For the
`claude-code` kind a start is:

```text
loop, TUI or schedule ──► conductor agent start --instance i --role r
                            1  write <session>.settings.json and <session>.tmux.conf
                            2  substrate: open a session in the instance's box
                            3  tmux -D -S <state>/sessions/<session>.tmux
                            4  one window: the harness with the role's profile
                            5  measure the box, write <session>.envelope.json
                      ◄──── session name and socket
harness ──► reports, requests, hand-over at the threshold
```

Rules carried over unchanged: one session per name, a successor only after its predecessor
stopped; hand-over at the context threshold with pushed branches; the terminal log as the record.

## 8. State: config and store

The operator's question, "is config also state?", has the answer: config is *desired* state and is
not kept in the store; the store keeps what *happened*, including every config change.

| kind | examples | lives | authoritative copy | survives the box |
|---|---|---|---|---|
| config | `conductor.yaml`: instances, placements, roles, settings, secret names | with the daemon, in a git repository | the file | yes |
| records (the store) | decisions, dispatches, readings, findings, charters, hand-offs, snapshots, config changes, reconciliations | in the one shared database of the deployment, which every instance runtime, the daemon and every UI open | the database | yes |
| runtime state | agents, sockets, envelopes, leases, terminal logs, the watch's files | inside the box, under `<runtime>/` | the box | no; rebuilt from records at the next start |
| cache | exports of `origin/main`, build caches, repository caches | inside the box | none | no |

**One shared database per deployment** (operator, 2026-11-08: "with this multi-user, multi-operator
thing, we of course need some shared state database ... this complexity with instance level state
synced to the operator's machine, it doesn't make any sense ... it's just one database and it will be
shared"). The store is opened through Eventlog's port (ADR 0041: Eventlog is the shared persistence
port; ADR 0053: a store provider sits on Eventlog rather than growing a second implementation; ADR
0063: the wire bytes are the same across its adapters) with two of its adapters: `eventlog-sqlite`
for a deployment that is one host (one file under the daemon's state directory, opened by the daemon
and by every instance runtime on that host), `eventlog-postgres` for everything else (a server every
box and every workstation reaches; its password a `secret:<name>`). `daemon.store` names the one URL;
the daemon hands it to every instance runtime it starts, and every UI reads through the daemon's API.
Each event names its instance, so one database holds every instance's records and the daemon's own.
No copy per instance, no sync, no restore: a box that is lost has written nothing the database does
not hold, and the next start of the instance opens the same store. The records directory
(`decisions/YYYY-MM.jsonl`, `charters/`, `docs/handoff/`, `rules.md`) stays a human-readable
projection written by the CLI, committed by the daily tick, and is not the store.

Today's eventlog tree (one file per event) is read once by `conductor store migrate` into the
database and then not opened again; the tree backend and the per-instance `store:` key are not in
v1. The earlier decision "remote -> sqlite, local too - with sync" (§13 row 4) is superseded by this
one.

## 9. Config and secrets

- One file, `conductor.config/2`: `version`, `daemon` (port, TLS listener, `cadence.reconcile`,
  `store` (the one URL), the secrets source, the dashboard's served path), `globals` (defaults every instance
  starts from: roles with kind, model and profile, cadence, thresholds, limits), `instances[]` (name,
  `placement`, sources, checkouts, records, roles, overrides of any global key). Precedence: role over
  instance over global. `conductor config show --effective --instance <i>` prints every key with its
  level. A `conductor.config/1` file is read by `conductor config migrate` once: `dashboard_port` and
  `confinement.*` are dropped with a note, `host.roles` becomes `globals.roles`. What each level
  holds:

  | level | file section | holds |
  |---|---|---|
  | daemon | `daemon` | port and TLS listener, `cadence.reconcile`, `store`, the secrets source, the dashboard's served path, the operators and their grants |
  | global | `globals` | the defaults every instance starts from: the kind and model per role, cadence, thresholds, limits, retention, the announcement hook |
  | instance | `instances[].*` | overrides of any global key; `placement`, sources, checkouts, records, session prefix, the instance's roles |
  | role | `roles[].*`, under `globals` or an instance | kind, profile, model, settings, tools, writable set, network, limits, schedule, placement |

  An instance's `thresholds` of today keep working: a key set on an instance is an override.
- Secrets never enter the file: the file names them (`forge_token: secret:gitlab-bot`), the daemon
  resolves names at box open and passes them as substrate slots. The source is the `secrets`
  workspace's storage port (`~/beyond10x/secrets`, 0.6.0, read 2026-11-08): `secrets-keychain` (the
  OS keychain: the Secret Service on Linux, the native keychain on macOS) on the control host, and
  `secrets-remote` (the Secrets service: encrypted, tenant-scoped custody) where conductord runs in a
  deployment; `secrets-federation` routes each namespace to one backend, so one config serves both
  (operator, 2026-11-08: "with local OS keychain - remotely something else of the b10x arsenal").
  Conductor writes no secrets trait of its own.
- The rule of ADR 0110 (proposed) is kept: "An agent's own process holds no credential for an external
  system" (`adr/0110:62-64`); forge writes keep going through the gates route, and the only slot in an
  agent's box is the model token.
- Every accepted `PUT /v1/config` is validated, written, committed to the config repository with the
  caller and reason, recorded in the daemon's store as `ConfigChanged`, and reconciled. The tuner's
  settings proposals (`self-improvement.md`, §10 "The tuner") use this route inside their grant,
  which is empty until the operator writes keys into it (§13).

## 10. Roles under the daemon

| role | started by | where | kind in v1 |
|---|---|---|---|
| supervisor, teacher | the daemon, `globals.roles` | the control host, a local substrate box | `claude-code`; the teacher only when the operator asks; it reviews the supervisor from outside |
| controller (one per instance) | the instance runtime at start | the instance's box | `claude-code`; picks work across the instance's repositories; starts workers |
| workers, custom roles | the controller or the schedule, through the runtime | the instance's box, one tree each | `claude-code`; another kind per slot once it exists |
| releaser (conductor's repository) | its controller | its box | after a release, the daemon's `instances/{name}/upgrade` installs the binary at each placement, one instance at a time, under the supervisor's grant; rollback on a runtime that does not come up |

The star stays: workers to their controller, controllers to the supervisor, the supervisor to the
operator. The daemon talks to runtimes, never to agents.

**No separate maintainer.** conductor's own repository is a repository in one instance's
`sources`; its controller plans and delivers conductor changes like any other repository's. What
was special about `conductor-dev` becomes two things:

- the built-in worker role `releaser` (version, changelog, tag, forge Release, announcement), which
  any repository can use;
- the daemon route `POST /v1/instances/{name}/upgrade`, under the supervisor's grant: install the
  released binary at the instance's placement, restart the runtime, run `conductor doctor` inside,
  and roll back when the runtime does not answer. The supervisor runs it one instance at a time
  after a release; the operator sees it in the daily brief.

The findings a session files about conductor (`conductor finding file-finding`) go to that
repository's planning store, as before.

**Built-in and custom roles.** Built-in roles ship with a profile each: `supervisor`,
`controller`, `planner`, `implementer`, `reviewer`, `releaser`. A custom role is one entry in
`roles[]`:

```yaml
roles:
  - role: ui-pass
    profile: ~/example-org/roles/ui-pass.md
    model: sonnet
    scope: repository
    repositories: [example-app]
    schedule: daily 06:00
    handover: 150k
```

The loop starts it on its schedule (`daily HH:MM`, `every 6h`, `on: release`, `on: red-ci`), one
session at a time, in a tree of its scope, and ends it when its report arrives. A custom role
reports to the controller and can file findings and dispatch requests; it cannot merge. Its
profile lives in the instance's `profiles/`, outside the conductor repository, so a
company-specific role never enters the product.

**The tuner.** The daily tick computes the KPIs per instance (`conductor metric compute`). A
built-in host role, `tuner`, runs after it:

1. it reads the readings against the targets and the open findings;
2. it proposes one settings change at a time as a record (`settings-proposal`: key, old value, new
   value, the reading it should move, the window to measure);
3. it applies the change when the key is inside the grant in `globals.tuning`, a list of keys with
   a bounded range, for example `cadence.watch: 60s..600s`, `controllers.max_working: 2..8`,
   `thresholds.context_handover: 120k..250k`;
4. it measures after the window, keeps the change when the reading improved, reverts it
   otherwise, and records both.

A key outside the grant, and anything that touches a profile, becomes a finding and a story for
the conductor repository's controller. The operator widens or narrows the grant in config, and
sees a proposal inside it only in the brief.

## 11. What v1 removes, and what this supersedes

Removed from the product (operator, 2026-11-08: "the more we remove the better"):

| removed | replaced by |
|---|---|
| the per-instance dashboard server and `conductor_<i>-dashboard.service`, `dashboard_port` | the instance API and the daemon's one dashboard (§5) |
| `host/instances.json`, the registry, `conductor config validate`'s prefix check against it | `Instances` derived from the config (§3, §4) |
| `conductor run --install`, `conductor_<i>-run.service`, `task supervisor:start`, `task teacher:start`, `task maintainer:start`, the adapters under `~/.claude/agents/` | the daemon starts every runtime and host role (§4, §10) |
| `confinement.provider`, `confinement.mode`, `confine/systemd.rs`, `confine/none.rs`, the host ledger's slices for sessions | Substrate placements (§6) |
| `conductor session start`'s hard-wired `claude --bg` launcher, `claude attach` as the attach path | the `claude-code` agent kind behind the trait (§7); attach through the API |
| the eventlog tree as a backend, a store per instance | one shared database through Eventlog's port, `sqlite` on one host and `postgres` otherwise (§8) |
| the maintainer role and `conductor-dev`'s separate session | the conductor repository as a repository of one instance; its releaser (§10) |
| the bridge session of `roles-and-surfaces.md` §4 | the federated API and `/v1/ask` |
| per-instance `CLAUDE_CONFIG_DIR` and binary pins of `roles-and-surfaces.md` §5 | the box and `instances/{name}/upgrade` |
| a secrets trait of conductor's own | the `secrets` crates (§9) |

Superseded in the planning store (moved on 2026-11-08, each with this design as the reason):

| artifact | verdict |
|---|---|
| `design:conductor-on-agency` and its epics `agency-runner`, `dispatch-runner`, `loom-agent-runner`, `cross-repository-waves` | superseded: Agency is dropped (§13); a dispatch names no runner, a role slot names its kind |
| `epic:host-bridge` and its stories bridge-session, ask-bridge-route, ask-direct, dashboard-ask-input, host-dashboard-home, host-dashboard-instances, host-instances-list, status-all-instances | superseded by the daemon's federated API and the one dashboard; `ask` stays as `/v1/ask`; `mailbox-operator-messages` stays |
| `epic:session-confinement` and its stories substrate-client, substrate-tool-server, substrate-gate-provider, session-writes-bwrap, session-forge-credentials | superseded by the `substrate` placement and the secret slots |
| `story:placement-loom`, `story:placement-mantle`, `story:store-postgres` of this epic | archived: Loom and mantle are not placements, a database is not a backend (§13) |
| `epic:conductor-run` (the loop daemon, shipped in part) | stays: the loop is the instance runtime's core |
| `epic:multi-instance-host` host ledger and grants | stays for boxes on this host: disk and build slots |
| `epic:process-supervision` C5 | stays, done by the daemon |

## 12. Read and set aside

Surveyed on 2026-11-07 at each repository's `origin/main`, so nobody reads them twice. None is a
dependency of v1.

| candidate | what it is | why it is not in v1 |
|---|---|---|
| Agency 0.1.0 (`~/beyond10x/agency`, 2026-10-31) | an ESS specification and what `ess` generates from it; the generated server exits at start ("unmet startup obligations", `generated/rust/agency/src/bin/agency-server.rs:22-23`); no CLI; the service and runs milestones `proposed`; four of the eight gaps conductor filed (G2, G3, G7, G8) unspecified | dropped by the operator (§13); atlas was asked to deprecate ADR 0105 |
| Loom 0.57.2 (`~/beyond10x/loom`, 2026-11-05) | a `Command` is a run-once Job whose exit code the operator writes to `status.run.exitCode` (`crates/loom-operator/src/reconcile.rs:914-1000`); no attach or PTY, no message into a running `Command`, no Substrate in the pod (`ConfinementMode` is `[none]`, `ess/domains/agent.yaml:44-48`), no remote executor | not a confinement; a cluster is substrate's `pod` adapter. A `loom.agent/1` turn may become an agent kind later (§7) |
| mantle 0.3.1 (`~/beyond10x/mantle`) | a whole session on an EC2 or KubeVirt worker from a `Session` manifest; `start`, `attach`, `status`, `stop`; the daemon socket forwarded over SSH | a remote substrate daemon gives the same box as the local one; one placement model instead of two |
| takt 0.34.0 (`~/beyond10x/takt`, 2026-11-06) | one run is one case on one unit (`--story`, `--work aep:<id>`); exits 0 to 9 (`crates/takt-cli/src/main.rs:1522-1574`); `events.jsonl`; `ApprovalRequired` exits 4 and `takt approve` resumes; no dispatcher in code; its design defers orchestration across repositories to conductor (`.engineering/planning/design/continuous-orchestration.md:236-239`); its harness child runs under no OS sandbox (`website/docs/known-limitations.md:32`) | the first run-to-exit agent kind after v1 (§7): a program the box confines together with its harness child |
| substrate 0.16.1 (`~/beyond10x/substrate`) | adapters `local`, `linux`, `macos`, `systemd`, `oci`, `pod`; a daemon with Unix socket and HTTPS serving; a Rust SDK and CLI; sealed secret slots; `exec.sandbox-unavailable` as a named refusal; Docker and Firecracker on the roadmap; a wire contract marked "development, not stable"; read again at 0.20.0: writable host roots the daemon declares by name | **in v1**: the only placement (§6); the unstable contract is pinned by version in `b10x.toml` and checked by `conductor doctor` |
| secrets 0.6.0 (`~/beyond10x/secrets`) | `secrets-core` (storage port), `secrets-keychain` (the OS keychain as a backend), `secrets-remote` (the service as a backend), `secrets-federation` (one backend per namespace), `secrets-client` | **in v1**: the secrets source (§9) |

## 13. Decisions

Taken by the operator on 2026-11-07 and 2026-11-08, recorded here and on the public page; not
reopened.

| # | question | answer |
|---|---|---|
| 1 | teacher: own role or part of the supervisor? | its own role, above the supervisor, outside the system like an adversarial review (2026-11-07) |
| 2 | merge step by step, or one fork? | "we want then basically a fork until all of it is working" (2026-11-07); "a new integration branch in which all of the stories can go active, but not merge to default branch yet ... its basically v0 -> v1 - since v0 is shipping code right now, we dont want to interrupt it" (2026-11-08): §14 |
| 3 | Agency, Loom and takt (the proposal of 2026-11-07) | "forget entirely about agency entirely, it was just a design, tell atlas to deprecate it. then in general i think we should just rely on substrate for local and remote confinement (bwrap, docker, k8s) - and then support multiple types of 'agents' - currently we use ClaudeCode agents. but at some point a controller could also be a takt protocol, and the workers could also be various different 'executions' - we just need to abstract it. any combination possible - so abstract it - but keep exact behaviour now (claudecode-based)" (2026-11-08): §6, §7, §12 |
| 4 | a remote instance's store: in the box with a mirror, or a shared database? | "remote -> sqlite, local too - with sync" (2026-11-08, morning); superseded by row 13 the same day |
| 5 | the dashboard: served by the daemon or deployed separately? | both at once: served by the daemon where the operator controls conductord, and deployed remotely in a cluster for colleagues (2026-11-08): §5.3 |
| 6 | the secrets source? | the `secrets` crates: the OS keychain locally, the Secrets service remotely (2026-11-08): §9 |
| 7 | the first tuning grant? | empty; the tuner proposes, the brief shows the proposals, the operator writes keys into the grant (taken by this design under the operator's rule "take the reversible option"; an empty grant is one) |
| 8 | which stories go active? | all of the epic's, on the integration branch (2026-11-08) |
| 9 | how many operators? | "multiple operators possible of the instance cluster" (2026-11-08): §3 `conductor.operator`, §5.3 |
| 10 | the daemon on a remote box; how engineers join | "conductor D, the daemon itself, is running also on the remote host ... we can connect via API. So this must of course be secured ... some engineer of us is basically joining ... an instance ... they would basically hand out their token to the pool of Claude tokens. And the agents of kind Claude Code, they would make use of these tokens" (2026-11-08): §4, §5.3, `conductor.token` |
| 11 | the Claude app as a UI? | option A: the `claude-code` kind starts a controller for Remote Control on a named owner's token from the pool, so it appears in that owner's Claude app ("yes, kind of option A. Yeah, this is option A", 2026-11-08): §17 |
| 12 | who issues a command: a person or a program? | told apart by type, `Actor::Human` or `Actor::Agent`, on every command, event and record; a grant belongs to a human actor only ("we have the actors, and we kind of need to distinguish between who is an agent actor and who is a human actor. I think it's important", 2026-11-08): §3; the form, superseded by row 14 |
| 13 | one store per instance with sync, or one shared database? | one shared database per deployment, `sqlite` on one host and `postgres` otherwise, through Eventlog's adapters; no per-instance copy, no sync, no restore ("with this multi-user, multi-operator thing, we of course need some shared state database ... this complexity with instance level state synced to the operator's machine, it doesn't make any sense ... it's just one database and it will be shared", 2026-11-08): §8, `conductor.store` |
| 14 | how is an actor written? | Actor is a kind and its fields, not a sum type; Daemon is the third kind (ess tests a field in a predicate, not a union's variant, so the refusal of a non-human actor is synthesized; taken by this design, 2026-11-08): §3 |

## 14. The integration branch and the order of work

The branch `v1` is cut from `main` after the design merges. Every story of the epic lands on `v1`
through a pull request against `v1`, with its own test gate (`task check` runs on it in CI);
nothing merges into `main` until the cutover. The live instance keeps `main`'s binary and units
throughout; the fork is tried on a scratch instance on this host (`conductor init` into a scratch
records directory, `example-org` sources) and on a scratch remote substrate when step 8 reaches it.

| step | story | what lands on `v1` |
|---|---|---|
| 0 | spec-v1 | the domains and components of §3 in `spec/`, the `Actor` type (human, agent or daemon) on every command, `ess specify validate`, `task regen`, one scenario per new command; the generated types every later step builds on |
| 1 | daemon-reconcile | `conductor daemon serve`: `Reconcile`, `Instances`, `Reconciliations`; the registry file gone |
| 2 | instance-api | `conductor instance serve` with `/v1/` on its socket; the daemon proxies and serves the dashboard app at `/` and on the TLS listener; the per-instance dashboard server gone |
| 3 | federated-views, tui-readonly | the federated routes; `conductor tui` read-only with attach |
| 3a | daemon-remote | the daemon as a system unit on a remote box; the TLS listener with Identity bearers as its only API; `conductor login`, `--daemon <url>`; the dashboard, the TUI and the CLI against it from a workstation |
| 4 | config-globals | `conductor.config/2`: levels, precedence, `config show --effective`, `PutConfig`, `config migrate`, operators and grants |
| 5 | store-shared | the one shared database through Eventlog's `sqlite` and `postgres` adapters; `daemon.store` handed to every runtime; `store migrate` from the tree |
| 6 | agent-kinds | the `agent::Kind` trait with `claude-code`; today's session scenarios and tests pass unchanged; the `agents/*` routes |
| 7 | placement-substrate-local, secrets-source | a local substrate box per instance and per agent; the profile of §6; slots from the keychain |
| 7a | token-pool | `conductor join` and `leave`; the instance's token pool in the secrets source; the `claude-code` kind leases per agent start, resumes on another token after a limit stop; owners on every agent record; `tokens` on the API; the controller for Remote Control on a named owner's token |
| 8 | placement-substrate-remote | a remote daemon over HTTPS with a bearer; the instance API over substrate's transport; attach as a stream |
| 9 | host-roles-by-daemon, instance-upgrade | the supervisor and teacher started by the daemon; the maintainer retired; `instances/{name}/upgrade` with rollback |
| 10 | tuner | the grant, empty, and the TUI's settings editor inside it |
| any | terminal-log-name | `<session>.terminal.log` |
| cutover | — | the live instance moves to `v1`'s binary and config, once; reversible by `main`'s binary and units; `v1` merges into `main` after the cutover has held for one day; the release is 1.0.0 |

## 15. Open questions for the operator

One remains.

**Does the controller keep a tree per repository, or work through the workers' trees only?**

Example: an instance with the repositories `loom` and `takt`; the controller (one `claude-code`
session in the instance's box) must decide what takt works on next and whether a finished story
holds.

| option | the controller has | it can | it costs |
|---|---|---|---|
| A | a full worktree per repository, as every repository controller has today | read code, run `aep plan artifact list`, run `cargo test` to check a claim itself | one checkout plus one `target/` per repository per instance (today's disk, GiB per repository) |
| B (default) | read-only exports of each repository's `origin/main` (the cache of §8), no `target/` | read code and the planning store, plan, judge diffs; it cannot build; a check it wants is a worker's job (`reviewer` on the branch) | one export per repository, refreshed by the loop's fetch; no build cache |
| C | nothing | read the forge and the shared store only; every look at code is a worker it starts | the cheapest box; a controller that asks a worker for every fact |

Default without an answer: B.

## 16. Against a team's operating model

A team note of 2026-11-07, read on 2026-11-08: one shared cloud box runs two conductor instances;
two or three people steer them from the Claude app on a phone or laptop; the aim is fewer people
running more agentic flows. Its decided and proposed points, each against v1:

| the note | v1 | gap |
|---|---|---|
| three layers per box: a box operator; one project desk per project, the single writer of its records, which briefs the work; one controller per repository doing the work | supervisor (host); controller (instance; the single writer, through the runtime); workers (repository or unit) | names only: the note's desk is v1's controller, the note's controller v1's worker (`epic:role-names`) |
| every project adopts ESS spec-first | conductor itself is spec-first (§3); the `specifications` collector reads each repository's `spec/` | none |
| a second person with their own Claude login and sessions on the box; reads everything, writes only to the project's inbox; decision rights on record; the desk follows the record, not whoever talks to it | operators with Identity bearers and grants (§5.3); `/v1/ask`, `deliver` and the mailbox are the inbox; every record names its operator; a class H decision is answered by the operator the grant names | a v1 grant is by action (answer, settings, instances); the note's rights are by topic (money, live calls): `Grant` gets a decision scope. A person's own Claude sessions are outside conductor: v1 neither starts nor traces them |
| steering from the Claude app: Remote Control sessions, each owned by the login that started it | the dashboard deployed remotely, the TUI, the CLI, `ask` through the supervisor; and, decided on 2026-11-08 (§17, option A), the controller started for Remote Control on a named owner's token from the pool, so it appears in that owner's Claude app | none |
| sessions move between people and machines as files: export with checksums, import as another instance, the box's shared tools instead of the export's, a hand-over note on top | an instance moves by changing its `placement` in the config: records and hand-overs are in git, the store is shared so the new box opens the same records, the box gets the host's tools as read-only roots | **partial**: the harness state (the model's memory, transcripts) is runtime state in v1 and does not survive the box; the note's moves carried it. Either the `claude-code` kind exports and imports its harness state per agent, or memory is declared disposable and the hand-over note is the carrier |
| reboot: every session, run loop and dashboard back within 15 minutes with no manual step | `conductord.service` reconciles every instance and host role at boot | none |
| two projects in parallel: no out-of-memory kill, no gate timeout caused by the other | limits per instance and per role through substrate; the host ledger's build slots and disk | none |
| stall detection: every stall over one hour caught by conductor or the operator, never by a person noticing | the loop's watch (busy or idle by terminal-log activity), `auto-resume`, respawn at hand-over; the daemon's findings to the supervisor | **partial**: no named stall threshold and no stall record. v1 adds `thresholds.stall` and a `Stall` finding with its hours, and counts them |
| single writer: every records commit from the desk or the loop, none from a person's session | the runtime writes the store; the records projection is committed by the daily tick; a person's bearer writes through the API only | none |
| secrets as files readable by the build user; a password-manager service account later | the keychain and the Secrets service, sealed slots, one model token per box (§9) | none; a password manager is not a backend of the `secrets` crates |
| usage limits: several flows around the clock on one seat; a second person's sessions on their own seat | the token pool (§5.3): every engineer who joins contributes a seat; an agent is resumed on another token after a limit stop; every record names the owner | none, with the pool |
| box capacity: a bigger box before a third project | a third instance on a second box through a remote placement; the daemon stays where it is | none; an improvement |
| tool churn: pin per repository, upgrade with the next conductor release | `b10x.toml` pins, `conductor doctor`, `instances/{name}/upgrade` | none |
| five metrics: units landed per week, steering time per day, decisions waiting on a person, median wait on a decision, stall hours per week | `kpis` and `series`: decisions waiting (`unanswered`), units landed (merged pull requests) | **partial**: the median decision wait and the stall hours are not KPIs yet; both derive from the store. Steering time is not conductor's to measure |

What v1 has that the note does not ask for: a box around every agent, remote placements, SQLite
one shared database, several operators with grants, a TUI, agent kinds.

The note is not a requirement of v1 (operator, 2026-11-08: "that last ask was just a follow up
potential design adjustment"). The candidate adjustments it suggests, each small and the operator's
call, are: a decision scope on `Grant`; the `Stall` finding, `thresholds.stall` and two KPIs (median
decision wait, stall hours).

## 17. The Claude app as a UI: option A

Decided on 2026-11-08 (§13, row 11).

| option | what | cost |
|---|---|---|
| A (taken) | the `claude-code` kind can start an agent for Remote Control on a named owner's token from the pool (`roles.controller.token_owner`), so an instance's controller appears in that owner's Claude app | the owner's seat carries the controller; one more flag on the kind and one lease rule; part of `story:token-pool` |
| B (not taken) | the Claude app is not a v1 UI; people steer through the dashboard, the TUI and `ask` | the note's primary way of steering is not offered |
