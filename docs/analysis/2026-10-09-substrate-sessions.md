# Running a Claude Code session inside substrate (spike, 2026-10-09)

Story `substrate-session-spike`. Question: does a `claude` session run inside a substrate session
with cgroup limits, and what does substrate report about it?

**Verdict: no-go for controllers under substrate today; go for plain delegated systemd scopes.**
`claude -p` runs confined (cgroup limits enforced, one network aperture, read-only roots) and
answers. `claude --bg` is refused by Claude Code's workspace trust inside the sandbox, `GET
/v1/metrics` is unserved on this host, and three substrate bounds (one aperture per run, at most
one CPU, read-only credentials) do not fit a long-lived controller that builds and pushes.

## Setup

| item | value |
|---|---|
| daemon | `substrate-daemon` 0.7.11, `cargo install --git …/substrate --tag 0.7.11 --locked --root <scratch>/inst`; the installed `~/.cargo/bin/substrate-daemon` is 0.2.0 (see below) |
| delegated root | `systemd-run --user -p Delegate=yes --scope` around the daemon; the daemon moves itself into `<scope>/daemon` and writes `+cpu +memory +pids` to the scope's `cgroup.subtree_control`, so the root stays process-free. No root, no sudo, no unit or drop-in written |
| daemon flags | `--deployment personal --allow-uid $(id -u) --cgroup-root <scope> --egress-aperture claude=<model-api-host>:443/tcp --ca-bundle <resolved CA file>` |
| host | bubblewrap 0.12.0 (has `--disable-userns`, `--assert-userns-disabled`), `/usr/bin/socat`, systemd 261, Claude Code 2.1.295 |
| user service | `user@1000.service`: `Delegate=yes`, `DelegateControllers=cpu memory pids`; the transient scope got `cpu memory pids`, never `io` |

Every command and response is in the scratch root `~/.cache/conductor-dev-analysis/w03/scratch/spike/log/`
(`00`–`14`), with the scripts in `bin/`.

## What `GET /v1/machine` says

| fact | 0.7.11 | meaning |
|---|---|---|
| `exec.namespaces` (user, mount, pid, ipc, uts, network) | present | confinement floor proved |
| `exec.cgroup-limits` {processes, memory, cpu} | present | limits enforceable |
| `exec.cgroup-kill`, `exec.no-egress` | `true` | whole-tree kill, no network by default |
| `exec.egress-apertures` | `[{name: claude, destination: <pinned ip>:443}]` | one declared destination |
| `sessions.pty`, `sessions.unrecorded` | `true` | PTY sessions available |
| `exec.resource-usage`, `metrics.stream` | **absent** | metrics unserved |
| `workspace.storage-quota`, `exec.scratch-quota` | absent | `/` has no `prjquota` (expected) |

`exec.resource-usage` is absent because the probe requires `io.stat` at the cgroup root
(`crates/substrate-host/src/probe.rs:248-262`); the user service does not delegate `io`
(`user-1000.slice/cgroup.subtree_control` = `cpu memory pids`). The other seven files exist.

## Runs

| run | limits (mem / pids / cpu_millis / timeout) | result |
|---|---|---|
| `/usr/bin/true` with `measurements: ["resource-usage"]` | 64 MiB / 16 / 2 s / 15 s | refused: `{"class":"unserved","code":"exec.metrics-unserved","message":"The active host has not proved the requested execution resource counters."}` |
| `/usr/bin/true` | same, no measurements | `exited`, code 0, `filesystem: workspace-rw-system-ro`, cgroup `substrate-ex_…` |
| probe: `$HOME/.claude`, `claude` on PATH, `/etc` | no read-only roots | `~/.claude` absent, `credentials-not-readable`, `claude-not-on-path`, no `/etc` |
| `claude -p --model haiku "reply with OK"` | 1 GiB / 64 / 60 s / 120 s | **`exited`, code 0, stdout `OK`**, ~2.1 s wall |
| same | **64 MiB** / 64 | `signal: KILL`, refusal `{"class":"exhausted","code":"exec.memory-limit","message":"The declared memory ceiling ended the execution."}` |
| same | 1 GiB / **8** | `exit 134`, `refusal: null` after ~12 s at `pids.current=8` |
| `claude --bg -n spike-probe …` in a scratch git repo | 2 GiB / 128 / 180 s / 180 s | `bg-exit=1`: `Workspace not trusted. Run \`claude\` in /workspace/repo once and accept the trust prompt, then retry.` Step 3 stopped there |

How `claude -p` was made runnable, all inside the exec request, nothing copied:

- `read_only_roots`: the Claude Code versions directory at `/opt/claude-versions`, `~/.claude` at
  `/cred/claude`.
- `HOME=/workspace/home`; the run creates `/workspace/home/.claude/.credentials.json` as a symlink
  to `/cred/claude/.credentials.json`. The credentials are readable (`credentials-readable-via-root`)
  and read-only.
- `sandbox: {network: "aperture", aperture: "claude"}`; `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`.

Applied record of the successful run (substrate's own report):

```json
"network": {"mode": "aperture", "name": "claude", "mechanism": "loopback-forwarder",
            "bytes": {"to_destination": 59604, "from_destination": 27915}},
"read_only_roots": [{"host_path": "~/.local/share/claude/versions", "mount": "/opt/claude-versions"},
                    {"host_path": "~/.claude", "mount": "/cred/claude"}]
```

## Limits and usage observed

Substrate's `GET /v1/metrics?resource_kind=exec&resource_id=…` answered, during and after every
run, `{"class":"conflict","code":"exec.metrics-not-requested",…}` (the measurement could not be
requested, see above). Usage was read from the exec cgroup's own files once a second instead.

| `claude -p` run | value |
|---|---|
| `memory.max` / `memory.swap.max` | 1073741824 / 0 |
| `pids.max` | 64 |
| `cpu.max` | `50000 100000` (= `cpu_millis / timeout_ms`, clamped to at most one CPU: `process.rs:3067-3074` at 0.7.11) |
| peak memory (last sample, t=3 s) | 141 MiB |
| peak processes (last sample) | 26 |
| CPU used (last sample) | 0.65 s |

The cgroup is removed at exit, so terminal values were not readable; the last sample is a lower
bound.

## Other refusals met

| what | verbatim |
|---|---|
| CA bundle given as the conventional symlink | `Error: the certificate bundle must name one non-empty regular file` (pass the resolved file) |
| operation id shorter than 16 characters | `{"class":"refused","code":"request.schema-invalid","message":"Request does not match the closed operation schema.","address":"input"}`: the address names `input`, not `op` |
| workspace destroy after the runs | response `{"class":"failed","code":"operation.outcome-unknown",…,"retriable":true}`; `GET /v1/ops/<op>` then showed `terminal`, `outcome.kind: success` after 0.59 s |
| installed 0.2.0 daemon | `GET /v1/metrics` and `GET /v1/sessions`: `resource.not-found`; no apertures, no read-only roots, no `exec.resource-usage` |

## Why not controllers under substrate yet

| need of a controller | substrate 0.7.11 on this host |
|---|---|
| model API + git remote + package registries + MCP endpoints | one aperture per run, chosen by name |
| parallel builds | `cpu.max` clamped to one CPU |
| writable `~/.claude` (sessions, settings, token refresh) | credentials only through a read-only root; a refresh write would fail (not observed, inferred) |
| `claude --bg` lifetime beyond the starting command | exec ends → whole cgroup killed (`--die-with-parent`, cgroup kill); the bg session never started here, so this is inferred |
| background-session control (`claude agents`, `attach`) | seccomp refuses new Unix-domain sockets; whether `--bg` needs one is not tested (trust stopped it first) |
| trust | Claude Code refuses an untrusted workspace; the sandbox `HOME` is a fresh workspace, so trust would have to be written there |
| usage per session | `exec.metrics-unserved` until `io` is delegated to the user service (a system-level change, not made) |

## Go / no-go

**No-go** for running controllers under substrate. **Go** for both stories on the plain systemd
path this spike already exercised without privilege:

| story | path |
|---|---|
| `instance-cgroup-limits` | `systemd-run --user --scope -p Delegate=yes` (or `-p MemoryMax= -p TasksMax= -p CPUWeight=`) per instance; the scope gets `cpu memory pids` here |
| `instance-usage-metrics` | read the instance scope's cgroup v2 files (`cpu.stat`, `memory.current`, `memory.peak`, `pids.current`, `pids.peak`); io bytes from `/proc/<pid>/io`, since `io.stat` is absent without `io` delegation; the substrate `GET /v1/metrics` source stays a fixture-only path until a host serves it |

Substrate becomes a candidate again after: `io` delegated to the user service (operator decision),
more than one aperture per run or a per-run destination set, a CPU limit above one CPU, and a
writable Claude config with credentials delivered through a secret slot. Each is a substrate or
host change, not a conductor story. One-shot `claude -p` work could run under substrate now.
