# Spike: a session's commands run inside substrate through an MCP server (2026-10-09)

Story `substrate-tool-spike`. Plan: § 5.4 of [the substrate benefits page](2026-10-09-substrate-benefits.md).
Versions: `b10x-harness` 0.13.3 (installed), harness at `798325f`, substrate pinned by it at `0569597`
(0.7.8), metaharness at `4155db5`, Claude Code 2.1.295. Every command below was run on this host on
2026-10-09 unless a line says "inference" or "not run".

## Result

| step | done? | answer |
|---|---|---|
| 1. `b10x-harness tools --substrate-embedded` in a delegated scope | yes | `run` is **withheld**, and so is every toolchain tool. Cause: the delegated scope has no `io` controller, so substrate withholds `exec.resource-usage`, and harness requests that measurement on every exec |
| 2. scratch binary on `b10x-harness-substrate` | **no, refused** | `Embedded::exec` is refused `exec.metrics-unserved`, for the same cause. A variant on `substrate-host` directly, without the measurement, did run: git fails, offline cargo builds on one core, writes outside `target` are refused, 64 of 559 tests fail |
| 3. the same wrapped in `metaharness_tools::Server`, one `claude --bg` session | **not run** | stopped at step 2, as the brief says. The server would publish no `run` verb. No session was started |

## Setup

- **Adopted tree: a copy, not the managed tree.** `cw03-u3` passes the name rule (ASCII alphanumerics, `_`, `-`;
  `harness/crates/harness-substrate/src/embedded.rs:213-251`). But the driver makes the tree's
  **parent** its workspace root and creates `.substrate-apertures` there on every open
  (`substrate@0569597 crates/substrate-host/src/lib.rs:694-704`). Observed: it appeared in the scratch root
  on the first run. For the managed tree, that parent is the directory holding every unit's tree.
  So the spike used `cp -a` of the tree at `~/.cache/conductor-spikes/u3/root/cw03-u3`. The copy keeps
  the linked `.git` file, so `gitdir:` points into the main checkout.
- **Capsules go to scratch.** Every run set `XDG_STATE_HOME=~/.cache/conductor-spikes/u3/state`.
  Otherwise the embedded driver writes `~/.local/state/b10x-harness/capsules` (`embedded.rs:146-158`).
- **Delegated cgroup.** A wrapper re-execs itself under `systemd-run --user -p Delegate=yes --scope`.
  It moves itself into a child group `runner`, writes `+cpu +memory +pids` to the scope's
  `cgroup.subtree_control`, then passes the scope as `--cgroup-root`. This is the recipe from
  `substrate/scripts/delegated-lane.sh`.
- **Crate cache.** The rust toolchain sets `CARGO_HOME=/workspace/.cargo` (`harness/crates/harness-toolchain/builtins/rust.yaml:25`).
  So `CARGO_HOME=<copy>/.cargo cargo fetch --locked` was run on the host first: 483M.

## Step 1: what the harness publishes

```
systemd-run --user -p Delegate=yes --scope -- <wrapper> \
  b10x-harness tools --workspace ~/.cache/conductor-spikes/u3/root/cw03-u3 --substrate-embedded \
  --cgroup-root <scope> --toolchain rust,taskfile --allow-program cargo --allow-program git \
  --allow-program task --process-write-subtree target
```

Exit 0. `tools`: `file_read`, `dir_list`, `search`, `find`, `file_write`, `file_edit`. `withheld`:
`run`, `rust_check`, `rust_build`, `rust_test`, `rust_clippy`, `rust_fmt_check`, `rust_fmt`,
`rust_bench`, `rust_run`, `rust_fetch` and `taskfile_run`, each with the same reason:

```
`exec.resource-usage` must be an object because this client requests that measurement on every run,
and this machine says nothing. substrate withholds the fact until every required cgroup v2 counter,
including block I/O, is available.
```

The cause:
- The scope's `cgroup.controllers` is `cpu memory pids`, and so is `user@1000.service`'s.
- The root cgroup has `io` in `cgroup.subtree_control`, but the user manager is not delegated it.
- So `io.stat` is missing, and `probe_resource_usage` requires `io.stat` (`substrate@0569597 crates/substrate-host/src/probe.rs:247-262`).
- Every other exec fact is present: `exec.cgroup-limits`, `exec.namespaces`, `exec.no-egress`, `exec.workspace-scoped-write` and `workspace.openat2-beneath`.
- The two stderr lines `/bin/sh: line 1: blocked/probe: Read-only file system` and `root-probe: …` come from the scoped-write probe.

**Toolchain roots applied:** one read-only root, `~/.rustup -> /toolchain/rustup`. The toolchain env is:

| variable | value |
|---|---|
| `HOME` | `/workspace` |
| `CARGO_HOME` | `/workspace/.cargo` |
| `CARGO_TARGET_DIR` | `/workspace/target` |
| `RUSTUP_HOME` | `/toolchain/rustup` |
| `PATH` | `/toolchain/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:/usr/local/bin:/usr/bin:/bin` |

Programs: `cargo`, `rustc`, `rustfmt`, `task`. `taskfile` adds no root: `task` resolves from `/usr/local/bin`, which is visible under `/usr`.

**Unblocking it is a host change.** Inference, not tried: delegate `io` to the user manager with a root-owned drop-in
for `user@.service` (`Delegate=cpu cpuset io memory pids`). The user manager then has to restart, which
ends every process under it, including Claude Code's daemon. The other route is upstream: harness making
the measurement optional when the fact is absent. That contradicts its stated rule (`harness/crates/harness-substrate/src/lib.rs:273`).

## Step 2: exec in the adopted tree

Scratch project `~/.cache/conductor-spikes/u3/probe`, clap derive:
- dependencies: `b10x-harness-substrate` and `b10x-harness-toolchain` at `798325f`, `b10x-substrate-host` and `-wire` at `0569597`;
- a `harness-exec` path: `Embedded::open_with`, `workspace_adopt`, `Backend::exec`;
- a `host-exec` path: `HostDriver` directly, building the same `ExecStartInput` as `confined_exec_input` but with `measurements` empty.

**Through `b10x-harness-substrate` (the step as specified).**
- First attempt: `exec.workspace-subtree-absent`, "Writable workspace subtree target is absent". A write subtree must exist before the first exec.
- After `mkdir target`:

```
REFUSED … exec.start was not dispatched: DriverError { class: Unserved, code: "exec.metrics-unserved",
message: "The active host has not proved the requested execution resource counters.",
address: Some("measurements"), retriable: false }
```

So step 2 cannot be done on this host. The variant below departs from the plan:
- it imports `substrate-host`, the driver crate, which crosses substrate's consumer rule (`embedded.rs:16-19`);
- it has no resource record, so CPU and memory come from the delegated scope's own `cpu.stat` and `memory.peak`, and from polling the exec's cgroup.

**Through `substrate-host`, no measurement.** `--process-write-subtree target`, network none, the limits harness sets (900 s, 8 GiB, 2048 pids, 3,600 s CPU).

| argv | exit | output | wall |
|---|---|---|---|
| `git status` | 128 | `fatal: not a git repository: (null)`. The linked `.git` points outside the workspace, as § 5.2 inferred | 17 ms |
| `touch crates/probe-outside-target` | 1 | `touch: cannot touch 'crates/probe-outside-target': Read-only file system`. No file on the host | 17 ms |
| `touch target/probe-inside-target` | 0 | the file exists on the host | 15 ms |
| `cargo test --locked --offline -p conductor` | 0 | 0 tests. The package named `conductor` is `crates/conductor-model`; the story's `-p conductor` names it, and the brief's `-p conductor-cli` is the CLI | 3.2 s |
| `cargo test --locked --offline -p conductor-cli` (cold) | 101 | built offline with `CARGO_HOME` read-only, then stopped at the first failing test binary (`adv_cli`: `` `ess` runs: … NotFound ``) | 184.7 s |
| `… -p conductor-cli --no-fail-fast` (warm) | 101 | 495 passed, 64 failed, 3 ignored | 367.2 s |

The applied confinement the driver recorded for the cargo exec:
- `filesystem: workspace-rw-system-ro`, `network: none`;
- `workspace_access: scoped [target]`;
- `read_only_roots: ~/.rustup -> /toolchain/rustup`;
- cgroup `substrate-ex_u3_1`.

Resource readings (cold run):

| reading | value | source |
|---|---|---|
| `cpu.max` of the exec cgroup | `100000 100000`: **one core** of 20 | polled during the run (`substrate@0569597 process.rs:2967-2985`) |
| CPU | 194.4 s in 184.7 s wall, 1.05 cores on average | delegated scope `cpu.stat` delta |
| memory peak | 7,408,582,656 B, against the 8 GiB limit | exec cgroup `memory.peak`, last poll. Includes page cache |
| `target/` | 6.8G in the sandbox, against 2.4G for the same command on the host | `du -sh` |

The `target/` difference has a cause: `CARGO_HOME=/workspace/.cargo` hides the machine-wide `~/.cargo/config.toml`, which sets `debug = "line-tables-only"`.

Host baseline, same tree, `CARGO_BUILD_JOBS=8 cargo test --locked --offline -p conductor-cli --no-fail-fast`:

| run | result | wall |
|---|---|---|
| cold | 559 passed, 0 failed, 3 ignored | 266.1 s |
| warm | 559 passed, 0 failed, 3 ignored | 351.0 s |

The test phase is dominated by waits: one binary takes 230 s on the host and 206 s in the sandbox. Host load from other sessions makes these walls noisy. So the one-core clamp shows in compile time, not in this suite's test time.

Why 64 fail in the sandbox and none on the host:

| count | cause |
|---|---|
| 51 | `ess` not on PATH (`~/.cargo/bin` is not visible) |
| 11 | `aep` not on PATH (collectors "tried twice: … start `aep`: No such file") |
| 1 | `aep` and `claude` not on PATH (`start_snapshot_refuses_a_negative_free_disk`) |
| 1 | `store_tree.rs:241`: `FileEventStore::open_existing` on a committed fixture store fails `Read-only file system`. The test opens a source-tree fixture read-write, and the confinement refused it |

`--driver` stages exactly one host program (`harness/crates/harness-cli/src/lib.rs:2841-2846`, `driver: Option<&Path>`). This suite needs three: `ess`, `aep` and `claude`.

## Step 3: not run

Stopped at step 2. With `run` withheld, a `metaharness_tools::Server` would publish no run verb:
- `Server::new(verbs)`: `metaharness/crates/metaharness-tools/src/server.rs:27`, `:40`;
- `serve(&mut server, stdin, stdout)`: `server.rs:155-175`, one line at a time.

So none of these was observed:
- the verbs a session sees;
- SendMessage delivery beside the server;
- `claude agents --json` listing;
- a long `run` blocking a sub-agent;
- the MCP client's behaviour on a long call.

No session was started, no `conductor trust` was run, and no MCP config was written.

Inference: four of these five do not depend on substrate. They can be asked of an unconfined `metaharness mcp-serve` with `--tools "SendMessage,Agent,Read,Grep,Glob"`: the verbs seen, SendMessage, `claude agents --json`, and the MCP timeout. The fifth, sub-agent blocking, needs any slow verb, not a confined one.

## Go / no-go

- **Tool route: no-go on this host today.**
  - Harness withholds `run` without the `io` controller, and adding it is a root change that restarts the user manager.
  - Without `run`, step 3's questions stay open.
  - The parts that work (write refusal, no network, the one-core clamp) were reached only by bypassing harness through `substrate-host`.
- **Gates through the in-process host on an adopted tree: no-go as a gate, go as a mechanism.** The mechanism works: offline cargo with a read-only crate cache, writes refused outside `target`, a one-core clamp.
  - It needs a copy rather than the managed tree, because the driver writes into the tree's parent.
  - git does not work in it.
  - 64 of 559 conductor-cli tests fail because `ess`, `aep` and `claude` are absent, and `--driver` stages only one program.
  - The hidden machine cargo config makes `target/` 2.8 times larger.
  - The cold build peaks at about 7.4 GB of the 8 GiB limit, page cache included.

## Cleanup

| started | stopped | checked |
|---|---|---|
| 10 transient scopes `w03-u3-scope-<pid>` (one per run) | each ended with its process | `systemctl --user list-units --all 'w03-u3*'`: none. No `w03-u3` cgroup under `app.slice`. `ps`: no probe or harness process |
| sessions | none started | — |
| MCP servers | none started | — |

Left on disk, not deleted (build directories are not deleted by hand):
- `~/.cache/conductor-spikes/u3/root/cw03-u3/target` 7.0G;
- `~/.cache/conductor-spikes/u3/root/cw03-u3/.cargo` 483M;
- `~/.cache/conductor-spikes/u3/probe/target` 906M;
- the tree's own `target/` 2.4G, from the host baseline.
