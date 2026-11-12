# Design: conductor builds through slot

Status: proposal, agreed between the maintainers of both sides, 2026-11-12; the four questions of
§ 9 are open with the operator. No store record yet. slot is the host build-slot daemon of the
`beyond10x/slot` repository (docs: https://beyond10x.github.io/slot/).

Paths without a crate are under `crates/conductor/src/`, at `812cb885e`. slot paths are prefixed
`slot:` and describe slot `main` at tag 0.10.0 (`0db3d10`). Live checks ran against a slot daemon
0.5.0 and client 0.6.0 and say so. A slot feature not built at 0.10.0 is marked "planned".

Words: **slot** is the daemon. A **conductor build slot** is one of `thresholds.build_slots`. A
**substrate slot** (`placement.rs:26-31`) is a credential file of substrate and unrelated to both.

## 1. Why

Two programs admit builds on one host, and neither knows the other. `conductor build run`
reserves disk in the host ledger and counts its own build slots; slot queues every `cargo` and
`go` build an agent shell runs, through its PATH shim, against one jobserver pool. On a host with
both, a build that conductor admitted waits a second time in slot's queue while it holds its disk
grant, a conductor build slot and its repository's target lock; conductor's CPU controls are
stripped or bypassed by slot; and `slot cargo test` passes conductor's guard without touching the
ledger. This document divides the work between the two and fixes the names slot builds to.

## 2. Today

### 2.1 conductor

- **The host ledger.** `conductor.host.HostGrant`s in a store in the host directory, `host/`
  beside the config file, one for every instance of the file (`host.rs:1-20`,
  `config.rs:1584-1603`). Its lock is `host/host.lock` (`host.rs:97`, `host.rs:544-552`); each run
  holds `host/host-runs/<grant>.lock` from request to release (`host.rs:459-488`). A grant's
  record: `crates/conductor-model/src/host.rs:196-238`. Nothing expires by time: every pass first
  releases grants whose process group is gone (`host.rs:657-706`).
- **`conductor build run`** (`build.rs:1-107`, `build.rs:815-1469`). Takes `host.lock`, sweeps,
  records the request (`build.rs:828-874`); `--reserve auto` (and none) takes the largest peak of
  the repository and command key in 14 days plus 50 %, at least 1G, else 4G (`build.rs:690-741`).
  Arrival order with backfill; a waiting run says `queued: position <n> … waiting for disk` or
  `… for slots: all <n> build_slots are taken` (`build.rs:919-939`). One reclaim at a time,
  outside the lock, reused for 5 min (`build.rs:2509-2658`). A refusal prints the shortfall, may
  record one operator item per fall of free disk, and exits 2 with the command never run
  (`build.rs:1157-1225`, `build.rs:496-502`); so does a request whose session stopped
  (`build.rs:954-964`).
- **The run.** `nice -n 19 ionice -c3 <command>` (`build.rs:1473-1496`) in a process group of its
  own (`build.rs:1276-1277`). A watchdog `SIGSTOP`s the group under `gate_stop` and reclaims,
  `SIGCONT`s at `gate_resume`, never kills (`build.rs:2382-2476`). A stopping signal is forwarded
  to the group, `SIGKILL` after 10 s (`build.rs:2168-2206`). At exit: peak, pauses, cache counts,
  release `Exited` or `Killed` (`build.rs:1388-1468`). Run time is the grant's start to release.
- **Shared target directory.** For a cargo-based command (`build.rs:1524-1538`) `build run` sets
  `CARGO_TARGET_DIR=<cache>/target/<key>`, the key the origin URL's last step and an FNV hash of
  the URL (`build.rs:1670-1692`). The lock is conductor's own, `<cache>/target/.locks/<key>.lock`,
  taken after the grant and held until the command ended: cargo's lock covers only its build
  step (`build.rs:93-107`, `build.rs:1758-1808`). It names the command's process group, so an
  orphaned command still blocks (`build.rs:1895-1902`). A `CARGO_TARGET_DIR` already set is kept,
  unlocked (`build.rs:1724-1730`). `conductor gc` keeps, not refuses, a target whose lock is held
  or whose group runs (`gc/targets.rs:1-7`, `gc/targets.rs:88-113`).
- **CPU controls.** `thresholds.build_slots` (default 3, `config.rs:367-368`) bounds `build run`
  grants of every instance of the file at once; a head that fits waits for a free one and is never
  refused for one (`build.rs:899-1016`, `host.rs:745-763`). `thresholds.short_lane_slots`
  (default 1, `config.rs:379-380`) admits a command known short (3 runs in 14 days, p90 at most
  `short_lane_max`, default 3 min) ahead of longer requests, capped to 4 jobs (`build.rs:81-92`,
  `build.rs:749-785`, `build.rs:2151-2161`). `thresholds.cpu_reserve` (default 2,
  `config.rs:364-365`) sets each build's share, `(cores - cpu_reserve) / build_slots`, written as
  `CARGO_BUILD_JOBS` and `MAKEFLAGS=-j<n>` (`build.rs:2090-2147`). Sessions run in the instance's
  slice with its `MemoryMax`, `CPUWeight`, `TasksMax` (`systemd.rs:130-157`). `conductor
  resource` records and enforces nothing (`resource.rs:1-16`). Nothing else counts or delays.
- **Guard.** In a controller's session a compiling `cargo`, a building `task` or an npm build or
  test not under `conductor build run` is denied: "`<run>` builds, tests or gates outside the
  host's disk ledger: run it as `conductor build run --reserve auto -- <run>` …"
  (`guard/rules.rs:1641-1652`, `guard/rules.rs:2645-2717`). The program word is found past
  launchers (`env`, `nice`, `timeout` and nine more, `guard/rules.rs:2512-2540`); `slot` is none,
  so `slot cargo test` passes and bypasses the ledger, while a shimmed `cargo test` is denied.
- **Dashboard and metrics.** `/metrics`: `conductor_build_queue_length`, the `Requested` disk
  grants (`daemon/metrics.rs:21-23`). Dashboard: `build_slot` is the constant `"not recorded"`
  (`dashboard.rs:1686`); series of disk grants granted, refused, waited and peak against reserve
  (`dashboard/series.rs:230-257`); waits on `build-slot`, `build-queued` (`dashboard/repo.rs:55-60`,
  `watch.rs:88-90`); K8 "waiting hours" (`dashboard/series.rs:680`). Nothing shows a slot job.
- **Placements.** A host instance's sessions run in its systemd slice on the host, with the shim
  on `PATH` when the operator installed it; no unit conductor writes sets a network property
  (`systemd.rs:130-157`; no `PrivateNetwork` or `IPAddressDeny` in the crate), so they share the
  host's network namespace. A substrate box writes only the instance's state, records, cache and
  store directory (`placement.rs:1279-1320`): neither the host directory nor `$XDG_RUNTIME_DIR`
  is mounted, so a boxed `build run` reaches no shared ledger and a boxed `slot` client no daemon
  (not run). Conductor asks substrate for `NetworkEgress` with one aperture, the model host
  (`placement.rs:101-106`, `placement.rs:209-226`); whether the box gets its own network
  namespace is substrate's (not verified). A local substrate daemon refuses `oci` and `pod`
  (`placement.rs:5-8`). A Kubernetes pod mounts one Secret and one volume claim, no host path, and
  takes the limits of `daemon.kubernetes.resources` (`daemon/pods.rs:404-468`); the image has no
  `cargo` and no `slot` (`Dockerfile`). A pod cannot reach a host daemon.
- **Identity in a build.** The provider sets `CONDUCTOR_SESSION` to the session's name
  (`actor.rs:23-24`) and `CONDUCTOR_INSTANCE` names the instance (`config.rs:203`); `build run`
  records `CLAUDE_CODE_SESSION_ID` (`build.rs:222`, `build.rs:864`) and leaves the session name
  empty (`build.rs:2235`). Nothing records that the operator is attached to a session or talked to
  it last; `conductor session attach` is a `tmux attach` on the session's socket (`attach.rs:1-7`).
  The daemon types its deliveries into a session as prompts, each starting with a tag of the
  message grammar: `DISPATCH`, `REPORT`, `DECISION-REQUEST`, `DECISION`, `NEED`, `RESOURCE`,
  `ESCALATION`, `HANDOVER` (`message.rs:97-175`).

### 2.2 slot

- `slot <command…>` and `slot run -- <command…>` send the command over the unix socket
  (`$SLOT_SOCKET`, else `$XDG_RUNTIME_DIR/slot.sock`, `slot:crates/slot/src/paths.rs:5-8`) to the
  daemon, which queues it, **runs it itself** in a process group of its own with the client's
  environment, streams the output back, and the client exits with its code
  (`slot:crates/slot/src/server/mod.rs:845-887`). The job runs in `slot.service`'s cgroup (under
  `--memory-max` in a scope in `slot.slice`, `server/mod.rs:848-866`), not the caller's, at the
  daemon's nice (0) with no ionice class; its CPU priority is `CPUWeight=20` on the unit
  (`slot:crates/slot/src/cli.rs:107-110`) and the headroom's `CPUQuota`.
- Admission: jobserver tokens by toolchain; memory (`slot serve --min-free-memory`, default 4 GB,
  `cli.rs:133-137`); and a disk floor, `statvfs` free bytes of the job's working directory and of
  its `CARGO_TARGET_DIR` against `--min-free-disk`, default 25.0 GB (`cli.rs:129-132`). A job
  below either waits; it is not refused (`server/mod.rs:780-815`,
  `slot:crates/slot/src/admission.rs:130-167`). slot does not read conductor's ledger.
- slot strips `MAKEFLAGS`, `MFLAGS`, `CARGO_MAKEFLAGS` and `CARGO_BUILD_JOBS` from a job and sets
  its jobserver (`slot:crates/slot/src/toolchain.rs:66`, `toolchain.rs:130-135`); every other
  variable of a local client reaches the job (`server/mod.rs:630-640`).
- A job's environment has `SLOT_JOB=<id>`; a `slot` client or the shim that sees it runs the
  command directly, inheriting the outer job's jobserver: no second queue entry
  (`slot:crates/slot/src/client.rs:138-141`, `client.rs:358-372`). The job's `PATH` is the
  client's, so a shimmed `cargo` inside it runs the real cargo.
- `jobs.jsonl` in slot's state directory (`$XDG_STATE_HOME/slot/`, `--state-dir`) has one line
  per finished job, the fields of `/api/state` below; a job running when the daemon died has none.
- A client that cannot reach the daemon prints `daemon unreachable … ran locally` and runs the
  command itself, outside the pool (`client.rs:323-329`). A daemon reloading re-executes only
  while no job runs, holding admission for at most 60 s at a time; a job lost before it started is
  resent once by its client (`slot:crates/slot/src/server/upgrade.rs:1-9`, `client.rs:318-355`).
  A client whose daemon dies mid-job prints `slot: connection to daemon lost` and exits 3, which a
  job's own exit 3 cannot be told from (`client.rs:526-537`); the unit's `KillMode=mixed` kills
  the job with it (inferred from the unit, not run), and a job in a `--memory-max` scope is left
  running. A hand-over re-exec that keeps running jobs is planned.
- A client that gets a signal, or goes away, cancels its job: a queued one ends 130, a running
  one gets `SIGTERM`, `SIGKILL` after 5 s (`server/mod.rs:790-800`). `slot cancel <id>` works for
  the submitting uid only (`server/mod.rs:643-654`).
- HTTP: `slot serve --http`, default `127.0.0.1:7411`, `off` turns it off while the socket keeps
  working. `SLOT_HTTP` is the clients' variable (`status`, `headroom`, `sessions`). Routes:
  `GET /api/state`, `/api/events`, `/api/jobs/{id}/output`, `/api/history`, `/metrics`,
  `GET`/`PUT /api/limits` (`slot:crates/slot/src/server/http.rs:25-30`). `PUT /api/limits` has no
  authentication, only an `Origin` check (`slot:crates/slot/src/server/limits.rs:138`). Live,
  daemon 0.5.0: `GET /api/version` is 404; a job in `/api/state` has id, argv, cwd, repo, session,
  client, toolchain, remote, tokens, state, times, exit, CPU, peak RSS, `wait_reason`; no labels.
- slot's harness hook allows any `slot …` and any program it does not know, `conductor` included.
- Remote: `SLOT_HOST` sends a job to a build server over ssh, without `CARGO_TARGET_DIR`
  (`toolchain.rs:193`) or credential-like names `SLOT_ENV` does not list (0.7.0); `.slot.toml`
  `local = […]` (0.7.0); fallback to the local daemon (0.8.0); target seeding by reflink and
  `--cache-budget` (0.9.0); `slot shell` (0.10.0). A job records `remote`, not the host.

### 2.3 Both on one host, today

`conductor build run -- cargo test` in a controller's shell with the shim first on `PATH`:

1. Ledger request; wait for disk and a conductor build slot. Held: a queue position.
2. Granted. Held: the reservation and one conductor build slot.
3. Wait for the repository's target lock. Held: 2.
4. Start the repository's sccache server; spawn `nice -n 19 ionice -c3 cargo test` with
   `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS`, `MAKEFLAGS` and the cache variables; `start_run`
   marks the grant `Running`.
5. `cargo` resolves to the shim, which `exec`s `slot run -- <real cargo> test`: the group leader
   is now the slot client.
6. Wait in slot's queue (tokens, memory, disk floor). Held: 2 and 3; the ledger counts it as run.
7. slot runs cargo in its own group and cgroup, without `CARGO_BUILD_JOBS` and `MAKEFLAGS`, at
   nice 0. cargo's own lock is taken here.

So the build is admitted twice (a conductor build slot, then slot tokens), in two orders. The
conductor share of the cores, the short lane's 4-job cap, nice and ionice reach only the slot
client. The watchdog's `SIGSTOP` stops the client and not the build (`server/mod.rs:887`), so the
disk gate does not hold for these builds today; whether the job then blocks on a full output pipe
is not verified. Cancel works: a signal to `build run` reaches the client, which cancels the job.
`slot conductor build run …` queues as a generic job holding one token, and the grant and the
target lock are then waited for inside it. `conductor build run -- slot cargo test` is not
cargo-based to `build run` (`slot` is no launcher of `command_key`, `build.rs:550-557`), so it
gets no shared target, no compile cache, no lean debug information, and its peaks are filed under
`slot cargo`.

## 3. Division of ownership

| concern | owner | conductor's part | slot's part |
|---|---|---|---|
| CPU admission, jobserver, headroom, call mode | slot | none while slot answers; `build_slots`, short lane and the job share only as the fallback | all |
| memory admission | slot | the instance slice's limits for sessions, not builds | `--min-free-memory`, expected peak |
| disk reservations | conductor | the ledger, reclaim, refusal, operator item | its floor only for jobs with no grant (§ 6) |
| shared target directory and its lock | conductor | as today | none |
| which session may build | conductor | the guard | its hook, for shells conductor does not guard |
| which build goes first | slot, from classes | `background` for its daemon steps (§ 4) | the class, then arrival |
| compile cache | conductor per repository | as today | its sccache watchdog |

Conductor detects slot per `build run` by the socket: `slot` on `PATH` and `slot status` exiting 0
within 1 s. The socket is what submit uses; HTTP may be off. No answer: the fallback, today's
behaviour unchanged.

## 4. The build path

One `conductor build run -- <command>` with slot answering:

1. **Inside a slot job?** `SLOT_JOB` set: refuse, exit 2, "conductor build run runs outside slot:
   drop the `slot` prefix: run `conductor build run -- <cmd>`". Held: the one token of the outer
   generic job, until the refusal. The guard also denies `slot … conductor build run` (§ 6), since
   slot's hook allows any `slot …`.
2. **Detect slot** (§ 3). Held: nothing.
3. **Disk.** Ledger request and grant as today, counting no conductor build slot and no short
   lane. Held while waiting: a queue position.
4. **Target lock** for a cargo-based command, as today. Held while waiting: the reservation.
5. **Submit.** Exec `slot run --strict --report <file> --label … [--priority background] --
   <command>` in the run's own group, with no `nice` or `ionice` before it: they would lower only
   the client. Not set: `CARGO_BUILD_JOBS`, `MAKEFLAGS`. Set as today: `CARGO_TARGET_DIR`, the
   cache and debug variables. `start_run` records the group. Held while waiting in slot: the
   reservation and the target lock.
6. **Run.** slot runs the job; a shimmed `cargo` or nested `slot` inside it sees `SLOT_JOB` and
   runs directly. The watchdog measures as today; under `gate_stop` it runs `slot pause --label
   conductor.grant=<id>` and at `gate_resume` `slot resume …` (§ 5, item 8).
7. **Release.** The client exits with the job's code. `build run` reads `{id, wait_ms, run_ms,
   exit, end}` from the report file (item 13), records the queue wait and the end on the grant,
   then releases as today. No HTTP read.

Why this order: CPU tokens are machine-wide and the scarcest at any instant; a token held while
waiting for disk or a repository lock stalls every build on the host. A reservation held while
queued for CPU blocks only other disk-starved requests, and the target lock only other trees of the
same repository, which would wait for it anyway. So the token is taken last, and the only wait
under the reservation and the lock is slot's queue. The lock also keeps `conductor gc` off a target
whose build is still queued.

Labels conductor sets on every job (names fixed here; keys within slot's `[a-z0-9._-]+`):

| label | value | source |
|---|---|---|
| `conductor.instance` | the instance's name | `CONDUCTOR_INSTANCE`, else the active config |
| `conductor.repository` | `--repository`, else the repository name | the request |
| `conductor.grant` | the grant id | the ledger; set only under the filesystem condition of § 6 |
| `conductor.session` | the session's name | `CONDUCTOR_SESSION`; absent outside a conductor session |

A controller is a session, so no `conductor.controller`. slot's own `session` field keeps the
harness session id. Conductor passes labels with `--label`, never `SLOT_LABELS`, so a value may
hold a comma.

Priority: conductor passes `background` for a build of a daemon step (`CONDUCTOR_DAEMON_STEP=1`)
and nothing otherwise; slot then applies `normal`, or `interactive` when it infers one (§ 9, 3).
slot runs a `background` job at nice 19 in the idle I/O class (item 6). Conductor does not pass
`interactive`: its only signals, an attached tmux client and the deliveries it types, are weaker
than slot's prompt record (an attached operator may only watch; most deliveries are the daemon's
relays), and two sources would need a rule for when they disagree. Conductor owes the inference
the tags of its deliveries instead (§ 6), so the hook does not count them as the operator's.

## 5. What conductor needs from slot

Each item carries slot's answer.

1. [ ] `GET /api/version` → `{version, api: "1"}`; a breaking change to any route below bumps
   `api`. **Agreed.**
2. [ ] A published, versioned document of the routes: each route, its query parameters, the job
   schema with optional fields, a job's states, ends and exit codes, the error bodies, what
   `api: "1"` promises. **Agreed:** OpenAPI 3.1 from slot's types, `GET /api/openapi.json` and a
   release asset `slot-<v>-openapi.json`.
3. [ ] `slot run --label k=v` (repeatable) and `SLOT_LABELS=k=v,k=v`; labels in `jobs.jsonl`,
   `/api/state`, `/api/events`, `/api/history`; `?label=k=v` on those routes, repeated as AND.
   **Agreed**, keys `[a-z0-9._-]+`; a value with a comma only through `--label`.
4. [ ] Per finished job: `wait_ms`, `run_ms`, `exit`, `end` = `exit | signal | cancelled{by:
   client|cancel|label} | lost`. **Agreed**; at start slot records `lost` for jobs left with no
   end line.
5. [ ] A job labelled `conductor.grant` skips the disk floor; memory admission stays.
   **Agreed**, under the filesystem condition of § 6.
6. [ ] `--priority interactive|normal|background` and `SLOT_PRIORITY` at submit;
   `PUT /api/jobs/{id}/priority` for a queued job, socket and loopback only; order by class, then
   arrival, no aging in v1; a `background` job runs at nice 19 and ionice idle. **Agreed.**
7. [ ] `slot cancel --label k=v`, same uid. **Agreed.**
8. [ ] `slot pause|resume <id>|--label k=v`: stops and continues the job's processes except the
   compile servers, since an sccache server a job started lives in its group and serves other
   jobs. **Agreed with change** (conductor asked for `SIGSTOP` to the group). A paused job keeps
   its tokens: from `gate_stop` to `gate_resume` the pool is that many tokens smaller, for as long
   as the fall of free disk lasts. Accepted: a job that gave them back would resume over the pool.
9. [ ] `GET /api/limits` stays (the headroom read). **Agreed.**
10. [ ] `SLOT_STRICT=1` or `slot run --strict`: daemon unreachable before start → exit 75, nothing
    ran; daemon lost after start → exit 76, for every client, replacing today's 3. **Agreed with
    change** (the name; conductor proposed `SLOT_LOCAL_FALLBACK=0`).
11. [ ] The web page accepts `?label=k=v`. **Agreed.**
12. [x] slot's hook allows `conductor build run -- …`. **Already true**; slot adds a test.
13. [ ] `slot run --report <file>`: when the client exits, write `{id, wait_ms, run_ms, exit,
    end}` to the file. **Agreed** (slot's proposal; replaces a label query over HTTP).
14. [ ] A token for writes over HTTP: the daemon creates `$XDG_RUNTIME_DIR/slot/token` (mode
    0600) at start; `PUT /api/limits`, priority, pause, resume and cancel over HTTP need
    `Authorization: Bearer <token>`; the unix socket stays the unauthenticated owner channel;
    reads stay open on loopback. slot's web page gets its writes through a cookie set by a
    one-time link `slot ui` prints, so the page never embeds the token. **Agreed.**

HTTP submit is not needed: conductor execs `slot` for submit and is no second client of the socket
protocol. After item 13 `build run` reads nothing over HTTP; the dashboard does (§ 8, phase 6).

## 6. What slot needs from conductor

- The labels and the `background` class of § 4, set by `build run` only; no other conductor
  command submits.
- `conductor.grant` only on a granted run whose working directory and `CARGO_TARGET_DIR` are on the
  filesystem of `thresholds.disk_path` (same device): the ledger counts only that filesystem, and
  slot's floor stays the guard of any other.
- No `CARGO_BUILD_JOBS` or `MAKEFLAGS` from `build run` when slot answers (slot strips them).
- No `build run` inside a slot job (§ 4 step 1), and the guard denies `slot … conductor build
  run`: slot's hook allows any `slot …`, so conductor's guard is the only one that can.
- The guard treats `slot` and `slot run` as launchers: `slot cargo test` outside `build run` is
  denied with today's message plus "it then waits in slot's queue".
- The slot socket is never mounted into a substrate box: slot runs a job outside the caller's box,
  so a boxed session with the socket would run commands unconfined.
- No box shares the host's network namespace: today anything on the host's loopback can change
  the headroom. Recommended: both slot's token (item 14) and boxes with a namespace of their own,
  conductor failing one that shares the host's. Today it asks for `NetworkEgress` with one aperture (`placement.rs:101-106`,
  `placement.rs:209-226`) and checks no namespace (the box's namespace: not verified). A host
  instance's sessions are unboxed and share it (§ 2.1); item 14 closes that for them.
- With item 14: conductor never reads `$XDG_RUNTIME_DIR/slot/token` for a sandboxed session and
  never mounts the runtime directory's `slot/` into a box. `build run` needs no token: submit,
  pause and resume go through the `slot` command on the socket, and it reads nothing over HTTP.
- The prompt prefixes: slot's prompt hook skips a prompt whose first non-blank line starts with
  any line of `${XDG_CONFIG_HOME:-~/.config}/slot/prompt-prefixes.d/*` (one literal prefix per
  line, `#` comments allowed); slot ships its own default list for prompts a harness injects.
  Conductor writes `prompt-prefixes.d/conductor` at install and upgrade, one line `[<TAG>` per
  tag of the message grammar (`message.rs:97-175`): its deliveries are not the operator's.
- (d) of § 9 question 2 sees a session only where slot's hook is in its harness config;
  `slot sessions` lists live sessions whose config lacks it, so the gap is visible.
- Later (§ 9 settled 1): the live reserved bytes, in a file in the host directory, for slot's floor.

## 7. Failure cases

- **slot daemon down at the request.** `slot status` fails; the fallback: `build_slots`, the
  short lane and the job share as today, and the command run without `slot run`. A shimmed `cargo`
  then runs locally through slot's own fallback.
- **slot dies after detection, before start.** The strict client exits 75; `build run` runs the
  command under the fallback, keeping its grant and lock. Without `--strict` (today) the client
  resends once or runs locally, outside the pool and outside conductor's count.
- **slot dies mid-job.** The job dies with the unit (inferred); the strict client exits 76, and
  `build run` releases the grant `Killed` and says the daemon was lost. Today the code is 3.
- **slot upgrade mid-build.** Re-exec waits for running jobs (holds of at most 60 s); the grant
  and lock wait longer. Nothing to do. **Ledger refusal:** exit 2 before slot sees anything.
- **Cancel.** A signal to `build run` reaches the client, which cancels the job (works today).
  `slot cancel` of a conductor job ends the client 130 or with the job's signal code; `build run`
  records the report's `end` (`cancelled{by: cancel}`) on the grant. Stopping an instance cancels
  `conductor.instance=<name>` (item 7).
- **Watchdog.** It never kills. Under `gate_stop` it pauses through slot (item 8); before
  phase 4 the pause stops only the client, as today, and the paused line says so.
- **Remote mode.** With `SLOT_HOST` the job writes on the build server: the grant reserves local
  disk the job does not use, the shared target is not sent, and `--reserve auto` learns near-zero
  peaks. § 9, question 3.

## 8. Phases

Each ships alone; each later one needs the slot items named.

1. **Correctness, conductor only.** Guard: `slot` and `slot run` are launchers, `slot …
   conductor build run` is denied, the denial adds the slot sentence. `build.rs`: `slot` is a
   launcher for `command_key` and `cargo_based`. `build run` refuses inside `SLOT_JOB` with the
   fix named. No admission changes.
2. **Delegate CPU, today's slot.** Detection by `slot status`; with slot answering, no conductor
   build slots, no short lane, no job variables, no nice or ionice before the client, the command
   wrapped in `slot run --`; the fallback otherwise. The dashboard links "slot queue" to slot's
   page when served from the same host, and says the link is local-only.
3. **Strict and report** (items 10, 13, 4). `--strict` with the 75 and 76 handling; the queue
   wait and the end on the grant (a field added in `spec/domains/host.yaml`); K8 counts it.
4. **Labels and version** (items 1-3, 5, 7, 8). Labels on submit when `api` is `"1"`; the grant
   label skips slot's floor; the watchdog pauses by label; an instance stop cancels by label.
5. **Priority** (item 6). `background` for daemon steps; `conductor` install and upgrade write
   `prompt-prefixes.d/conductor` (§ 6).
6. **Panel** (items 3, 11). A read-only dashboard panel, slot's queue and this instance's jobs
   from `/api/state?label=conductor.instance=<name>`; the link opens on the instance's jobs.

## 9. Open questions

Settled with slot's maintainer and moved into the text: jobs with no grant compete with
conductor's reservations, accepted now, with the reserved-bytes file of § 6 later; `build run`
inside a slot job is refused (§ 4 step 1).

1. **Per-instance CPU share** (the operator's). A job slot runs is outside the instance's slice,
   so the instance's `CPUWeight` and `MemoryMax` no longer bound its builds. Options: (a) accept,
   one host pool; (b) slot weights jobs by `conductor.instance`; (c) keep conductor's
   `build_slots` per instance as a cap beside slot. Recommendation: (a).
2. **Sessions the operator talks to** (the operator's). Options: (a) the operator's own shells set
   `SLOT_PRIORITY=interactive`; (b) slot treats a job with no `conductor.*` label as
   `interactive`; (c) slot infers it from a terminal; (d) slot's harness hook records each
   operator prompt per session, and jobs of a session prompted in the last 10 min are
   `interactive`, which covers sessions conductor did not start. Recommendation: (d), with (a) as
   a manual override; conductor then passes only `background` (§ 4). (d) reaches a conductor
   session only where slot's hook is installed in it (`slot sessions` shows where it is not), and
   skips conductor's deliveries by the prefix file of § 6.
3. **Remote mode with a grant** (the operator's). Options: (a) `build run` removes `SLOT_HOST` so
   conductor's builds stay local; (b) with `SLOT_HOST` set, no grant, no target lock, a
   `conductor.remote` label; (c) as today. A job records `remote` but not the host.
   Recommendation: (a), shared by slot's maintainer.
4. **The grant label only on the ledger's filesystem** (the operator's). Deferring to the ledger
   for every granted job would drop slot's floor for a target on a filesystem other than
   `disk_path`, which the ledger never reads. Slot's maintainer agreed to the condition of § 6;
   listed for the operator's acceptance.
