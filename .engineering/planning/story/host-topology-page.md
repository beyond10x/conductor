---
format: aep.planning-md/3
id: story:host-topology-page
kind: story
status: draft
title: concepts/the-host.md shows each placement's processes, units, sockets and directories
revision: 2
---
## Why

The operator asked, through the director of a trial instance on 2026-11-10, for a topology docs
update: what runs where on a v1 host (conductord, the instance runtime, the substrate box, the
sessions on the host), for placement none and systemd. website/docs/concepts/the-host.md has a
"what runs" table but no units, sockets or directories per placement. The trial's director wrote
the proposal below from a host running conductor 1.0.1 at placement none, and from the source at
v1.0.1; the gates were not run on it.

## Acceptance

- website/docs/concepts/the-host.md has a "Topology" section after "Placement": per placement,
  the processes, user units, sockets and directories of a running host, each row with its
  source in the frontmatter `source:` line; `task docs-check` and the site build pass.
- No row about the boxed placement is published until story:box-runtime-starts has a live run
  that confirms it; the proposal's "Not confirmed on a running host" rows are checked against
  that run, in particular the box network and cwd rows that story:box-reaches-forge-and-daemon
  contradicts.

## The proposal, as received

### Proposal: a Topology section for `the-host.md`

- Target page: `website/docs/concepts/the-host.md` of conductor, read at tag v1.0.1.
- Proposed position: a new `## Topology` after `## Placement` and before `## Shared state`. It
  uses `###` for its three parts and the page's table style. Other pages under
  `website/docs/concepts/` use fenced blocks only for yaml, json, console and a quoted message
  (`disk.md:104-108`), never for a diagram, so the section is tables only.
- Gates the developers named: `task check` (its last step is `task docs-check`) and the CI site
  build `npm --prefix website run build`. Neither was run: this file is a proposal.
- Date: 2026-11-10. Written from the v1.0.1 tag and one live host running conductor 1.0.1 with
  `placement: none`.

### Proposed text

````markdown
### Topology

What exists on a host for each placement, so that `systemctl --user`, `ss` and `ls` can be matched
to the parts above. `<state>`, `<records>` and `<cache>` are the paths `conductor config show`
prints for the instance; `<daemon dir>` is the global `--state-dir`, else
`~/.b10x/conductor/daemon`.

### A runtime with no box

A `placement` that names no adapter, such as `none`, keeps the runtime in a user unit. Nothing
else runs for it.

| part | what you see | source |
|---|---|---|
| conductord | the user unit `conductord.service`, `WantedBy=default.target`, `Restart=on-failure`, `RestartSec=10`. Its command is `conductor --config <file> --state-dir <daemon dir> daemon serve` | `conductor daemon serve --install` |
| the dashboard and `/v1/` | `127.0.0.1:<daemon.port>` (7300), a TCP listener held by conductord; no listener when `daemon.loopback: false` | conductord |
| the instance runtime | the transient user unit `conductor_<instance>-runtime.service`, made by `systemd-run --user` in the slice `conductor_<session_prefix>.slice` (`conductor.slice` without a prefix), `Restart=on-failure`, `RestartSec=30`. Its command is `<daemon dir>/binaries/<version>/conductor --config <file> instance serve`, its working directory `<state>` | conductord |
| the unit's environment | `CONDUCTOR_CONFIG`, `CONDUCTOR_INSTANCE`, `CONDUCTOR_STORE`, `CONDUCTOR_API_TOKEN`, `CONDUCTOR_API_SOCKET` and `PATH`. `systemctl --user cat` shows the token; do not paste it | conductord |
| the instance API | the Unix socket `<state>/api.sock`, held by the runtime; conductord proxies `/v1/instances/<name>/<route>` to it | the runtime |
| the loop | in the runtime's main thread, holding `<state>/run/lock` | the runtime |
| the runtime's output | the journal, `journalctl --user -u conductor_<instance>-runtime.service`. No `runtime.log` is written | systemd |
| the sessions | one `claude --bg` background session per controller, per lead and per host role, as your user, in no box. The session's settings file is `<state>/sessions/<session name>.settings.json`; it pins `CONDUCTOR_INSTANCE` and, for a role with a guard, wires `conductor guard` as a `PreToolUse` hook | `conductor session start`, `conductor spawn` |
| the host roles | `supervisor` and `teacher`, each a `claude --bg` session in the unit `conductor_<role>.service`, started by conductord for each role `globals.roles` lists, with state under `<daemon dir>/host`. With `globals.roles: []` neither runs | conductord |

`/v1/instances` shows such an instance with `"box_id": null`. `conductor doctor` ends with `daemon
unit: conductord.service running` and does not need `substrate-daemon`.

### A runtime in a substrate box

`placement: systemd` or `linux` moves the runtime out of the user unit. conductord keeps no
`conductor_<instance>-runtime.service` for it.

| part | what you see | source |
|---|---|---|
| `substrate-daemon` | a process you start and keep running, not conductord. Its socket is `$XDG_RUNTIME_DIR/substrate/daemon.sock`, or the path in `CONDUCTOR_SUBSTRATE_SOCKET` in conductord's environment | you |
| its state | the `--state` database and the `--workspaces` directory you pass; `~/.local/state/substrate/state.db` and `~/.local/state/substrate/workspaces` in [getting started](/docs/getting-started#5-start-the-substrate-daemon) | you |
| its cgroup | the delegated subtree you pass as `--cgroup-root`, carrying `cpu`, `memory` and `pids`; `systemd` needs it | you |
| its writable root | `sessions-<instance>=<state>/sessions`, one per instance with a box; the directory exists, mode 0700, before the daemon starts | you; conductord creates the directory |
| its aperture | `conductor`, which lists the forge hosts of the instance's sources and `api.anthropic.com`; the runtime's session starts with it | you |
| conductord | the same user unit and listener as above; it reads the daemon's `/v1/machine` and opens one box per instance. `<daemon dir>/boxes.json` records the boxes it opened | conductord |
| the box | one substrate workspace per instance, with the runtime a PTY session in it. `/v1/instances` shows its `box_id` | conductord |
| the runtime's command | `<daemon dir>/binaries/<version>/conductor --config <file> instance serve`, working directory `<state>` | conductord |
| the runtime's environment | set beside nothing inherited: `CONDUCTOR_CONFIG`, `CONDUCTOR_INSTANCE`, `CONDUCTOR_STORE`, `CONDUCTOR_API_SOCKET`, `PATH` and `CONDUCTOR_API_LOG=<state>/sessions/runtime.log`. `CONDUCTOR_API_TOKEN` is left out | conductord |
| mounted read-only at their own paths | the directory of the conductor binary, the directory of `claude`, and `checkouts.root` when `checkouts.trees` is not under it | conductord |
| mounted writable | `<state>/sessions` at its own path, through the root `sessions-<instance>`; and six places, each a subdirectory of the box's workspace | conductord |
| the model token | a sealed slot at file descriptor 3, filled from a slot of `daemon.substrate_slots`; the pool files are in `<daemon dir>/slots` | conductord |
| network | only the forge hosts of the sources and `api.anthropic.com` | substrate |
| the runtime's output | `<state>/sessions/runtime.log`. The journal holds conductord's lines only | the runtime |
| the sessions | `claude --bg`, unboxed, as your user, exactly as above: substrate 0.20.0 starts no agent session in a box | `conductor session start`, `conductor spawn` |

A box keeps its lease by renewing it, and its runtime past substrate's limit of one run by a restart
in the same box.

### Directories

The same for both placements.

| directory | holds |
|---|---|
| the config file, `~/.b10x/conductor/conductor.yaml` | what conductord reconciles; the directory also holds the default store `conductor.sqlite` |
| `<daemon dir>` | `api.token` (mode 0600), `runtimes.json`, `binaries.json`, `boxes.json` and `binaries/<version>/` |
| `host/` beside the config file | the disk ledger and its locks ([disk](./disk.md)) |
| the store, the URL in `daemon.store` | one database for every instance; for sqlite the file with its `-wal`, `-shm` and `.open-lock` companions |
| `<records>`, `~/.b10x/conductor/<instance>/records` | the instance's records, a git repository |
| `<state>`, `~/.b10x/conductor/<instance>/state` | `api.sock`, `run/` (`lock`, `pending.json`), `mailbox/`, `sessions/` |
| `<state>/sessions/` | per session `<name>.settings.json`, `<name>.profile.md` and `<name>.envelope.json`; `runtime.log` in a box |
| `<cache>`, `~/.cache/b10x/conductor/<instance>` | the collectors' exports |
| `checkouts.root` | the repositories the controllers work in |
| `checkouts.trees` | the managed worktrees of the controllers |
````

### Sources

Format: `fact | source | observed or from source`. Line numbers are at v1.0.1. Host paths are
written generically; the instance on the host is `<instance>`. Commands ran with
`CONDUCTOR_CONFIG`, `CONDUCTOR_INSTANCE` and `PATH` set as the brief says.

### Observed on the host (conductor 1.0.1, `placement: none`)

1. `conductord.service` is a user unit with `Restart=on-failure`, `RestartSec=10`,
   `WantedBy=default.target`, and `Environment="PATH=<redacted path list>"` | `systemctl --user cat conductord.service` | observed
2. its command is `conductor "--config" <file> "--state-dir" <daemon dir> "daemon" "serve"` | same | observed
3. the unit is active, main process is the conductor binary, journal line `serving the dashboard and /v1/ on http://127.0.0.1:7315/` | `systemctl --user status conductord.service` | observed
4. port 7315 is a TCP listener of the conductord process: `LISTEN 0 128 127.0.0.1:7315 ... users:(("conductor",pid=<conductord>,fd=16))` | `ss -ltnp | grep 7315` | observed
5. `daemon.port: 7315`, `daemon.loopback: true`, `daemon.tls: null`, `daemon.dashboard: /`, `daemon.cadence.reconcile: 60s`, `daemon.substrate_slots: 0`, `globals.roles: []` | `conductor config show` | observed
6. the runtime unit is transient: `# /run/user/<uid>/systemd/transient/conductor_<instance>-runtime.service`, description `[systemd-run] <daemon dir>/binaries/1.0.1/conductor --config <file> instance serve` | `systemctl --user cat conductor_<instance>-runtime.service` | observed
7. `Slice=conductor_<instance>.slice` (the instance's `session_prefix` is `<instance>`), `WorkingDirectory=<state>`, `Restart=on-failure`, `RestartSec=30s`, `Description=conductor instance <instance>` | same | observed
8. its environment names `CONDUCTOR_CONFIG`, `CONDUCTOR_INSTANCE`, `CONDUCTOR_STORE=sqlite:<path>`, `CONDUCTOR_API_TOKEN=<redacted>`, `CONDUCTOR_API_SOCKET=<state>/api.sock` and `PATH`; no `CONDUCTOR_API_LOG` | same | observed
9. the runtime's command is `<daemon dir>/binaries/1.0.1/conductor "--config" <file> "instance" "serve"`; its cgroup is `.../conductor_<instance>.slice/conductor_<instance>-runtime.service` | `systemctl --user status conductor_<instance>-runtime.service` | observed
10. the runtime writes its lines to the journal (`PR checks done: ...`, `session stalled: ...`) | same | observed
11. `<state>/api.sock` is a listening Unix socket: `u_str LISTEN ... <state>/api.sock` | `ss -xl | grep -i conductor` | observed
12. `<daemon dir>` holds `api.token`, `binaries`, `binaries.json`, `boxes.json`, `runtimes.json` | `ls` | observed
13. `<state>` holds `api.sock`, `broker`, `controllers.lock`, `host.migrated`, `mailbox`, `run`, `sessions`, `snapshots`, `specs`, `watch`; `mailbox/` holds a subdirectory `conductor` | `ls` | observed
14. `<state>/run` holds `ci.last`, `cycle.last`, `daily.last`, `lock`, `mailbox.handled`, `pending.json` | `ls` | observed
15. `<state>/sessions` holds `<name>.envelope.json`, `<name>.profile.md` and, for most sessions, `<name>.settings.json`; no `runtime.log` | `ls` | observed
16. the store directory holds `conductor.sqlite`, `conductor.sqlite.open-lock`, `conductor.sqlite-shm`, `conductor.sqlite-wal` | `ls` | observed
17. `host/` beside the config file holds `host.lock`, `host-runs`, `ledger.gen`, `ledger.open` (and a `store.sqlite` set) | `ls` | observed
18. the records directory holds `.git` | `ls -A <records>` | observed
19. the cache directory holds `exports`, `heads`, `scratch`, `target`; the trees directory holds one directory per repository | `ls` | observed
20. `GET /v1/instances` answers `placement: "none"`, `box_id: null`, `state: "Running"`, `binary_version: "1.0.1"`, `changed_by: {"kind": "Daemon"}`, a `roles` array | `curl -s http://127.0.0.1:7315/v1/instances` | observed
21. a session's settings file has keys `disableAllHooks`, `env`, `hooks`; `env` is `{"CONDUCTOR_INSTANCE": "<instance>"}`; one `PreToolUse` hook `CONDUCTOR_INSTANCE=<instance> conductor guard record-guard-decision --from-pre-tool-use || exit 2` | read of the keys of `<state>/sessions/<name>.settings.json` of a controller session | observed
22. `conductor doctor` ends `daemon unit: conductord.service running`, `substrate-daemon: not needed`, `ready` | `conductor doctor` | observed
23. the journal shows a refused delivery that names the launcher socket `<state>/sessions/<name>.tmux` as absent | `systemctl --user status conductor_<instance>-runtime.service` | observed

### From source (v1.0.1)

24. conductord reads `--config`/`CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml`; its directory is `--state-dir`, else `~/.b10x/conductor/daemon`; serves `127.0.0.1:<daemon.port>`; `--install` makes `conductord.service` | `daemon/mod.rs:1-13`; `the-host.md:22-24,45-47` | from source
25. the daemon's token file `api.token`, mode 0600 | `daemon/api.rs:29,318` | from source
26. `runtimes.json` and `boxes.json` | `daemon/reconcile.rs:54,61` | from source
27. the unit name `conductor_<instance>-runtime.service` | `daemon/reconcile.rs:84-89` | from source
28. the runtime's `Service`: slice, `cwd` `<state>`, env `CONDUCTOR_CONFIG`, `CONDUCTOR_INSTANCE`, store variable, `CONDUCTOR_API_TOKEN`, `CONDUCTOR_API_SOCKET`, `PATH`; `Restart=on-failure`, `RestartSec=30`; command `<binary> --config <file> instance serve` | `daemon/reconcile.rs:938-985` | from source
29. the slice `conductor_<prefix>.slice`, else `conductor.slice` | `systemd.rs:79-84`; `security/sessions.md:146` | from source
30. the runtime unit is made by `systemd-run --user`; every agent session is `claude --bg` and unboxed | `security/sessions.md:129-133` | from source
31. a runtime in a box is a substrate PTY session in the instance's workspace; with no adapter the runtime keeps the user unit | `placement.rs:11-16`; `the-host.md:100-103` | from source
32. `CONDUCTOR_SUBSTRATE_SOCKET`; default `$XDG_RUNTIME_DIR/substrate/daemon.sock` | `placement.rs:82,1026-1034`; `getting-started.md:173-174` | from source
33. substrate-daemon is started by the operator, not conductord; `--cgroup-root` carrying `cpu`, `memory`, `pids`; writable root `sessions-<instance>` created before the daemon starts; aperture `conductor`; example `--state` and `--workspaces` paths | `getting-started.md:167-192` | from source
34. the root name `sessions-<instance>` (bytes outside letters, digits, `_`, `-` written `_`) | `placement.rs:551-566` | from source
35. aperture `conductor` and its egress hosts (forges of the sources, `api.anthropic.com`) | `placement.rs:84-87,651-662,1341` | from source
36. the box's argv `<binary> --config <file> instance serve`, `cwd` `<state>`, env set beside nothing inherited | `placement.rs:513-518,621-623,666-675` | from source
37. the box env is the unit's env without `CONDUCTOR_API_TOKEN` | `daemon/reconcile.rs:878-890` | from source
38. `CONDUCTOR_API_LOG=<state>/sessions/runtime.log`; `runtime.log` is written in the sessions directory | `placement.rs:575-578,632-635`; `instance.rs:46-48`; `the-host.md:96` | from source
39. read-only mounts: binary directory, `claude` directory, `checkouts.root` when the trees are not under it | `placement.rs:519-522,636-650,1345-1350` | from source
40. writable: the sessions root at its own path; six places, each a subdirectory of the workspace | `placement.rs:523-524,538,683-686,1351-1358`; `config.rs:704-711` | from source
41. the model token slot at descriptor 3; pool files in `<daemon dir>/slots`; `daemon.substrate_slots` | `placement.rs:27-31,89-90,697-698,1359-1364`; `daemon/reconcile.rs:893-900` | from source
42. the box's lease and the restart in the same box | `placement.rs:31-32,1187` | from source
43. the runtime's `<state>/api.sock` and the hashed fallback name in the temporary directory | `instance.rs:36-44,55-57`; `the-host.md:61-64` | from source
44. `<state>/run/lock`; `<state>/run/pending.json`; `<state>/mailbox/conductor` | `run.rs:84-87`; `run/handlers.rs:18,67`; `run/mailbox.rs:57-60` | from source
45. host roles: `supervisor`, `teacher`; unit `conductor_<role>.service`; state `<daemon dir>/host`; command `session start --role <role>`; started for each role `globals.roles` lists | `daemon/host_roles.rs:38-68`; `daemon/reconcile.rs:529-570`; `security/sessions.md:16` | from source
46. the launch `claude --bg -n <name> ... --settings <state>/sessions/<name>.settings.json`; the pinned copy; settings per role (guard for `conductor` and `controller`) | `security/sessions.md:27-32,62-71` | from source
47. `<name>.profile.md` in `<state>/sessions` | `session.rs:1031-1034` | from source
48. the terminal log and the envelope are written in the sessions directory | `placement.rs:535-537` | from source
49. defaults `~/.b10x/conductor/<instance>/records`, `.../state`, `~/.cache/b10x/conductor/<instance>` | `getting-started.md:127-134` | from source
50. the default store `conductor.sqlite` under `~/.b10x/conductor/`; the records are a git repository; `checkouts.root` and `checkouts.trees` hold the repositories and managed worktrees; the cache holds the collectors' exports | `guides/run-isolated.md:42-48` | from source
51. `host/` beside the config file holds the disk ledger | `the-host.md:105-108` | from source
52. the runtime in a box needs `substrate-daemon`; `conductor doctor` checks only its version | `getting-started.md:136,167` | from source

Count: 23 observed, 29 from source.

Frontmatter `source:` additions for `the-host.md`: `run/handlers.rs`, `run/mailbox.rs`,
`systemd.rs`, `config.rs`, `daemon/api.rs` is already listed. The `source:` line names no
website pages today; if the developers want the docs behind the new rows named, also add
`website/docs/getting-started.md`, `website/docs/guides/run-isolated.md`,
`website/docs/security/sessions.md`. Also add to the description line, not required:
"the units, sockets and directories of each placement".

### Not confirmed on a running host

The boxed-placement rows rest on source and docs only. A trial on 2026-11-10 with
substrate-daemon 0.20.0 did not start the runtime in a box. Defect items are in
`conductor-defects.md` (13 to 20) in this directory.

- "The runtime's environment" row (`CONDUCTOR_CONFIG`, ... `PATH`, `CONDUCTOR_API_LOG`): the
  row matches the source, and the trial found the runtime refuses to start without `HOME`
  (item 15). Item 18: `CONDUCTOR_API_TOKEN` is left out, so daemon-proxied writes would be
  refused; the row says it is left out, the consequence is inferred.
- "Mounted read-only" and "mounted writable" rows: they match the source. The trial found the
  box mounts too little for the runtime: no config file, store, records, `run/`, `api.sock`
  (item 16). The page does not claim the runtime works in the box; the implementor decides
  whether the rows change once item 16 is fixed.
- "network" row and the box's working directory: `BoxRequest.egress` and `cwd` are built
  (`placement.rs:527,651-662`) but the trial source read found they are never sent to substrate
  (item 17), so the row "only the forge hosts ... and `api.anthropic.com`" and the box
  `cwd` `<state>` rest on the request struct and `guides/run-isolated.md:19-21`, not on a box
  that was seen. The aperture `conductor` is what the session asks for (`placement.rs:1341`).
- Crash loop (item 19) and `conductor doctor` answering `ready` for a boxed config (item 20):
  the page text says neither; item 19 also means no `runtime.log` appeared in the trial.
- "Mounted writable" and "its writable root": confirmed only as far as the daemon's
  `/v1/machine` listing `exec.writable-roots.host` (getting-started.md:194-198).
- "the runtime's output ... `runtime.log`": not seen written in the trial; the runtime did not
  start.
- "A box keeps its lease by renewing it": source only (`placement.rs:31-32`), a box never ran
  past one lease.
- "the model token ... descriptor 3 ... `<daemon dir>/slots`": source only; the trial's
  instance named no `model_token`.
- "`/v1/instances` shows its `box_id`": the field is `null` on the host observed; its value in a
  box was not read back.
- "conductord keeps no `conductor_<instance>-runtime.service` for it": inferred from
  `reconcile.rs:912-932` (a box runtime is closed through its box, else its unit is stopped).
- "a delegated cgroup ... `systemd` needs it": getting-started.md:170-172 only.

### Gaps

- `<state>` entries `broker`, `controllers.lock`, `host.migrated`, `snapshots`, `specs`, `watch`,
  `run/ci.last`, `run/cycle.last`, `run/daily.last`, `run/mailbox.handled` are seen on the host
  but their meaning was not read from a source in the scope; the section names none of them.
- The `<name>.tmux` launcher socket (`session.rs:265,322`): seen in a journal line as absent;
  whether `claude --bg` sessions have one on a unit host was not established; left out.
- `<name>.envelope.json`: only `placement.rs:535-537` ("the envelope") names it; the exact file
  name's source was not read.
- The six places of a box's workspace are named in `config.rs:704-711` (`checkout`, `trees`,
  `build-cache`, `scratch`, `harness-state`, `records`) but how they map to directories in the
  workspace was not read; the section does not list them.
- Contents of the `host/` directory beyond the disk ledger (`host-runs`, a `store.sqlite` set).
- Whether `substrate-daemon`'s own units or `--deployment`, `--allow-uid`, `--ca-bundle` flags
  belong in the topology: only the flags the brief and getting started name are used.
- Any `conductor_host.slice` use (`systemd.rs:104`) by host roles: not read.
- Systemd `--install` unit text for conductord is observed only, `daemon/install.rs` was not read.
- No live check of a `placement: systemd` or `linux` host was possible (defect items 13 to 20).
