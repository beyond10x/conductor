# Conductor: the next architecture

Status: proposed 2026-11-07 by the teacher session from the operator's request of the same day; to be
decided by the operator. Supersedes `roles-and-surfaces.md` §4 (the bridge session) and §5 (isolation
through per-instance config directories), the dashboard stories of `epic:host-bridge`, and the
substrate stories of `epic:session-confinement`. Keeps `roles-and-surfaces.md` §2, §3 and §6 (levels,
roles, tool surfaces), `conductor-run.md` (the loop as a program) and `self-improvement.md` (KPIs).
The public page is `website/docs/concepts/architecture.md`; this document carries the reasoning.
Checked against the atlas ADRs on 2026-11-07 (`~/beyond10x/atlas/architecture/adr/`, see §9a) and
against takt, loom and mantle as they are at origin/main of the same day.

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

## 2. The shape

```text
control host                                   placements (local, substrate, mantle)
┌──────────────────────────────────────┐       ┌─ instance A, in a box ───────────────┐
│ conductord                           │       │ conductor instance serve             │
│  reconciles instances from config    │──────►│   loop (no model)                    │
│  global API  /v1/...                 │◄──────│   instance API /v1/... (socket)      │
│  federates instance APIs             │       │   controller, workers, custom roles   │
│  serves the dashboard app, once      │       │   store (authoritative) + cache       │
│  hosts supervisor and teacher        │       └──────────────────────────────────────┘
│  mirror of every instance's store    │       ┌─ instance B, remote substrate ───────┐
│  conductor.yaml (desired state)      │──────►│ …                                    │
│  secrets (sealed, never in config)   │       └──────────────────────────────────────┘
└──────────────────────────────────────┘
      ▲ dashboard (browser)   ▲ conductor tui   ▲ conductor <cmd> (CLI)   ▲ operator chat session
```

Four components, each with one job:

| component | one per | job | replaces |
|---|---|---|---|
| **conductord** | control host | reads the config, reconciles instances (desired = config, actual = what runs), starts and stops instance runtimes at their placement, owns the global API, federates the instance APIs, serves the dashboard app once, starts the host roles (supervisor, teacher), keeps a read-only mirror of every instance's store | `host/instances.json`, `conductor_<i>-run.service`, `conductor_<i>-dashboard.service`, the bridge session of `roles-and-surfaces.md` §4 |
| **instance runtime** (`conductor instance serve`) | instance | the loop (`conductor-run.md`), the instance API, the session launcher for the instance's controller, workers and custom roles; writes the instance's store | `conductor run`, the per-instance dashboard server (its routes become the instance API) |
| **placement** | instance, and per role | where and how the runtime and its sessions are confined: `host` (today's systemd units, named explicitly), `substrate` (local or remote substrate-daemon), `loom` (run-to-exit agents in a cluster, through Agency's API where it exists), `mantle` (a whole remote worker) | the `confinement.provider` enum; `epic:session-confinement`'s substrate stories; the runner seam of `design:conductor-on-agency` |
| **UIs** | any number | the dashboard (one web app over the global API), the TUI (`conductor tui`), the CLI (`conductor <cmd>` as an API client), the operator's chat session (through the supervisor) | the per-instance dashboards; `conductor ask` becomes `/v1/ask`, routed by the supervisor |

The roles are unchanged from the public page: supervisor (host, one), teacher (host, one, above the
supervisor and outside the system), controller (one per instance), workers (built-in roles planner,
implementer, reviewer, releaser), custom roles on a schedule. The maintainer is retired: conductor's
repository is a repository of one instance.

## 3. conductord: reconciling instances

The daemon is `conductor daemon serve`, installed as the user unit `conductord.service` (or a system
unit on a server). It owns the config file. Its loop:

```text
read config ─► desired instances (name, placement, store, roles, settings)
observe     ─► actual instances (what answers on each placement; what the mirror last saw)
reconcile   ─► start what is desired and absent; stop what runs and is gone from config;
               restart what changed placement or binary; carry on what matches
record      ─► one record per reconciliation action in the daemon's own store
```

- Reconciliation runs on a config change (the daemon watches the file and takes `PUT /v1/config`),
  on a placement event (an instance runtime stops answering), and every `cadence.reconcile`
  (default 60 s). It is idempotent: a second pass with nothing changed does nothing.
- The daemon never runs a model. A judgement it needs (an instance that fails to start three times)
  is a finding for the supervisor, which it starts and keeps running the way the loop keeps a
  controller today.
- Removing an instance from the config stops its runtime and keeps its store mirror and records;
  deleting data is a separate command (`conductor instance purge <name>`, class H, operator's yes).
- The daemon's own state (reconciliation log, mirror index) is a store like any instance's (§6).

## 4. APIs and federation

### 4.1 Instance API

Every instance runtime serves `/v1/` on a Unix socket inside its box (`<runtime>/api.sock`), and on
the placement's transport when the box is remote (substrate exposes it through its own session
transport, §5). The routes are today's dashboard routes, renamed and with the same semantics:

| today (`/api/...`) | next (`/v1/...`) | writes |
|---|---|---|
| `board`, `repositories`, `pull-requests`, `workflow-runs`, `releases`, `reviews` | the same | no |
| `events?since=`, `unanswered`, `decisions`, `series`, `kpis`, `waiting-on`, `sessions`, `blockers`, `specifications`, `health` | the same | no |
| `plan` GET/POST, `plan/story` | the same | POST |
| `config` GET/POST | `settings` GET/PUT: the instance's effective settings and their level | PUT |
| `answers` POST | the same | POST |
| (none) | `sessions/{name}/attach` (WebSocket, a PTY stream), `sessions/{name}/deliver` POST, `sessions/{name}/restart` POST, `store/events?since=` (the eventlog, for the mirror) | yes |

Writes carry the token the daemon minted for the instance at start, the same rule as today's dashboard
token. The instance API never reaches another instance.

### 4.2 Global API

`conductord` serves `/v1/` on `127.0.0.1:<daemon_port>` (default 7300) and, when configured, on a TLS
listener for a remote dashboard. Three kinds of routes:

| kind | routes | how |
|---|---|---|
| the daemon's own | `instances` (list with placement, state, binary version, last seen), `instances/{name}` (start, stop, restart, upgrade), `config` GET/PUT (validated, written, committed, reconciled), `secrets` (names only), `reconciliations`, `health` | the daemon's store and the config |
| proxied | `instances/{name}/*` → the instance's `/v1/*` | one connection per instance, held by the daemon; the dashboard token is translated so the browser holds one token, the daemon's |
| federated | `sessions`, `decisions`, `unanswered`, `kpis`, `events` (SSE), `ask` | the union of every instance's answer, each row carrying `instance`; `ask` is delivered to the supervisor, whose answer names its route (`→ <instance>/<repository>`, the rule of `roles-and-surfaces.md` §4) |

An instance that does not answer is a row with `state: unreachable` and its last mirror time, never an
error for the whole view.

This API is conductor's application API, not a second Agency. ADR 0105 (atlas, accepted 2026-10-22)
makes conductor an application on Agency's API that "starts runs, and reaches teams and tool bindings,
through Agency's API instead of starting sessions itself" (`adr/0105:50-52`), and keeps "remote, cluster
and swarm runs" in Loom (`:96`). conductord therefore owns instances, config, federation and the
mirror; where Agency is deployed, a run that Agency can start goes through it (the `loom` placement,
§5); the direct placements exist for the host and for the gaps Agency still has (`conductor-on-agency.md`
§3.2, G1 to G8).

### 4.3 UIs over the APIs

- **Dashboard:** today's Vue app, built once, served by the daemon at `/`; its data calls go to the
  global API. The instance picker is the `instances` route. The per-instance pages are the proxied
  routes. A dashboard can also be deployed elsewhere and pointed at a daemon URL and token.
- **TUI:** `conductor tui` (and `conductor` with no arguments at a terminal) reads the global API;
  attach is the `sessions/{name}/attach` stream rendered in the terminal; `s` edits settings through
  `settings` PUT; it runs on any machine that reaches the daemon.
- **CLI:** `conductor status`, `conductor session list|attach|deliver`, `conductor decision …` become
  API clients (`--daemon <url>`, default the local socket). Commands that today open the store
  directly keep working against a local placement for one release, then go through the API.
- **The operator's chat session** stays what it is: it talks to the supervisor, which uses the API.

## 5. Placements and confinement

Conductor implements no sandbox. A placement names what runs the instance runtime and its sessions,
and what confines them. The `confine::Provider` trait (`crates/conductor/src/confine/mod.rs`) stays
the seam; `systemd` and `none` stay as the `host` placement; the new providers are substrate's, loom's
and mantle's. ADR 0109 (atlas, accepted 2026-10-27) already decides the direction: confinement is one
Substrate contract, `ConfinementProfile` with one adapter per mechanism, and "conductor's `systemd-run`
… move[s] onto Substrate in later hops" (`adr/0109:84-85`).

A placement is set per instance and may be overridden per role: an interactive, long-lived session
(the controller, the supervisor) needs a PTY, messages and attach, which `substrate` and `mantle` give;
a unit of work that runs to an exit (an implementer on one story, takt on one case) fits `loom`, which
gives no attach and no messages.

| placement | runs where | confinement | transport to the daemon |
|---|---|---|---|
| `host` | the control host, today's units and slices | systemd (cpu weight, memory, tasks), no write fence | Unix socket |
| `substrate` with `daemon: local` | the control host, inside a substrate workspace | substrate's `linux`/`systemd` adapter: bubblewrap plus a delegated cgroup; read-only roots, writable workspace, cpu/memory/pids/storage limits, default-deny egress with named capabilities (`~/beyond10x/substrate/README.md:225-226`, `ROADMAP.md:27`) | substrate's Unix socket (`substrate-daemon --socket`) |
| `substrate` with `daemon: <url>` | a remote host running `substrate-daemon` | the same adapters on that host; `oci` where it ships; Docker and Firecracker when substrate ships those drivers (`ROADMAP.md:30-31`) | HTTPS/WSS with an Identity token (`ROADMAP.md:28`; `substrate-daemon --tcp-listen --tcp-bearer-file`) |
| `loom` | one pod per `LoomAgent` in the namespace a kubeconfig names, reconciled by `b10x-loom-operator` (`~/beyond10x/loom/crates/loom-operator/src/reconcile.rs:1664-1669`) | the resource's `boundary`, seccomp/apparmor, NetworkPolicy; secrets only as `secretRef`/`secretFiles` (credentials are refused at admission); Substrate inside the pod is not there yet (`dependency-blocker:loom-child-confinement` in takt) | the kube API (through Agency's API when it exists, ADR 0105; directly as the interim D2 runner of `conductor-on-agency.md` §4 until then); results as `status.state`, `status.run.exitCode` and the workspace volume |
| `mantle` | a whole worker (EC2, KubeVirt) that mantle provisions | mantle's manifest (`kind: Session`): placement class, repositories, agent kind, resources, network capabilities (`~/beyond10x/mantle/examples/substrate.yaml`) | mantle's SSH-forwarded socket (`docs/design/00-mantle-on-substrate.md:29-32`) |

What the daemon passes to substrate for one instance (the `ConfinementProfile` admitted through
`substrate_host::confinement::open(AdapterRequest)`, `crates/substrate-host/src/confinement/mod.rs:142`,
or the SDK's `client.workspace()` builder):

- argv and cwd: `conductor instance serve --instance <name>`, the instance's records directory;
- read-only roots: the conductor binary and its profiles, the harness binary, the instance's
  checkouts root when `checkouts.trees` is elsewhere;
- writable places: today's six (`confine/places.rs:84-101`: checkout, trees, build cache, scratch,
  harness state, records), which become one substrate workspace with subdirectories;
- limits: cpu, memory, pids, storage, from the role's `limits` and the instance's `confinement.limits`;
- egress: the forge hosts of the instance's sources, the model API host, and nothing else; `network:
  open` stays accepted for the `host` placement only;
- secrets: every credential the roles need (forge token, harness token, the dashboard token) as a
  sealed memfd slot (`README.md:375-378`), never in argv or env. The daemon resolves them from its
  secrets source (§7) at start.

Rules:

- A placement that cannot give what the profile asks for is a refusal with a name
  (`exec.sandbox-unavailable`), handled like today's `ProviderUnavailable`: the instance does not start
  and the daemon records why. Nothing falls back to `host`: ADR 0109 says "`local` is a configuration,
  not an absence … Nothing falls back to it" (`adr/0109:63-65`), and a host policy may refuse it
  (`:85-86`). `host` is written in the config on purpose, with a reason, or not at all. Today's
  `confinement.mode: report` keeps its meaning for one release and then goes.
- Sessions inside the box are substrate exec or PTY sessions (`get_pipe_session`), one per role
  session, started by the instance runtime through the same SDK; attach is the PTY stream carried by
  the instance API. tmux stays the terminal multiplexer inside the box so that today's attach, deliver
  and the terminal log keep working unchanged.
- The daemon keeps `substrate-daemon` running on the control host (`epic:process-supervision` C5),
  and treats a remote substrate like any unreachable instance: a row, not an error.
- Loom is a run-to-exit placement: a `LoomAgent` of the `Command` kind is a Job that ends with an exit
  code, takes no message after start and has no attach (`~/beyond10x/loom`: no pty or tmux in the crate
  tree; `docs/design/loom-agent-crd-deployment.md:325-331`); an `AgentServe` agent hosts Loom's own
  turns and A2A, not a Claude Code session. So `loom` runs workers whose unit is one run: the program is
  takt (`takt run --case …`, exits 0 to 9, `website/docs/concepts/how-a-run-ends.md`) or an
  implementer on one story; the controller reads the exit code and takt's `events.jsonl` from the
  workspace volume, and an `ApprovalRequired` exit (4) becomes a decision request and a new generation.
  What conductor still carries for this placement: the repository in (a PVC or claim) and the branch
  out (`git bundle`, pushed from the box's forge route), which the 2026-10-26 trial did by hand
  (`conductor-on-agency.md` §5). The approved Loom design `effect-isolation` lists a remote worker as its
  option C and names mantle for it (`~/beyond10x/loom/.engineering/planning/architecture-design/effect-isolation.md:209-219`).
- Mantle is a coarse placement: mantle owns the agent process and PTY, and lists sessions only from
  its own local state (`mantle --help`). Conductor uses it for a whole instance when the instance's
  sessions should live on a worker nobody else touches; the instance runtime is the agent mantle
  starts, and the instance API rides mantle's forwarded socket. The Glovebox design in atlas
  (`~/beyond10x/atlas/docs/design/glovebox.md`, draft 2026-10-29) is the closest existing design for
  a remote, confined session and names the model token as the only secret in the box (its slot
  `claude`, `:53-55`); this design follows it.

## 6. State: four kinds, four homes

The operator's question, "is config also state?", has the answer: config is *desired* state and is
not kept in the store; the store keeps what *happened*, including every config change.

| kind | examples | lives | authoritative copy | survives the box |
|---|---|---|---|---|
| config | `conductor.yaml`: instances, placements, roles, settings, secret names | with the daemon, in a git repository | the file | yes |
| records (the store) | decisions, dispatches, readings, findings, charters, hand-offs, snapshots, config changes, reconciliations | with the instance that wrote them, through the `Store` trait | the instance's store; the daemon holds a read-only mirror | the mirror does; the instance's copy does when its backend is outside the box |
| runtime state | sessions, sockets, envelopes, leases, terminal logs, the watch's files | inside the box, under `<runtime>/` | the box | no; rebuilt from records at the next start |
| cache | exports of `origin/main`, build caches, repository caches | inside the box | none | no |

### 6.1 The `Store` trait and its backends

Today's store is an eventlog tree, one immutable file per event, indexed when opened
(`conductor.md` §10). The trait is not conductor's to invent: ADR 0041 makes Eventlog the shared
persistence port with both adapters (`adr/0041:17-24`), ADR 0053 says a store provider "sit[s] on
Eventlog … rather than growing a second implementation" (`adr/0053:13-15`), and ADR 0063 fixes the
eventlog-tree with "file, SQLite and PostgreSQL wire bytes unchanged" (`adr/0063:29`). So the `Store`
trait is Eventlog's port, and the three backends are Eventlog's adapters, opened by `store:`:

| backend | `store:` value | for |
|---|---|---|
| `tree` | `tree://<path>` (default: `<state>/tree`) | today's form; a local placement; git-committable records |
| `sqlite` | `sqlite://<path>` | one file per instance inside the box or on the control host; the mirror's form |
| `postgres` | `postgres://…` (the URL names a secret, never a password) | a long-lived database several instances and the daemon share; an instance on a remote placement that must not keep its records in the box |

The trait is what the eventlog already offers: append an event, read events since an id, open the
indexed views. Every backend is append-only; a view is derived. The records directory
(`decisions/YYYY-MM.jsonl`, `charters/`, `docs/handoff/`, `rules.md`) stays a human-readable
projection written by the CLI, committed by the daily tick, and is not the store.

### 6.2 Where a remote instance's state lives

The operator's doubt ("the state being here on my machine, but the Claude sessions deployed
remotely") resolves into two supported arrangements, chosen per instance by `store:`:

1. **Store in the box, mirror at home (default).** The instance writes `sqlite://` inside its
   workspace. The daemon pulls `store/events?since=` through the instance API into its mirror
   (`<daemon state>/mirror/<instance>.sqlite`) every `cadence.mirror` (default 30 s) and on every
   event notification. The UIs read the mirror. If the box is lost, the mirror is the backup and the
   next start of the instance is seeded from it (`conductor instance restore <name>`). Nothing
   leaves the control host but the sessions' work.
2. **Shared database.** The instance and the daemon both open `postgres://`; no mirror is needed;
   the database's own durability applies. For an organization that already runs one.

Runtime state never travels: a session on a remote placement is attached through the API stream.

## 7. Config and secrets

- One file, `conductor.config/2`: `version`, `daemon` (port, TLS listener, `cadence.reconcile`,
  `cadence.mirror`, the secrets source, the dashboard's served path), `globals` (defaults every
  instance starts from: roles with model and profile, cadence, thresholds, store backend,
  confinement limits), `instances[]` (name, `placement`, `store`, sources, checkouts, records, roles,
  overrides of any global key). Precedence: role over instance over global. `conductor config show
  --effective --instance <i>` prints every key with its level. A `conductor.config/1` file is read
  unchanged for one release: `dashboard_port` is ignored with a warning, `confinement.provider`
  maps to `placement: host`.
- Secrets never enter the file: the file names them (`forge_token: secret:gitlab-bot`), the daemon
  resolves names from its secrets source at start, and passes them as substrate slots. The source is
  behind one trait with three implementations, and the ecosystem has already decided the first two:
  Connectors for access tokens (ADR 0107: a consumer "obtains a current access token … never the refresh
  token", `adr/0107:51-54`), the Secrets service for custody (ADR 0023: encrypted values, grants, a
  value-free audit, `adr/0023:24-34`), and an encrypted local file for a single host with neither,
  named explicitly in `daemon.secrets`. Which one a deployment uses is the operator's (class H).
  ADR 0110 (proposed) adds the rule this design keeps: "An agent's own process holds no credential for
  an external system" (`adr/0110:62-64`); forge writes keep going through the gates route, and the only
  slot in a session's box is the model token (as in Glovebox).
- Every accepted `PUT /v1/config` is validated, written, committed to the config repository with the
  caller and reason, recorded in the daemon's store as `config-changed`, and reconciled. The tuner's
  settings proposals (`self-improvement.md`, the public page §"Self-improvement over settings") use
  this route inside their grant.

## 8. Roles under the daemon

| role | started by | where | note |
|---|---|---|---|
| supervisor, teacher | the daemon, `globals.roles` | the control host, `host` or local `substrate` placement, in `conductor_host.slice` | the teacher only when the operator asks; it reviews the supervisor from outside |
| controller (one per instance) | the instance runtime at start | the instance's box | picks work across the instance's repositories; starts workers |
| workers, custom roles | the controller or the schedule, through the runtime | the instance's box, one tree each | reports to the controller |
| releaser (conductor's repository) | its controller | its box | after a release, the daemon's `instances/{name}/upgrade` installs the binary at each placement, one instance at a time, under the supervisor's grant; rollback on a runtime that does not come up |
| worker on a `loom` placement | the controller, as a `LoomAgent` `Command` | a pod | run-to-exit; its exit code and events are its report; a message to it is a new generation |

The star stays: workers to their controller, controllers to the supervisor, the supervisor to the
operator. The daemon talks to runtimes, never to sessions.

## 9. What this supersedes, and what stays

| artifact | verdict |
|---|---|
| `roles-and-surfaces.md` §4 the bridge session; `epic:host-bridge` stories bridge-session, ask-bridge-route, ask-direct, dashboard-ask-input, host-dashboard-home, host-dashboard-instances, host-instances-list, status-all-instances | superseded: the daemon's federated API and the one dashboard do this; `ask` stays as the route `/v1/ask` delivered to the supervisor; `mailbox-operator-messages` stays |
| `roles-and-surfaces.md` §5 isolation through per-instance `CLAUDE_CONFIG_DIR` and binary pins | superseded by placements; the per-instance binary pin becomes `instances/{name}/upgrade` |
| `epic:session-confinement` stories substrate-client, substrate-tool-server, substrate-gate-provider, session-writes-bwrap | superseded by the `substrate` placement (§5); session-forge-credentials is answered by secret slots |
| `epic:conductor-run` (the loop daemon, shipped in part) | stays: the loop is the instance runtime's core |
| `epic:multi-instance-host` host ledger and grants | stays for the `host` placement; a substrate placement gets limits from the profile instead |
| `epic:process-supervision` C5 | stays, done by the daemon |
| `design:conductor-on-agency`, `epic:agency-runner`, `epic:dispatch-runner` | untouched; a dispatch runner is a concern inside the instance runtime |
| the public page's "Next architecture" of the same day (one picture, TUI, settings, custom roles, tuner) | kept; this document adds the daemon, the APIs, placements and the store |

## 9a. The atlas decisions this rests on

Read on 2026-11-07 in `~/beyond10x/atlas/architecture/adr/`. atlas also records conductor as paused
(`.engineering/planning/story/mantle-substrate-pin-m1.md:14`); this design is the resumption.

| ADR | decision | effect here |
|---|---|---|
| 0105 conductor is an application on Agency (2026-10-22) | conductor starts runs through Agency's API; remote, cluster and swarm runs stay in Loom | conductord is conductor's control plane for instances, not a second Agency; the `loom` placement goes through Agency when it exists (§4.2, §5) |
| 0106 the product layer | conductor is not a product; Applications layer | nothing in this design is a product surface; the dashboard is conductor's own UI |
| 0109 confinement is one Substrate contract (2026-10-27) | `ConfinementProfile`, one adapter per mechanism; conductor's `systemd-run` moves onto Substrate; nothing falls back to `local` | the `substrate` placement; `host` explicit, no fallback (§5) |
| 0108 Loom owns the agent spec | `loom.agent/1`, the `LoomAgent` CRD and its controller; A2A served by Loom | the `loom` placement writes `LoomAgent` resources, nothing of its own (§5) |
| 0041, 0053, 0063 Eventlog is the persistence port | file, SQLite and PostgreSQL adapters, no second implementation | the `Store` trait is Eventlog's port (§6.1) |
| 0023, 0107, 0110 (proposed) credentials | Secrets custody, Connectors access tokens, no external credential in an agent process | the secrets source trait and the one-slot rule (§7) |
| 0093 a Substrate daemon is a Connectors Connection | intended, not implemented | a remote `substrate` placement may later be addressed as a Connection instead of a URL and token |
| `docs/design/glovebox.md` (atlas, draft) | a remote confined session for takt through mantle | the model for `mantle` and the one-secret rule |

What the surveys found about the candidates, so nobody reads them twice:

| candidate | what it gives conductor | what it does not |
|---|---|---|
| substrate 0.16.1 (`~/beyond10x/substrate`) | adapters `local`, `linux`, `macos`, `systemd`, `oci`, `pod`; a daemon with Unix socket and HTTPS serving; a Rust SDK and CLI; sealed secret slots; `exec.sandbox-unavailable` as a named refusal | Docker and Firecracker drivers (roadmap); a stable wire contract ("development, not stable") |
| mantle 0.3.1 (`~/beyond10x/mantle`) | a whole session on an EC2 or KubeVirt worker from a `Session` manifest; `start`, `attach`, `status`, `stop`; the daemon socket forwarded over SSH | a programmatic API per turn; a session list beyond its own local state |
| loom 0.57.2 (`~/beyond10x/loom`) | `LoomAgent` reconciled into a Job, Deployment or CronJob by `b10x-loom-operator`; `status.run.exitCode`; A2A for serving agents; `AgentExecutor` trait with "a remote executor" named as possible (`crates/loom-commission/src/ports/executor.rs:37`) and not implemented | attach, PTY, a message into a running `Command`; a host or VM placement; Substrate inside the pod |
| takt 0.34.0 (`~/beyond10x/takt`) | one program that drives one case to a named exit (0 to 9); `.takt/cases/<id>/events.jsonl`; runs as a Loom `Command` | an agent or runtime of its own; a dispatcher (designed, not built) |

## 10. Migration, each step reversible

| step | change | reversible by | story |
|---|---|---|---|
| 1 | `conductor daemon serve`: reconcile instances from the config (start and stop today's run units), `GET /v1/instances`, the registry file becomes derived | stop the daemon, run the units by hand | story:daemon-reconcile |
| 2 | instance API: the dashboard routes become `/v1/` on the runtime's socket; the daemon proxies `instances/{name}/*` and serves the dashboard app once; `-dashboard.service` retired, `dashboard_port` ignored with a warning | start the old dashboard unit | story:instance-api |
| 3 | federated views (`sessions`, `decisions`, `unanswered`, `kpis`, `events`, `ask`) and `conductor tui` read-only | nothing to undo | story:federated-views, story:tui-readonly |
| 4 | `globals` with precedence, `config show --effective`, `PUT /v1/config` with commit and `config-changed` records | keys stay valid at the instance level | story:config-globals |
| 5 | the store opened through Eventlog's port with the `tree` and `sqlite` adapters; `store:` key; the daemon's mirror and `instance restore` | `store: tree://` | story:store-trait, story:store-mirror |
| 6 | `placement: substrate` local: substrate-daemon kept running, the profile from §5, secret slots from the daemon's secrets source (Connectors, Secrets, or the local file); `host` placement unchanged and explicit | `placement: host` | story:placement-substrate-local, story:secrets-source |
| 7 | `placement: substrate` remote over HTTPS with Identity; the instance API over substrate's transport; attach as a stream | local placement | story:placement-substrate-remote |
| 8 | the `postgres` adapter | sqlite | story:store-postgres |
| 8a | `placement: loom` for run-to-exit workers: `LoomAgent` `Command` through Agency's API where it exists, else the kube API; takt as the program; exit code and events as the report | substrate placement | story:placement-loom |
| 9 | host roles started by the daemon in `conductor_host.slice`; the maintainer retired into an instance; `instances/{name}/upgrade` with rollback | the old task lines | story:host-roles-by-daemon, story:instance-upgrade |
| 10 | `placement: mantle` | substrate placement | story:placement-mantle |
| 11 | the tuner with an empty grant | an empty grant | story:tuner |

Steps 1 to 5 change nothing about where sessions run; the host keeps working as today throughout.

## 11. Open questions for the operator

| question | default without an answer |
|---|---|
| ADR 0105 says conductor starts runs through Agency; Agency's API lacks what a run here needs (G1 to G8). Do the direct placements (`substrate`, `mantle`, kube API for `loom`) go in now, with Agency taking over run starts when its API has them? | yes: direct now, `loom` through Agency as soon as its API starts a `Command` run and reports its exit |
| Is `loom` for run-to-exit workers only, with controllers and the supervisor on `substrate` or `mantle`? | yes |
| For a remote instance, is the authoritative store in the box with a mirror at home (§6.2 arrangement 1) or a shared database (2)? | 1; 2 when `store: postgres://` is set |
| Is the dashboard served by the daemon or deployed separately? | by the daemon at `/`, and deployable elsewhere against a daemon URL |
| The secrets source: Connectors, the Secrets service, or an encrypted local file? (class H) | the local file on a single host; Connectors where it is deployed |
| Does one controller per instance keep a tree per repository, or does it work through the workers' trees only? (from the public page) | workers' trees only |
| Which keys go into the first tuning grant? (from the public page) | none; the tuner proposes and the brief shows them |
