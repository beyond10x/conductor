# What conductor gains from running its sessions and builds on substrate

Read 2026-10-09. `substrate/` is `~/beyond10x/substrate` at `2743daadc` (tag `0.7.11`); `conductor/` is `~/beyond10x/conductor` at `4a3701257`. "Observed" means a command run on this host on 2026-10-09; "inference" means it was not run.

## Summary

| need | substrate gives | plain Linux | verdict |
|---|---|---|---|
| CPU, memory, pids limits per instance and session | per-exec cgroup: memory+swap, pids up to 4096, CPU **capped at 1 core per exec**; no limit over a group of execs, no `cpu.weight` | `systemd-run --user` slice per instance, unit or scope per session: `MemoryMax`, `CPUWeight`, `TasksMax` | plain Linux. Substrate's 1-core cap would slow every cargo gate |
| write confinement (the Bash gap) | only `/workspace` writable, but only for a process running inside substrate, and an exec cannot run in an existing checkout | bubblewrap (0.12.0 installed) or Landlock around the whole `claude` process, or Claude Code's own sandbox (story `bash-sandbox`) | plain Linux. Substrate cannot host a controller today (last row) |
| disk and build quota | hard byte+inode project quotas, only on substrate workspaces and `/scratch` | `setquota -P` on trees and `target/` dirs | same prerequisite (`prjquota`, missing on `/`). Plain Linux also covers the checkouts substrate cannot |
| attribution of load and cost | exact per-exec counters: CPU, memory current/peak, processes, OOM kills, block io | the same kernel counters from cgroup v2 files | plain Linux. The hard part, one cgroup per session, is the same for both |
| lifecycle: exits, restarts, orphans | durable operations, `exec.exited` events, leases, whole-tree kill. A daemon restart kills every exec; a PTY session dies after 1 h | systemd unit: `Restart=`, exit status in `systemctl --user show`, `KillMode=control-group` | plain Linux for long sessions. Substrate's model is bounded batch execs |
| multi-instance isolation | one daemon is one trust domain: one daemon, cgroup root and quota range per instance | one systemd slice per instance plus the instance-prefixed names already planned | plain Linux |
| controllers on a second machine | TLS 1.3 HTTPS/WSS plus a remote SDK, which require a hosted Identity service. No cross-machine scheduling | `ssh` plus `systemd-run --user` on the other host | plain Linux now. Substrate only once an Identity service runs |
| observability for dashboard and watch | `/v1/metrics`, a 1 Hz stream, retained events, absent facts kept absent | cgroup files, `systemctl --user show`, `statvfs` | plain Linux. Substrate's honest labels cover only what runs inside it |
| network control | no egress by default; one operator-declared aperture per run, address pinned at startup | nothing comparable without root (nftables per cgroup) | substrate's real gain, but one aperture cannot serve the model API and the forge together |
| `claude` itself inside substrate | exec: argv only, no stdin, at most 24 h, env names containing `token` refused, HOME must be in `/workspace`, AF_UNIX denied. PTY session: killed after 1 h | runs unchanged | does not fit today. Builds fit better, but only in copied trees |
| `claude` unconfined, its tools inside substrate through an MCP server (§ 5) | per-call write confinement, no network per call, exact usage per call, a tool withheld when the host cannot confine. Nothing implements it for Claude Code; metaharness refuses substrate for vendor harnesses | bubblewrap around the session gives the write boundary, without per-call usage or network | the better substrate route, second step and for builds and tests only: git in linked trees, the forge, `worktree`, offline cargo and the loss of the shell are open |

## 1. Capability inventory

State words follow substrate's own pages. **Shipped** means served by tag `0.7.11`.

Substrate's status pages lag the tag:
- The public status page names 0.7.3 (`substrate/website/docs/status.md:12`).
- `STATUS.md` says "Observed: 2026-09-15" and names 0.7.7 (`substrate/STATUS.md:3`, `:37`).
- The changelog has 0.7.11 on 2026-10-08 (`substrate/CHANGELOG.md:10`).

The daemon installed at `~/.cargo/bin/substrate-daemon` reports `substrate-daemon 0.2.0`, file dated Aug 16 (observed: `--version`). That build predates sessions, PTY, metrics and quotas, so a spike needs a current one.

| capability | what it does | state | host requirement | source |
|---|---|---|---|---|
| exec confinement | argv-only exec, cleared environment, writable `/workspace` only, read-only system, no network, seccomp denies new AF_UNIX sockets and `io_uring`, whole-cgroup kill | shipped | Linux; daemon in a delegated cgroup v2 subtree with `cpu memory pids`, delegation root kept process-free; bubblewrap that accepts `--disable-userns`; `/usr/bin/socat`; `--cgroup-root` | `substrate/website/docs/concepts/confinement.md:22-35`; `substrate/README.md:362-380`; `substrate/adr/0015-declared-host-roots-carry-no-host-ipc.md:17-21`; `substrate/adr/0004-the-host-driver-refuses-without-linux-confinement.md:15-19` |
| no nested user namespace | the child cannot create its own user namespace | shipped | as above | `substrate/AGENTS.md:98` |
| cgroup limits | memory+swap (`memory.oom.group=1`), pids up to 4096, timeout and CPU budget each up to 24 h; `cpu.max` quota never above one period, so **one core per exec** | shipped | as above | `substrate/crates/substrate-host/src/process.rs:1854-1863`, `:3055-3074` |
| filesystem guard | `openat2` beneath / no-link / no-mount for file APIs; symlink escape refused | shipped | Linux | `substrate/README.md:442`; `confinement.md:11-20` |
| read-only host roots | caller-declared host directories mounted `--ro-bind`, e.g. a toolchain; `/workspace` stays the only writable path | shipped | none extra | `substrate/adr/0010-declared-host-roots-are-mounted-read-only.md:51-54` |
| scoped workspace write | exec may get read-only or named writable subdirectories of its workspace | shipped (bundle 0.12.0) | none extra | `substrate/adr/0023-workspace-write-access-is-explicit.md:15-27` |
| existing directory as workspace | adopted directories can be read and written, but `exec.start` still requires a `ws_` id | half done | none | `substrate/STATUS.md:144-150`; `substrate/crates/substrate-daemon/src/app/operations.rs:416` |
| Git workspaces | shallow (depth 1-50) checkout of an exact commit from an operator-configured HTTPS source, with transient Connector authority; detached HEAD; `.git` hidden from file APIs | shipped, conditional | `--git-source`, a Connector broker | `substrate/README.md:246-258`; `substrate/website/docs/reference/contract.md:34-37` |
| egress apertures | no egress by default; operator declares `name=host:port/tcp[/max=size]`; a run selects one **by name**; DNS resolved once and pinned | shipped | bubblewrap; declared at daemon start | `substrate/README.md:288-332`; `substrate/docs/design/10-destination-bound-egress.md:200` ("One named aperture per run in the first slice") |
| secret slots | a credential reaches the child only as a sealed memfd at a declared fd; env names containing `secret`, `token`, `credential` are refused | shipped | memfd sealing probe | `substrate/README.md:260-286`; `substrate/docs/design/11-sealed-secret-slots.md:25` |
| sessions / PTY | leased raw-pipe or PTY session, one WebSocket attachment. Attachment loss, protocol failure or lifetime expiry kills the tree; attachment lifetime is 1 h | shipped (PTY probe-gated) | `sessions.pty` fact | `substrate/adr/0008-pipe-sessions-have-distinct-durable-identity.md:34-38`; `substrate/adr/0019-pty-is-a-second-session-mode.md:22-27`; `substrate/crates/substrate-daemon/src/app/sessions.rs:72` |
| durable operations | operation reserved in SQLite before dispatch; stored answer replayed on retry; daemon restart turns unprovable in-flight work into `unknown`, never redispatches | shipped | none | `substrate/docs/design/03-lifecycle-observation-and-reconciliation.md:82-92`; `substrate/website/docs/concepts/operations.md:47-76` |
| survive a daemon restart? | **no**: at startup the driver kills every `substrate-ex_` cgroup and proves it empty before admitting an exec | shipped behaviour | none | `substrate/crates/substrate-host/src/process.rs:320-345`; `substrate/docs/design/11-sealed-secret-slots.md:101-104` |
| survive a client restart? | a plain exec: yes, it runs without a client, bounded by its lease and timeout. A session: no, attachment loss kills it | shipped | none | `substrate/website/docs/use-cases.md:19`; `adr/0008-…:34-38` |
| leases and cleanup | optional renewable leases on workspace, exec, session; expiry kills and records; clocks survive reboots by boot identity | shipped | none | `operations.md:91-96`; `design/03-…:99-105` |
| events | typed events with generation and sequence (`workspace.created`, `exec.exited`, `operation.terminal`); bounded retention; reconciliation snapshots after a gap. Sessions have no `session.*` events | shipped | none | `operations.md:78-89`; `status.md:62`; `contract.md:112-124` |
| metrics | `GET /v1/metrics`: `wall_time_us`, `cpu_time_us`, `memory_current_bytes`, `memory_peak_bytes`, `processes_current`, `processes_peak`, `process_limit_hits`, `memory_oom_kills`, `io_read_bytes`, `io_write_bytes`, scratch use; `/v1/metrics/stream` 1 Hz, latest wins, no history. Opt-in per exec | shipped | `exec.resource-usage` fact | `substrate/website/docs/guides/storage-and-metrics.md:95-150`; `substrate/adr/0021-execution-metrics-are-explicit-exact-observations.md:16-22` |
| storage quotas | hard byte+inode limits on `/workspace` and per-exec `/scratch`; `/tmp` is memory-backed per exec | shipped, thinly proven: the seven real quota cases are `#[ignore]`d and ran once by hand | filesystem with `prjquota`/`pquota`, an exclusive project-ID range, a daemon binary with `cap_sys_admin` | `storage-and-metrics.md:8-54`; `substrate/README.md:147-161`, `:198-205`; `substrate/STATUS.md:49` |
| local transport | owner-only Unix socket; subject from kernel peer credentials | shipped | none | `substrate/website/docs/guides/deployment.md:11-20` |
| remote transport | TLS 1.3 HTTPS/WSS; every request resolved against a hosted Identity service, 5-minute credentials; static-bearer TCP is loopback-only | in source and release; roadmap track "active", clean-room conformance open | an Identity service, certificates | `deployment.md:32-113`; `substrate/ROADMAP.md:27`; `substrate/README.md:382-393` |
| cross-machine scheduling, Docker, Kubernetes, Firecracker | none | absent / proposed | KVM for Firecracker | `status.md:37-43`; `ROADMAP.md:28-30` |
| Rust SDK | typed client; path or exact Git revision, not crates.io; refuses a daemon advertising a different contract; `ManagedDaemon` can supervise a daemon child | shipped (source) | Rust, tokio | `substrate/website/docs/guides/rust-sdk.md:8-30`, `:212-240`; `substrate/adr/0030-rust-crates-are-source-distributed-and-non-publishable.md:26-29` |
| MCP adapter | stdio; starts its own disposable daemon; no PTY, apertures, host roots or secret slots | shipped as a **disposable test surface**, not production | as exec | `substrate/adr/0025-the-mcp-adapter-is-a-disposable-test-surface.md:19-31`; `substrate/website/docs/guides/mcp-adapter.md:12-14`, `:75-77` |
| honest refusal | a missing guarantee is a named refusal (`exec.sandbox-unavailable`, `…-unserved`), never a weaker run; requested and applied confinement recorded separately | invariant | none | `substrate/AGENTS.md:38-39`; `confinement.md:37-59` |

Host prerequisites (observed):
- bubblewrap 0.12.0 accepts `--disable-userns` (`bwrap --help`).
- `/usr/bin/socat` is present.
- `user@1000.service` has `cpu memory pids` (`cgroup.controllers`).
- `/` is `ext4 rw,noatime` without `prjquota` (`findmnt`).
- 20 CPUs (`nproc`).
- `/` has 28G free of 848G (`df -h /`).

Substrate's delegated lane gets its delegated root without privilege from `systemd-run --user -p Delegate=yes --scope` (`substrate/README.md:130-134`). So exec can run on this host; project quotas cannot.

## 2. Conductor's needs, mapped

**A precondition for both paths (observed).** Every `claude` process on this host runs in `/user.slice/user-1000.slice/session-3.scope` (`/proc/<pid>/cgroup`). Background sessions are `claude bg-pty-host` processes whose parent is `claude daemon run` (parent pid 1), not the command that started them (`/proc/<pid>/stat`).

So wrapping `claude --bg …` in `systemd-run --scope` or in a substrate exec confines nothing: the command returns, and the session lives on under Claude Code's daemon.

Inference: to put a controller in its own cgroup, one of these is needed. Neither is verified.
- (a) Run it without `--bg`, in the foreground of a systemd unit with a PTY.
- (b) Start Claude Code's daemon itself inside a delegated user unit, and move each `bg-pty-host` into a child cgroup.

Conductor starts controllers with `claude --bg` (`conductor/.agents/conductor.md:172`). The watch and the collectors find sessions through `claude agents --json` (`conductor/docs/design/conductor.md:106`, `:286`).

| need | conductor today | substrate | plain Linux | the difference that matters |
|---|---|---|---|---|
| resource limits per instance and session | none; all sessions share one scope (above). Limits are story `instance-cgroup-limits`, which already names the systemd fallback (`conductor/.engineering/planning/story/instance-cgroup-limits.md:28-31`) | per-exec limits only. No aggregate per instance: product quotas stay outside substrate (`substrate/website/docs/concepts/boundary.md:30-37`, `ROADMAP.md:38-39`). CPU is a hard per-exec cap of 1 core (`process.rs:3058-3074`), and the whole budget must be declared up front | `systemd-run --user --slice=<instance>.slice -p MemoryMax= -p CPUWeight= -p TasksMax=`; a per-session unit inside the slice | systemd gives proportional CPU sharing across 20 cores plus an instance ceiling; substrate gives neither. Both enforce, and neither enforces anything for `--bg` sessions |
| write confinement (Bash gap) | the guard parses Bash heuristically: "A controller can write, overwrite or delete any file its user can" (`conductor/README.md:176-181`; `conductor.md:265-277`) | enforced: only `/workspace` and `/scratch` are writable (`adr/0010-…:51-54`). But the process must be the exec itself, the exec's workspace must be a `ws_` workspace and not the checkout (`operations.rs:416`), and HOME, `~/.claude` and worktree state would all have to live inside it | bubblewrap around the session: `--ro-bind / /`, then `--bind` the checkout, its trees, the build cache, `~/.claude` and scratch, with the network shared. Or Landlock. Or Claude Code's sandbox settings (story `bash-sandbox`, `conductor/.engineering/planning/story/bash-sandbox.md:21-29`, unverified) | both enforce in the kernel. Plain Linux keeps the checkout, network and credentials where they are; substrate needs a different working model. Inference: Claude Code's own bubblewrap sandbox would not start inside substrate, which forbids nested user namespaces |
| disk and build quota | policed, not enforced: thresholds `disk_low` 100G, `build_slot` 60G, `gate_stop` 15G (`conductor/docs/config.md:80-87`), `[RESOURCE]` grants (`conductor.md:209-210`), the watch's `statvfs` (`conductor.md:290`) | hard quotas on substrate workspaces only: "Substrate never substitutes a directory-size scan for a hard quota" (`storage-and-metrics.md:53-54`) | `setquota -P` with one project id per instance on trees and `target/`; story `instance-storage-quota` (`…/story/instance-storage-quota.md:24-28`) | both need `prjquota`, which this host lacks; decision-blocker `project-quota-filesystem` is open (`…/decision-blocker/project-quota-filesystem.md:13`). Plain Linux applies to the existing trees; substrate only to trees it created |
| attribution of load and cost | none; the operator's load complaint was investigated by hand (`…/story/instance-usage-metrics.md:54-56`) | exact counters per exec (`storage-and-metrics.md:102-128`) | the same counters in each session cgroup's `cpu.stat`, `memory.current`, `memory.peak`, `pids.current`, `pids.peak`, `io.stat` and `memory.events`. Inference: substrate reads these same kernel values ("actual values come from the kernel", `storage-and-metrics.md:130`) | equal data; substrate adds explicit absence where a counter is missing. Model-token cost is in neither: it lives in transcripts, which the watch already reads (`conductor.md:287-288`) |
| reliable lifecycle | the watch detects exits, usage limits and context size from `claude agents --json` and transcripts (`conductor.md:279-294`); 429 and context hand-over are handled by restarting with `claude --bg --resume` (`.agents/conductor.md:120-149`) | durable exits with a cause (`exec.cpu-limit`, `exec.memory-limit`, `run-a-command.md:208-211`), `exec.exited` events, leases, orphan kill. A daemon restart kills all running execs (`process.rs:320-345`); a session is killed when its 1 h attachment ends (`sessions.rs:72`, `adr/0008-…:34-38`); an exec has no restart policy (`substrate/docs/design/01-contract.md:132-133`) | a systemd unit per session: `Restart=on-failure`, `systemctl --user show -p ExecMainStatus,Result`, `KillMode=control-group` to kill orphans, journald to keep output | substrate's records are more exact (named refusal, OOM, durable answer), but its lifetimes (1 h session, 24 h exec, killed on daemon restart) are shorter than a controller's. 429 and context hand-over are Claude Code events, and neither tool helps with them |
| multi-instance isolation | names, tasks, dashboard port and grants are planned per instance (`conductor/.engineering/planning/specification/multi-instance-host.md:29-42`) | "One daemon is one trust domain; it is not a multi-tenant isolation layer" (`status.md:56`): one daemon, delegated root and exclusive quota range per instance (`storage-and-metrics.md:35-37`) | one slice per instance, plus the same naming work | substrate adds one daemon per instance and isolates nothing that systemd slices do not |
| controllers on a second machine | not designed | HTTPS/WSS with Identity admission; no plaintext fallback and no static token off loopback (`deployment.md:85-113`). Conductor has no Identity service. No placement (`status.md:42`) | `ssh host systemd-run --user …`, with Claude Code logged in on that host and the checkout living there | substrate needs an Identity deployment plus everything in the write-confinement row. Inference: SendMessage does not cross machines, so a remote controller also needs the mailbox transport, which is modelled but has no command (`conductor.md:65-67`) |
| observability for dashboard and watch | sessions, transcripts, CI, `statvfs` (`conductor.md:284-290`) | `/v1/metrics`, `/v1/metrics/stream` (at most 4 streams per subject, 1 h each, `storage-and-metrics.md:139-146`), `/v1/events` with retention | read the same cgroup files on the watch's cadence | the dashboard story already plans both sources (`…/story/instance-usage-metrics.md:60-63`). The cgroup source is the one that works for every session |

## 3. Costs and risks

**Claude Code against substrate's exec and session model.** This is from the docs; nothing was run.

| Claude Code needs | substrate rule | result |
|---|---|---|
| network to the model API, and to the forge for `git push` and `gh` | one aperture per run (`design/10-…:200`); host resolved once and pinned (`README.md:311-316`) | the API and the forge cannot both be reached from one run. Inference: a pinned address for a CDN-fronted API host breaks when that address changes |
| OAuth credentials in `~/.claude` | env names containing `token` or `credential` are refused (`design/11-…:25`); secrets arrive as a memfd on an fd (`README.md:262-267`); `/workspace` is the only writable path (`adr/0010-…:51-54`) | inference: Claude Code reads a credentials file, not an fd, so the file would have to be written into `/workspace`, which defeats the point of the secret slot |
| a long-lived interactive session | PTY attachment lifetime is 1 h and losing the attachment kills the tree (`sessions.rs:72`; `adr/0008-…:34-38`); an exec has no stdin and runs at most 24 h (`process.rs:1854-1863`) | as a session, a controller dies at 1 h. As an exec, it can only be `claude -p` with the brief in argv |
| reaching other sessions (SendMessage, `claude agents --json`) | creating AF_UNIX sockets is denied (`adr/0015-…:17-21`); HOME is inside the sandbox | inference: the session becomes invisible to the watch and unreachable by SendMessage, which is conductor's channel (`conductor.md:65-67`) |
| its own child processes (Bash tool, sub-agents, MCP servers) | pids bound, 1 core (`process.rs:3058-3074`), whole-tree kill | fits, but slowly: workers and cargo share one core |
| `ssh-agent`, keyring for `gh` | AF_UNIX denied | inference: SSH-agent pushes and D-Bus keyring reads fail |

Builds on their own fit better. A `cargo test` can get a read-only toolchain root (the use ADR 0010 was written for, `adr/0010-…:70-71`), no network, `/scratch` for `target/`, and exact usage counters. It still needs:
- a copy of the tree in a `ws_` workspace (through file API pages, or a Git source with Connector authority);
- offline crate sources;
- and it gets one core.

**Running substrate.**
- One daemon per instance, each with a process-free delegated cgroup root (`README.md:362-380`).
- For quotas, also an exclusive project-ID range and a `cap_sys_admin` binary (`README.md:198-205`).
- Linux only (`adr/0004-…:15`); the container profile is amd64 only (`README.md:217-219`).
- Conductor's README also covers macOS for the dashboard (`conductor/README.md:161-163`), so anything built on substrate would work only on Linux.

**Contract maturity.**
- The bundle is development, not stable (`STATUS.md:24-27`).
- The SDK refuses a daemon whose advertised contract differs from its pin (`rust-sdk.md:12-15`), so daemon and SDK must be upgraded together.
- The confinement lane is a release condition taken from a recorded local run, not executed in CI (`STATUS.md:45`).
- The quota cases have one hand-run record (`STATUS.md:49`).

**What conductor would have to build.**
- A Rust client on `b10x-substrate-sdk`, pinned to an exact Git revision (`adr/0030-…:26-29`). Conductor already depends on tokio (`conductor/crates/conductor/Cargo.toml:25`).
- On top of it:
  - a long-lived owner for lease renewal and event cursors (the watch is the candidate);
  - operation ids persisted per dispatch;
  - reconciliation of `unknown` outcomes after restarts;
  - a mapping in the store from sessions to workspaces and execs.
- Spec-first applies: each of these goes into conductor's ESS specification first. `instances[].limits` is already planned that way (`…/story/instance-cgroup-limits.md:28`).

**Coupling rules.**
- Substrate embeds nothing of its consumers, and a consumer uses substrate only through its public crates (`substrate/AGENTS.md:35-37`).
- Clients must not branch on which driver answered (`substrate/AGENTS.md:40-42`).
- The MCP adapter is not a production surface (`adr/0025-…:19-31`).
- Substrate's "Stack adoption" track counts a consumer only once it records adoption in its own repository (`substrate/ROADMAP.md:31`).

**What a spike can settle.** The planned spike runs `claude -p` and `claude --bg` through substrate (`…/story/substrate-session-spike.md:20-26`). Given the `--bg` finding above, `claude --bg` will leave the sandbox, and its result would say nothing about confinement. The test that actually decides the question is a foreground `claude -p` with an aperture to the API host only, reading its credentials from `/workspace`.

## 4. Recommendation

1. **Adopt now, without substrate:**
   - a systemd slice per instance with `MemoryMax`, `CPUWeight` and `TasksMax`;
   - controllers started in their own units rather than through `claude --bg`, or Claude Code's daemon moved into a delegated unit;
   - usage read from cgroup files;
   - bubblewrap or Landlock around each controller for write confinement.

   `setquota -P` follows once the `prjquota` decision is taken.
2. **Adopt from substrate, later:** run large gates (`task check`, `cargo test`) as substrate execs, for exact usage, no network and hard `/scratch` quotas. Wait until execs can run in an adopted directory (`STATUS.md:144-150`) and the one-core-per-exec CPU cap is lifted or documented (`process.rs:3058-3071`). Reconsider remote controllers when an Identity service exists.
3. **Substrate does not help with:**
   - usage limits (HTTP 429) and context hand-overs;
   - `--bg` sessions living under Claude Code's daemon;
   - SendMessage routing;
   - per-instance aggregate limits and grants;
   - model-token cost;
   - the disk already full outside its workspaces.

## 5. The tools-in-substrate route (metaharness)

Read 2026-10-09. `metaharness/` is `~/beyond10x/metaharness` at `4155db5` (0.9.3; 0.9.1 is installed). `harness/` is `~/beyond10x/harness` at `798325f` (0.13.3), the revision metaharness pins. Claude Code is 2.1.295. "Observed" means run on this host today. "Inference" means not run.

**Correction first: metaharness does not run Claude Code's tools inside substrate.** The route the brief describes is assembled from two separate metaharness paths. Neither path does all of it.

| path | harness | where tools run | confined? |
|---|---|---|---|
| Claude, `--tool-surface owned` | Claude Code; built-ins off; MCP server `metaharness mcp-serve` | in the server process, through `LocalOperations::unconfined` (`metaharness/crates/metaharness-cli/src/lib.rs:309-313`) | **no.** "nothing under this confines the process … A run that wants the effects actually confined is a b10x run against substrate, not this" (`metaharness-cli/src/lib.rs:172-175`; `harness/crates/harness-tools/src/local.rs:1-29`) |
| b10x, `--substrate-embedded` | b10x's own loop, which is not Claude Code | substrate's host driver inside the loop's own process (`harness/crates/harness-substrate/src/embedded.rs:1-19`) | yes |

metaharness refuses substrate for Claude outright. `--substrate`, `--substrate-embedded`, `--cgroup-root` and `--toolchain` are refused for any kind except b10x: "substrate confines the tools **we** publish. The vendor harnesses bring their own" (`metaharness/crates/metaharness/src/builder.rs:1587-1595`). `--process-write-subtree` is also b10x-only (`builder.rs:1555-1566`). The design states it directly: "Claude Code's CLI cannot [constrain the process]" (`metaharness/docs/design/metaharness-protocol-v0.1.md:1115`).

So the route below, "Claude Code outside, its tools inside substrate", does not exist anywhere. The parts for it do exist and could be put together:
- metaharness's MCP server shell (`metaharness-tools/src/server.rs`);
- harness's catalogue (`harness-tools`);
- harness's confined provider (`harness-substrate::ConfinedOperations` over `Embedded`).

`metaharness-tools` deliberately links no substrate (`metaharness/crates/metaharness-tools/Cargo.toml:14-24`).

### 5.1 How the parts work

**Claude Code launch flags** (`metaharness/crates/metaharness-claude/src/launch.rs`):

| flag | effect | line |
|---|---|---|
| `--tools ""` | disables every built-in tool: no Bash, Write, Edit, Read, and also no SendMessage or Agent | 1107-1115 |
| `--mcp-config <file>` | one stdio server: `<this binary> mcp-serve --workspace <cwd> --writable [--allow-program P]…` | 1116-1117, 1132-1163 |
| `--allowedTools mcp__metaharness` | grants the whole server | 1118-1119 |
| `--strict-mcp-config` | account-level MCP servers excluded, always | 1057-1059 |
| `-p … --output-format stream-json --setting-sources "" --settings <file>` | print mode, hermetic: no user, project or local settings | 1051-1066 |

Further launch constraints:
- The environment is reduced to 7 inherited keys and PATH to `~/.local/bin:/usr/local/bin:/usr/bin:/bin` (`launch.rs:129-132`, `:1269-1273`).
- A `CLAUDE.md` in any ancestor directory refuses the run (`launch.rs:190-193`).

`claude --help` (observed) says `--tools` also takes a list of names, so the built-ins a session keeps can be chosen ("Bash,Edit,Read").

**What the server publishes.** Three verbs: `tool_search`, `tool_describe` and `tool_invoke`. The model sees them as `mcp__metaharness__tool_*` (`metaharness-tools/src/lib.rs:12-16`; `server.rs:9-13`). `tool_invoke` names one catalogue entry (`harness/crates/harness-tools/src/catalogue.rs:738-899`, `:931-939`):

| entry | operation | under `unconfined` (Claude path) | under `ConfinedOperations` (b10x path) |
|---|---|---|---|
| `file_read`, `dir_list`, `search`, `find` | read | std filesystem with the module's own path containment | reads come from the local provider beside the confined one (`harness-substrate/src/tools.rs:405-409`) |
| `file_write`, `file_edit` | write | std filesystem, containment by path arithmetic only (`local.rs:5-7`) | substrate guarded file API (`openat2` beneath the root, links refused) |
| `run` | shell | `Command::new(argv[0])` in the workspace (`local.rs:556-587`) | substrate `exec.start` (`harness-substrate/src/lib.rs:218-277`) |

Harness also has `Flat`, which publishes one tool per entry (`harness-tools/src/lib.rs:29-32`). metaharness serves only the three verbs (`server.rs:5`, `:119`).

**How a `run` call executes.** `run` is argv only, from a declared program set. "This is not a shell: … nothing is composed, redirected or substituted" (`catalogue.rs:896-905`).

| | unconfined (Claude path today) | confined (b10x path) |
|---|---|---|
| process | child of the MCP server, own process group (`local.rs:567-573`) | substrate exec in the in-process host driver, current-thread tokio (`embedded.rs:29-45`) |
| workspace | the real cwd | the real directory, **adopted**: its parent becomes substrate's root, and it must be one ASCII `[A-Za-z0-9_-]` path component (`embedded.rs:213-251`; `harness/crates/harness-cli/src/lib.rs:3380-3445`) |
| writable | whatever the user can write; containment applies only to the file tools | file tools: the workspace. `run` processes: **read-only** unless `--process-write-subtree` names directories (`tools.rs:105`; `harness-substrate/src/lib.rs:94-104`) |
| programs | exact `argv[0]` match, else `ProgramNotDeclared` (`local.rs:545-553`) | the same refusal (`tools.rs:422-430`). Only `/usr`, `/bin`, `/lib`, `/lib64`, the workspace and staged roots are visible; a host binary must be staged at `/toolchain/driver` (`metaharness/AGENTS.md:247-250`; `harness-substrate/src/toolchain.rs:36-42`) |
| environment | cleared, then 12 path-like names (`local.rs:108-121`, `:563`) | substrate's cleared environment plus toolchain env (`harness/crates/harness-toolchain/builtins/rust.yaml:22-27`) |
| network | the host's | `NetworkMode::None`, `aperture: None`, hard-coded (`harness-substrate/src/lib.rs:244-245`) |
| limits | 600 s wall time, 64 KiB output (`local.rs:52`, `:92`) | 900 s, 1 MiB, 2048 pids, 8 GiB, CPU 3,600 s (`lib.rs:206`, `:258-269`). The host clamps `cpu.max` to one period, so the requested 4 cores become **1** (`substrate/crates/substrate-host/src/process.rs:3056-3074`) |
| measured | exit, stdout, stderr, `timed_out`, `timeout_ms` (`local.rs:625-639`) | `ExecMeasurement::ResourceUsage` requested (`lib.rs:273`); the observation's resource record goes back in the tool result (`embedded.rs:497-505`) |
| refused | undeclared program, path escape | also: a machine that cannot confine publishes **no** `run` and no writes, and the withheld tool is recorded (`harness-substrate/src/lib.rs:13-22`; `tools.rs:94-106`) |

A confined `run` needs a delegated cgroup that the calling process sits inside. Otherwise "the loop publishes six tools instead of seven, with no error anywhere". The eval runner re-execs under `systemd-run --user --scope` for that reason (`metaharness/AGENTS.md:213-216`).

**The server is strictly sequential.** It reads one line, answers it, then reads the next (`metaharness-tools/src/server.rs:155-173`). Inference: one 15-minute `cargo test` blocks every other tool call of that session, including its sub-agents' calls.

### 5.2 What this route would gain over section 2

Assumed shape: a controller runs `claude --bg` unconfined as today. Its write-capable built-ins are switched off. A confined MCP server supplies `run`, `file_write` and `file_edit`.

| section 2 row | gain | label |
|---|---|---|
| write confinement (Bash gap) | every command a controller runs goes through `run`. It writes only the workspace subtrees it is given, and other checkouts, records and the `conductor` binary are unreachable. The guard's Bash heuristic (`conductor/docs/design/conductor.md:265-277`) is no longer needed for commands | inference from code |
| network control | per call: `run` has none. A separate tool could pick a named aperture, since the host's config carries `egress_apertures` (`substrate/crates/substrate-host/src/lib.rs:148-150`). The harness provider never sets one today | code; aperture use not built |
| attribution | exact CPU, memory and io per tool call | code |
| honest refusal | a missing confinement means a missing tool, recorded, never a weaker run | code |
| lifecycle, resource limits per session, model-token cost | no change. The session itself is still unconfined under Claude Code's daemon | — |

**Page blockers this route avoids.** Claude Code stays outside the sandbox, so these no longer apply to it:
- AF_UNIX: SendMessage and `claude agents` keep working.
- The credentials file: the OAuth login stays where it is.
- The 1-hour PTY session.
- One aperture per session: the model API is reached from outside.
- The exec-in-checkout blocker: it applies only to the daemon (`substrate/crates/substrate-daemon/src/app/operations.rs:416`; `STATUS.md:144-150`). The in-process host adopts an existing directory. Section 1's "half done" row holds for the socket, not for this route.

| remaining blocker | detail |
|---|---|
| one core per exec | the clamp above. A workspace gate on 1 of 20 cores, under a 900 s ceiling |
| git in a managed tree | a linked tree's `.git` is a file pointing outside the workspace (observed: `cw03-int/.git` holds `gitdir: …/conductor/.git/worktrees/cw03-int`). Inference: git fails inside the exec, and a commit needs the shared object store writable, which defeats the confinement |
| `git push`, `gh` | need the forge, a credential and often AF_UNIX (ssh-agent, keyring). Env names containing `token` are refused, so a credential arrives only through a secret slot |
| cargo | the rust provider is `--offline` with `CARGO_HOME={workspace}/.cargo` (`rust.yaml:25`, `:46`). Neither checks out has one (observed), so every tree needs its own crate cache or a custom read-only registry root (unverified) |
| host CLIs | `aep`, `ess`, `conductor` live in `~/.cargo/bin` and `worktree` in `~/.local/bin` (observed), so each must be staged. `worktree` and `conductor` write state outside the workspace and cannot work inside it |
| no shell | pipes, heredocs, `$(cat …)`, `&&` and redirection all stop working. Charters, skills and conductor's own start command use them (`conductor/.agents/conductor.md:172`) |
| sub-agents | inference: they share the parent session's MCP connection, so the sequential server queues them behind one another |

### 5.3 What conductor would need

1. **A confined MCP tool server.** `metaharness mcp-serve` cannot be reused for this: it is unconfined by design (`lib.rs:172-175`), and metaharness's position is that confinement is b10x's job.
   - Options: an upstream feature in metaharness, or a `conductor tools-serve` subcommand built on `b10x-harness-tools` + `b10x-harness-substrate` + `metaharness-tools::Server`.
   - Coupling for the second option: two git-pinned repositories, plus substrate 0.7.8 at `0569597` (`harness/crates/harness-substrate/Cargo.toml:29-30`), which is 3 releases behind 0.7.11. It also imports `substrate-host`, crossing substrate's consumer rule (`embedded.rs:16-19`).
   - The server needs concurrent dispatch. The current one is sequential.
   - Spec-first: the tool config (programs, write subtrees, apertures, toolchains) goes into conductor's ESS specification first.
2. **Per-controller launch flags.** `claude --bg … --tools "<kept built-ins>" --strict-mcp-config --mcp-config <file>`. The kept list must include at least SendMessage and Agent; `--tools ""` would remove conductor's channel. Unverified: whether `--tools` accepts deferred tools such as SendMessage.
   - The MCP command must be wrapped as `systemd-run --user --scope -p Delegate=yes …`, because the server inherits `session-3.scope` from Claude Code's daemon (section 2). Otherwise `run` is withheld without an error.
   - A daemon `--bg` session keeping stdio through `systemd-run --scope` is unverified.
3. **The guard.**
   - It loses the Bash rule (no Bash).
   - It keeps SendMessage.
   - It must match `mcp__<server>__.*` to see tool calls at all, since today it sees only five tools (`conductor/README.md:172-173`). That is story `guard-all-tools`.
   - It becomes the check on the host-side tools below.
4. **Toolchain staging.**
   - `--toolchain rust,taskfile`.
   - `--driver` for `aep` and `ess`.
   - Write subtrees: `target`, `.engineering`, generated directories.
   - A crate cache per tree.
5. **Host-side tools, argv-only and checked by the guard.** Git (status, diff, add, commit, log), forge (`git push`, `gh`), `worktree` and `conductor`. These stay unconfined, so the gain is that the *open shell* is gone, not that every effect is confined.
6. **What breaks daily.** Every skill, charter and brief that composes shell, which is most AEP and worktree instructions. Gates slow to one core. Controllers would have to learn the catalogue's names.

### 5.4 Verdict and first spike

**Verdict.** This is the better substrate route for conductor, better than running `claude` inside substrate: it keeps login, network to the model API, SendMessage and `claude agents`, and it confines exactly the Bash gap. It is not ready.
- No component implements it. metaharness rejects it for Claude.
- Git in managed trees, the forge, `worktree`, `conductor` and offline cargo all have to become host-side or staged tools.
- One core per exec makes gates slow.

Against section 4's plain-Linux recommendation (bubblewrap or Landlock around the whole session, Claude Code's own sandbox), it adds two things: per-call usage and per-call network. It costs a new server, two pins and the loss of the shell. Inference: plain Linux stays the first step, and this route is the second, for builds and tests only.

**Spike on this host, in increasing cost:**
1. **No model, no code.** Run `systemd-run --user --scope -p Delegate=yes b10x-harness tools --workspace <a conductor tree> --substrate-embedded --cgroup-root <delegated subtree> --toolchain rust,taskfile --allow-program cargo --allow-program git --allow-program task --process-write-subtree target`.
   - Settles: is `run` published or withheld for an adopted, non-`ws_` tree, and which toolchain roots are applied.
2. **Scratch binary on `b10x-harness-substrate` at `798325f`, no commits.** Adopt the same tree and exec:
   - `git status`, expected to fail on the linked `.git`;
   - `cargo test --offline -p conductor`, to read wall time, the 1-core clamp and the resource record;
   - a write outside `target`, expected to be refused.
3. **Wrap step 2 in `metaharness_tools::Server`.** Launch a throwaway `claude --bg --tools "SendMessage,Agent,Read,Grep,Glob" --strict-mcp-config --mcp-config <file>`.
   - Settles: does the session see the verbs, does SendMessage still deliver, does `claude agents --json` list it, does a long `run` block a sub-agent, and does Claude Code's MCP client time out a 15-minute call.

`/` has 28G free (observed). Step 2's `target/` plus a crate cache should stay under the 10G floor.
