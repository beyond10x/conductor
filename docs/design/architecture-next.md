# Design: conductor v1 — a daemon, confined instances, abstract agents, one store

Status: proposed 2026-11-07; revised 2026-11-08 on the operator's decisions (§13). Store record:
`design:architecture-next`, epic `epic:architecture-next`. The public page is
`website/docs/concepts/architecture.md`. The work lands on the integration branch `v1` (§14); the
shipping line, v0, is untouched until the cutover.

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
and remote; agents abstracted with today's Claude Code behaviour kept; SQLite everywhere with sync;
the dashboard served by the daemon and deployable remotely at the same time; secrets through the
`secrets` crates; everything built now, on one integration branch, v0 to v1.

## 2. The shape

```text
control host                                      placements (substrate: local or remote daemon)
┌────────────────────────────────────────┐        ┌─ instance A, a substrate box ───────────────┐
│ conductord                             │        │ conductor instance serve                    │
│  reconciles instances from config      │───────►│   loop (no model)                           │
│  global API  /v1/...                   │◄───────│   instance API /v1/... (socket)             │
│  federates instance APIs               │        │   agents: controller, workers, custom roles │
│  serves the dashboard app              │        │   store: sqlite (authoritative)             │
│  hosts supervisor and teacher          │        └─────────────────────────────────────────────┘
│  sync copy of every instance's store   │        ┌─ instance B, a box on a remote substrate ───┐
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
| **conductord** | control host | reads the config, reconciles instances (desired = config, actual = what runs), opens and closes each instance's box at its placement, owns the global API, federates the instance APIs, serves the dashboard app, starts the host roles (supervisor, teacher), keeps a synced copy of every instance's store | `host/instances.json`, `conductor_<i>-run.service`, `conductor_<i>-dashboard.service`, the bridge session of `roles-and-surfaces.md` §4 |
| **instance runtime** (`conductor instance serve`) | instance | the loop (`conductor-run.md`), the instance API, the starter of the instance's agents; writes the instance's store | `conductor run`, the per-instance dashboard server (its routes become the instance API) |
| **placement** | instance, overridable per role | a Substrate box, from a daemon on this host or a remote one, with an adapter (bubblewrap, a container, a pod) and a profile | the `confinement.provider` enum, `confine/systemd.rs`, `confine/none.rs`, the host ledger's slices |
| **agent kind** | role slot | what program fills a role and how conductor starts, feeds, reads and stops it; `claude-code` in v1, with today's behaviour | the one hard-wired `claude --bg` launcher |
| **UIs** | any number | the dashboard (one web app over the global API, served by the daemon and deployable elsewhere), the TUI (`conductor tui`), the CLI (`conductor <cmd>` as an API client), the operator's chat session (through the supervisor) | the per-instance dashboards; `conductor ask` becomes `/v1/ask`, routed by the supervisor |

The roles are unchanged from the public page: supervisor (host, one), teacher (host, one, above the
supervisor and outside the system), controller (one per instance), workers (built-in roles planner,
implementer, reviewer, releaser), custom roles on a schedule. The maintainer is retired: conductor's
repository is a repository of one instance.

## 3. Specification first: the v1 domains

Operator, 2026-11-08: "the whole system as visible in the design/stories/pub docs as proposed needs to
be modelled in ESS". The specification (`spec/`, `ess 0.90`) is the source of truth for every noun,
command, view and error below; `crates/conductor-model` is generated from it; a handler is written
against the generated types (`AGENTS.md` rule 1). The first story on `v1` is the model
(`story:spec-v1`); every other story depends on it and starts by reading its domain.

| domain | status in v1 | holds |
|---|---|---|
| `conductor.instance` | new | `Instance` (desired: name, placement, store, roles, settings; actual: state, binary version, last seen, box), `Reconciliation` (one record per action: start, stop, restart, upgrade, nothing), commands `Reconcile`, `StartInstance`, `StopInstance`, `RestartInstance`, `UpgradeInstance`, `PurgeInstance` (class H), views `Instances`, `Reconciliations`. Replaces the registry types of `conductor.host` (`InstanceRegistry`, `RegisteredInstance`, `RegisteredFile`, `DashboardInstance`, `InstanceCollision`) |
| `conductor.placement` | new | `Placement` (substrate daemon: local socket or URL with a bearer; adapter; profile: read-only roots, writable workspace, limits, egress, secret slots), `Box` (opened, closed, refused with the substrate reason), commands `OpenBox`, `CloseBox`, errors `PlacementRefused` (named: `exec.sandbox-unavailable` and the others substrate declares). Replaces `confinement.provider`, `confinement.mode` and the systemd provider's places |
| `conductor.agent` | new | `AgentKind` (`claude-code` in v1), `Capabilities` (`interactive`: PTY, attach, deliver, hand-over; `run-to-exit`: an exit code and an events file), `Agent` (a role slot filled: kind, placement, profile, limits, state), commands `StartAgent`, `DeliverToAgent`, `AttachAgent`, `StopAgent`, events `AgentStarted`, `AgentReported`, `AgentStopped`. `conductor.session` becomes the `claude-code` kind's behaviour, kept scenario for scenario |
| `conductor.store` | new | `Backend` (`sqlite` only), `Sync` (per instance: last event id copied, last sync time, lag), commands `SyncInstance`, `RestoreInstance`, views `StoreSync`. The records directory stays a projection |
| `conductor.secret` | new | `SecretName` (what the config names), `Source` (`keychain`, `remote`), `Slot` (a sealed slot handed to a box), command `ResolveSecrets` at box open, view `Secrets` (names and sources, never values) |
| `conductor.operator` | new | `Actor`, a sum type: `human` (an `Operator`, an Identity principal) or `agent` (an `Agent` of `conductor.agent`: instance, role, name, kind); `Operator`, `Grant` (answer decisions, change settings, start, stop and upgrade instances, purge, join); every command, event and record carries its `Actor`; `JoinInstance`, `LeaveInstance`; views `Operators`, `Actors`. The instance cluster has several operators (operator, 2026-11-08); a human and an agent are told apart by type, never by name (operator, 2026-11-08: "we have the actors, and we kind of need to distinguish between who is an agent actor and who is a human actor") |
| `conductor.token` | new | `TokenPool` (per instance), `PoolToken` (owner, seat, state `ok` or `limited until`), commands `ContributeToken`, `WithdrawToken`, `LeaseToken`, `ReleaseToken`, `MarkTokenLimited`, view `Tokens` (owners, seats, states, never values); the `claude-code` kind's lease per agent start (§5.3) |
| `conductor.config` | changed | `Level` (global, instance, role) and precedence, `EffectiveSettings` view, command `PutConfig` (validated, written, committed, reconciled) with event `ConfigChanged`; `dashboard_port` and `confinement.*` removed; `conductor.config/2` |
| `conductor.improvement` | changed | `TuningGrant` (keys with bounded ranges; empty by default), a proposal applied only inside the grant through `PutConfig` |
| `conductor.host` | shrunk | keeps grants, disk reservations, build runs and reclaim (the host ledger for boxes on this host); loses the registry, the dashboard instance and the host-roles config |
| `conductor.observation`, `conductor.direction`, `conductor.decision`, `conductor.dispatch`, `conductor.tool` | unchanged | the loop's facts, goals, decisions, dispatches and tools |

**Actors.** Every command the specification accepts names who issued it, and the type says whether
that is a person or a program: `Actor::Human(operator)` or `Actor::Agent(agent)`. An agent actor is
the agent the runtime started (its instance, role, name and kind), so a decision a controller took,
a report a worker wrote, a message the supervisor sent and a guard denial each name the program and
its role; a human actor is an operator with grants, so an answer, a config change, a join and a
purge each name the person. A grant belongs to an operator only; an agent holds none, and a command
that needs a grant refuses an agent actor by name (`ActorNotHuman`). Every event and every record
carries its actor, every view shows it, and the records projection writes it as `human:<name>` or
`agent:<instance>/<role>/<name>`; the `human:` prefix is the one AEP's transitions already write. The
design's class H decision is the one only a human actor may answer; class C is the one an agent may
take and record (§ 5 of the conductor design).

Components (`spec/components.yaml`): `conductord` (owns instance, placement, store, secret, config),
`conductor-instance` (owns observation, direction, decision, dispatch, agent, improvement, tool),
`conductor-cli`, `conductor-dashboard` and `conductor-tui` as clients that accept views and the few
writes the UIs make (answers, settings inside the grant). The two APIs are the components' accepted
commands and views, projected to OpenAPI by `ess`.

## 4. conductord: reconciling instances

The daemon is `conductor daemon serve`, installed as the user unit `conductord.service` (or a system
unit on a server). It owns the config file. Its loop:

```text
read config ─► desired instances (name, placement, store, roles, settings)
observe     ─► actual instances (what answers at each placement; what the sync last saw)
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
- Removing an instance from the config stops its runtime and keeps its synced store and records;
  deleting data is `conductor instance purge <name>`, class H, the operator's yes.
- The daemon's own state (reconciliations, sync index) is a store like any instance's (§8).
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
| (none) | `agents/{name}/attach` (WebSocket, a PTY stream, interactive kinds only), `agents/{name}/deliver` POST, `agents/{name}/restart` POST, `store/events?since=` (the eventlog, for the sync) | yes |

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

An instance that does not answer is a row with `state: unreachable` and its last sync time, never an
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

## 8. State: config and store

The operator's question, "is config also state?", has the answer: config is *desired* state and is
not kept in the store; the store keeps what *happened*, including every config change.

| kind | examples | lives | authoritative copy | survives the box |
|---|---|---|---|---|
| config | `conductor.yaml`: instances, placements, roles, settings, secret names | with the daemon, in a git repository | the file | yes |
| records (the store) | decisions, dispatches, readings, findings, charters, hand-offs, snapshots, config changes, reconciliations | with the instance that wrote them | the instance's SQLite file; the daemon holds a synced copy | the synced copy does |
| runtime state | agents, sockets, envelopes, leases, terminal logs, the watch's files | inside the box, under `<runtime>/` | the box | no; rebuilt from records at the next start |
| cache | exports of `origin/main`, build caches, repository caches | inside the box | none | no |

**One backend: SQLite, everywhere, with sync** (operator, 2026-11-08: "remote -> sqlite, local too -
with sync"). The store is opened through Eventlog's port (ADR 0041: Eventlog is the shared persistence
port; ADR 0053: a store provider sits on Eventlog rather than growing a second implementation; ADR
0063: the wire bytes are the same across its adapters) with its SQLite adapter only. Every instance
writes `<runtime>/store.sqlite` inside its box; the daemon writes `<daemon state>/store.sqlite` for
its own records and `<daemon state>/sync/<instance>.sqlite` for each instance, pulled through
`store/events?since=` every `cadence.sync` (default 30 s) and on every event notification. The UIs
read the daemon's copies. If a box is lost, the synced copy is the backup and the next start of the
instance is seeded from it (`conductor instance restore <name>`). The records directory
(`decisions/YYYY-MM.jsonl`, `charters/`, `docs/handoff/`, `rules.md`) stays a human-readable
projection written by the CLI, committed by the daily tick, and is not the store.

Today's eventlog tree (one file per event) is read once by `conductor store migrate` into the SQLite
file and then not opened again; the tree backend, the `store:` URL scheme and a database backend are
not in v1.

## 9. Config and secrets

- One file, `conductor.config/2`: `version`, `daemon` (port, TLS listener, `cadence.reconcile`,
  `cadence.sync`, the secrets source, the dashboard's served path), `globals` (defaults every instance
  starts from: roles with kind, model and profile, cadence, thresholds, limits), `instances[]` (name,
  `placement`, sources, checkouts, records, roles, overrides of any global key). Precedence: role over
  instance over global. `conductor config show --effective --instance <i>` prints every key with its
  level. A `conductor.config/1` file is read by `conductor config migrate` once: `dashboard_port` and
  `confinement.*` are dropped with a note, `host.roles` becomes `globals.roles`.
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
  settings proposals (`self-improvement.md`, the public page §"Self-improvement over settings") use
  this route inside their grant, which is empty until the operator writes keys into it (§13).

## 10. Roles under the daemon

| role | started by | where | kind in v1 |
|---|---|---|---|
| supervisor, teacher | the daemon, `globals.roles` | the control host, a local substrate box | `claude-code`; the teacher only when the operator asks; it reviews the supervisor from outside |
| controller (one per instance) | the instance runtime at start | the instance's box | `claude-code`; picks work across the instance's repositories; starts workers |
| workers, custom roles | the controller or the schedule, through the runtime | the instance's box, one tree each | `claude-code`; another kind per slot once it exists |
| releaser (conductor's repository) | its controller | its box | after a release, the daemon's `instances/{name}/upgrade` installs the binary at each placement, one instance at a time, under the supervisor's grant; rollback on a runtime that does not come up |

The star stays: workers to their controller, controllers to the supervisor, the supervisor to the
operator. The daemon talks to runtimes, never to agents.

## 11. What v1 removes, and what this supersedes

Removed from the product (operator, 2026-11-08: "the more we remove the better"):

| removed | replaced by |
|---|---|
| the per-instance dashboard server and `conductor_<i>-dashboard.service`, `dashboard_port` | the instance API and the daemon's one dashboard (§5) |
| `host/instances.json`, the registry, `conductor config validate`'s prefix check against it | `Instances` derived from the config (§3, §4) |
| `conductor run --install`, `conductor_<i>-run.service`, `task supervisor:start`, `task teacher:start`, `task maintainer:start`, the adapters under `~/.claude/agents/` | the daemon starts every runtime and host role (§4, §10) |
| `confinement.provider`, `confinement.mode`, `confine/systemd.rs`, `confine/none.rs`, the host ledger's slices for sessions | Substrate placements (§6) |
| `conductor session start`'s hard-wired `claude --bg` launcher, `claude attach` as the attach path | the `claude-code` agent kind behind the trait (§7); attach through the API |
| the eventlog tree as a backend, the `store:` scheme, a database backend | SQLite with sync (§8) |
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
| substrate 0.16.1 (`~/beyond10x/substrate`) | adapters `local`, `linux`, `macos`, `systemd`, `oci`, `pod`; a daemon with Unix socket and HTTPS serving; a Rust SDK and CLI; sealed secret slots; `exec.sandbox-unavailable` as a named refusal; Docker and Firecracker on the roadmap; a wire contract marked "development, not stable" | **in v1**: the only placement (§6); the unstable contract is pinned by version in `b10x.toml` and checked by `conductor doctor` |
| secrets 0.6.0 (`~/beyond10x/secrets`) | `secrets-core` (storage port), `secrets-keychain` (the OS keychain as a backend), `secrets-remote` (the service as a backend), `secrets-federation` (one backend per namespace), `secrets-client` | **in v1**: the secrets source (§9) |

## 13. Decisions

Taken by the operator on 2026-11-07 and 2026-11-08, recorded here and on the public page; not
reopened.

| # | question | answer |
|---|---|---|
| 1 | teacher: own role or part of the supervisor? | its own role, above the supervisor, outside the system like an adversarial review (2026-11-07) |
| 2 | merge step by step, or one fork? | "we want then basically a fork until all of it is working" (2026-11-07); "a new integration branch in which all of the stories can go active, but not merge to default branch yet ... its basically v0 -> v1 - since v0 is shipping code right now, we dont want to interrupt it" (2026-11-08): §14 |
| 3 | Agency, Loom and takt (the proposal of 2026-11-07) | "forget entirely about agency entirely, it was just a design, tell atlas to deprecate it. then in general i think we should just rely on substrate for local and remote confinement (bwrap, docker, k8s) - and then support multiple types of 'agents' - currently we use ClaudeCode agents. but at some point a controller could also be a takt protocol, and the workers could also be various different 'executions' - we just need to abstract it. any combination possible - so abstract it - but keep exact behaviour now (claudecode-based)" (2026-11-08): §6, §7, §12 |
| 4 | a remote instance's store: in the box with a mirror, or a shared database? | "remote -> sqlite, local too - with sync" (2026-11-08): §8 |
| 5 | the dashboard: served by the daemon or deployed separately? | both at once: served by the daemon where the operator controls conductord, and deployed remotely in a cluster for colleagues (2026-11-08): §5.3 |
| 6 | the secrets source? | the `secrets` crates: the OS keychain locally, the Secrets service remotely (2026-11-08): §9 |
| 7 | the first tuning grant? | empty; the tuner proposes, the brief shows the proposals, the operator writes keys into the grant (taken by this design under the operator's rule "take the reversible option"; an empty grant is one) |
| 8 | which stories go active? | all of the epic's, on the integration branch (2026-11-08) |
| 9 | how many operators? | "multiple operators possible of the instance cluster" (2026-11-08): §3 `conductor.operator`, §5.3 |
| 10 | the daemon on a remote box; how engineers join | "conductor D, the daemon itself, is running also on the remote host ... we can connect via API. So this must of course be secured ... some engineer of us is basically joining ... an instance ... they would basically hand out their token to the pool of Claude tokens. And the agents of kind Claude Code, they would make use of these tokens" (2026-11-08): §4, §5.3, `conductor.token` |
| 11 | the Claude app as a UI? | option A: the `claude-code` kind starts a controller for Remote Control on a named owner's token from the pool, so it appears in that owner's Claude app ("yes, kind of option A. Yeah, this is option A", 2026-11-08): §17 |
| 12 | who issues a command: a person or a program? | told apart by type, `Actor::Human` or `Actor::Agent`, on every command, event and record; a grant belongs to a human actor only ("we have the actors, and we kind of need to distinguish between who is an agent actor and who is a human actor. I think it's important", 2026-11-08): §3 |

## 14. The integration branch and the order of work

The branch `v1` is cut from `main` after the design merges. Every story of the epic lands on `v1`
through a pull request against `v1`, with its own test gate (`task check` runs on it in CI);
nothing merges into `main` until the cutover. The live instance keeps `main`'s binary and units
throughout; the fork is tried on a scratch instance on this host (`conductor init` into a scratch
records directory, `example-org` sources) and on a scratch remote substrate when step 8 reaches it.

| step | story | what lands on `v1` |
|---|---|---|
| 0 | spec-v1 | the domains and components of §3 in `spec/`, the `Actor` type (human or agent) on every command, `ess specify validate`, `task regen`, one scenario per new command; the generated types every later step builds on |
| 1 | daemon-reconcile | `conductor daemon serve`: `Reconcile`, `Instances`, `Reconciliations`; the registry file gone |
| 2 | instance-api | `conductor instance serve` with `/v1/` on its socket; the daemon proxies and serves the dashboard app at `/` and on the TLS listener; the per-instance dashboard server gone |
| 3 | federated-views, tui-readonly | the federated routes; `conductor tui` read-only with attach |
| 3a | daemon-remote | the daemon as a system unit on a remote box; the TLS listener with Identity bearers as its only API; `conductor login`, `--daemon <url>`; the dashboard, the TUI and the CLI against it from a workstation |
| 4 | config-globals | `conductor.config/2`: levels, precedence, `config show --effective`, `PutConfig`, `config migrate` |
| 5 | store-sqlite, store-sync | the SQLite backend through Eventlog's port; `store migrate` from the tree; the daemon's sync and `instance restore` |
| 6 | agent-kinds | the `agent::Kind` trait with `claude-code`; today's session scenarios and tests pass unchanged; the `agents/*` routes |
| 7 | placement-substrate-local, secrets-source | a local substrate box per instance and per agent; the profile of §6; slots from the keychain |
| 7a | token-pool | `conductor join` and `leave`; the instance's token pool in the secrets source; the `claude-code` kind leases per agent start, resumes on another token after a limit stop; owners on every agent record; `tokens` on the API |
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
| C | nothing | read the forge and the store sync only; every look at code is a worker it starts | the cheapest box; a controller that asks a worker for every fact |

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
| sessions move between people and machines as files: export with checksums, import as another instance, the box's shared tools instead of the export's, a hand-over note on top | an instance moves by changing its `placement` in the config: records and hand-overs are in git, the store is synced, `instance restore` seeds the new box, the box gets the host's tools as read-only roots | **partial**: the harness state (the model's memory, transcripts) is runtime state in v1 and does not survive the box; the note's moves carried it. Either the `claude-code` kind exports and imports its harness state per agent, or memory is declared disposable and the hand-over note is the carrier |
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
sync, several operators with grants, a TUI, agent kinds.

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
