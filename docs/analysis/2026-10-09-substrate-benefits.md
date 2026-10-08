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
