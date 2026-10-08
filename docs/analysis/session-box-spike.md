# How a controller session lives in a box conductor controls (spike, 2026-10-09)

Story `session-box-spike`. Question: can conductor hold a `claude` session in a cgroup it owns
and a `bwrap` filesystem boundary it sets, and still have the session listed, messaged, stopped and
resumed like today's `claude --bg` sessions?

**Verdict: way (a) go, way (b) not tried as a session and no-go for now.**
- (a) A foreground `claude` under `script`, in a `systemd-run --user` unit, passed every check:
  - it runs in the unit's cgroup with the unit's limits;
  - it is listed by `claude agents --json`, receives SendMessage, outlives the shell that started it;
  - it stops cleanly and resumes under the same id.
- Under `bwrap` it works once three more paths are writable. One gap stays open:
  `~/.claude.json` cannot be written through a single-file bind.
- (b) The second daemon could not be logged in without touching the running daemon's credentials.
  The cgroup mechanism was tried with stand-in processes. It shows that (b) needs Claude Code's
  daemon restarted inside a delegated unit.

## Setup

| item | value |
|---|---|
| host | Claude Code 2.1.295, systemd 261, util-linux `script` 2.42.3, bubblewrap 0.12.0 |
| scratch root | `~/.cache/conductor-spikes/u2`, written `<scratch>` below; every command's output is in `<scratch>/log/`, scripts in `<scratch>/bin/` |
| scratch config | `<scratch>/conductor.yaml`, one instance `w03-u2`, `checkouts.root` `<scratch>/checkouts`, `records` `<scratch>/records`, `state`, `cache` and `trees` under `<scratch>` |
| model | `--model haiku` for every session |
| ids | session ids chosen up front with `--session-id $(uuidgen)`; written `<id>` here |

`conductor trust --config <scratch>/conductor.yaml` printed:

```
trusted: ~/.cache/conductor-spikes/u2/checkouts/box
trusted: ~/.cache/conductor-spikes/u2/records
```

For (b), `HOME=<scratch>/b/cfg conductor trust --config <scratch>/b/conductor-b.yaml` marked
`<scratch>/b/checkouts/box2` and `<scratch>/b/records` in `<scratch>/b/cfg/.claude.json`.
`conductor trust` writes `$HOME/.claude.json` and does not read `CLAUDE_CONFIG_DIR`, so a session
under a second config dir needs `HOME` pointed at that dir. The config's paths must then be
absolute, because `~/` expands to the changed `HOME`.

## Way (a): `claude` in the foreground of a unit conductor owns

The launcher `<scratch>/bin/box.sh <fifo> <typescript> <command>` makes the FIFO and opens it
read-write on fd 3. It then runs `exec script -qfec "stty cols 160 rows 48; exec <command>"
<typescript> <&3`. Because the FIFO is held open read-write, `script` never sees EOF on stdin, and
whatever is written to the FIFO is typed into the session's PTY.

```
systemd-run --user --unit=w03-u2-a1 -p MemoryMax=2G -p TasksMax=256 -p CPUWeight=50 \
  -p StandardOutput=null --working-directory=<scratch>/checkouts/box \
  -E TERM=xterm-256color -E SHELL=/bin/bash -E PATH=~/.local/bin:~/.cargo/bin:/usr/local/bin:/usr/bin \
  <scratch>/bin/box.sh <scratch>/a1.fifo <scratch>/log/a1.typescript \
  "claude --session-id <id> -n w03-u2-1 --model haiku --allowedTools Bash"
```

Three things the first attempt hit:

| what | observed | fix |
|---|---|---|
| the unit inherits the user manager's environment, not the login shell's | `SHELL=/bin/fish`, `PATH=/usr/local/bin:/usr/bin:…` (`systemctl --user show-environment`). `script` ran the command through fish: `fish: Unknown command: claude`, exit 127 | `-E SHELL=/bin/bash -E PATH=…` |
| `script` copies the PTY to its stdout | the whole TUI stream landed in the journal | `-p StandardOutput=null`; the typescript keeps it |
| the positional prompt | the TUI started with an empty prompt, and the transcript held only the text typed through the FIFO 30 s later | type the first prompt through the FIFO: `printf '<text>' > fifo; printf '\r' > fifo` |

Results:

| check | command | output |
|---|---|---|
| cgroup | `cat /proc/<claude pid>/cgroup` | `0::/user.slice/user-1000.slice/user@1000.service/app.slice/w03-u2-a1.service` |
| unit limits | `systemctl --user show w03-u2-a1 -p MemoryMax -p TasksMax -p CPUWeight` | `CPUWeight=50`, `MemoryMax=2147483648`, `TasksMax=256`; the cgroup files say the same (`memory.max` 2147483648, `pids.max` 256, `cpu.weight` 50; `memory.current` 246575104, `pids.current` 27 after one turn) |
| process tree | `systemd-cgls --user-unit w03-u2-a1` | `script` (main pid) → `claude --session-id <id> -n w03-u2-1 …`, both in the unit |
| listed | `claude agents --json` | `{"pid": <pid>, "cwd": "~/.cache/conductor-spikes/u2/checkouts/box", "kind": "interactive", "sessionId": "<id>", "name": "w03-u2-1", "status": "idle", …}`; no short `id` field, which only background sessions carry. The registry entry is `~/.claude/sessions/<pid>.json` |
| SendMessage | second session `w03-u2-2`: `claude -p -n w03-u2-2 --allowedTools=SendMessage,ListPeers "…send w03-u2-1 … PING-A1"` | sender: `{"success":true,"message":"“Send PING-A1 to w03-u2-1” → w03-u2-1 (another Claude session on this machine; in that session's inbox …)"}`. Receiver's transcript: `<cross-session-message from="uds:/run/user/1000/cc-socks/<pid>.sock" from-name="w03-u2-2" …> PING-A1`, and it answered the turn |
| survives the starting shell | the `systemd-run` call's shell exited at once; next calls | unit `active` and the session answering for the following 2 min |
| stop | `systemctl --user stop w03-u2-a1` | 2.0 s; `ActiveState=inactive`, `Result=success`, `ExecMainStatus=0`; `kill -0 <claude pid>`: no such process; `~/.claude/sessions/<pid>.json` removed; journal: `Session terminated, killing shell... ...killed.` (that is `script`, on SIGTERM) |
| stop from inside | `/exit` typed through the FIFO (run `w03-u2-a4`) | unit `inactive`, `Result=success`, `ExecMainStatus=0`; TUI printed `Resume this session with: claude --resume "w03-u2-3"` |
| resume | unit `w03-u2-a2`, same launcher, `claude --resume <id> -n w03-u2-1 --model haiku`; typed "Which word did you reply with first, and what text did another session send you?" | `First word: READY. Peer text: PING-A1.`, appended to the same `<id>.jsonl` (no new id) |

## Way (a) under `bwrap`

`<scratch>/bin/bwrap-claude.sh [extra writable paths] -- <claude args>` runs:

```
bwrap --ro-bind / / --dev-bind /dev /dev --bind <scratch> <scratch> \
  --bind ~/.claude ~/.claude --bind ~/.claude.json ~/.claude.json [extra --bind …] \
  --die-with-parent -- ~/.local/bin/claude <args>
```

No PID, network or IPC namespace is added. `--proc /proc` was left out, so `/proc` comes read-only
with the root. That was a choice made beforehand, not a failure that was observed. Inference: a
fresh procfs cannot be mounted without a PID namespace of its own. The wrapper ran as
the `<command>` of `box.sh` in units `w03-u2-a3` and `w03-u2-a4`.

The probe typed into each session ran one Bash command:
- `touch ./inside-cwd` and `touch ~/.cache/w03-u2-outside-probe`;
- a background subshell;
- `echo $TMPDIR` and `touch /tmp/w03-u2-probe`;
- `ls -ld /run/user/1000/cc-socks`.

| check | brief's set only (root ro; scratch, `~/.claude`, `~/.claude.json` rw), `w03-u2-a3` | plus `~/.cache/claude-tmp` ← `<scratch>/tmp-a4` and `/run/user/1000/cc-socks` rw, `w03-u2-a4` (resuming the a3 session) |
|---|---|---|
| cgroup | `…/app.slice/w03-u2-a3.service`; `script` → `bwrap` → `claude` | `…/app.slice/w03-u2-a4.service` |
| credentials, network to the model API | the model answered, so the OAuth credentials under `~/.claude` were read and the API was reached | same |
| its own child processes (Bash tool) | refused before the command ran, twice: `EROFS: read-only file system, mkdir '~/.cache/claude-tmp/claude-1000/-home-…-checkouts-box'`. That `TMPDIR` comes from the user's `settings.json` `env` | `cwd-write-ok`, `outside-write-refused` (`touch: … ~/.cache/w03-u2-outside-probe … Das Dateisystem ist nur lesbar`), `child-started`, `child-done`, `tmpdir=~/.cache/claude-tmp`, `tmp-write-refused` |
| listed in `claude agents --json` | yes: `w03-u2-3 interactive idle`, same session id | yes |
| SendMessage | sender `w03-u2-4`: `No agent named 'w03-u2-3' is reachable.`; `/run/user/1000/cc-socks/<pid>.sock` did not exist | sender `w03-u2-5`: `{"success":true,…}`; receiver's transcript holds `<cross-session-message … from-name="w03-u2-5" …> PING-A4` |
| stop, resume | `systemctl --user stop`: `Result=success`, `ExecMainStatus=0`, claude gone | the a3 session resumed under `bwrap` with its history; `/exit` gave `ExecMainStatus=0` |

**Open gap: `~/.claude.json` is not persisted.** A bind of the single file is not enough. Claude Code
writes it through a lock directory and a temporary file beside it, both in `~`. From
`claude -p --debug-file` in the same wrapper (`w03-u2-6`):

```
[ERROR] Failed to save config with lock: Error: EROFS: read-only file system, mkdir '~/.claude.json.lock'
[ERROR] Failed to write file atomically: Error: EROFS: read-only file system, open '~/.claude.json.tmp.<n>'
[ERROR] Config fallback write also failed; continuing without persisting: …
```

`~/.cache/claude-cli-nodejs/…/mcp-logs-…` writes were dropped the same way. The session works,
but its per-project state in `~/.claude.json` stays as it was: `lastSessionId` for the checkout
was still the a1 session after a3 and a4 had run and exited. Options, none tried:
- point the boxed session at its own `CLAUDE_CONFIG_DIR`, which then needs its own login;
- accept a session that does not persist this file;
- make `~` writable, which gives up most of the boundary.

**Not observed:** an OAuth token refresh under `bwrap`. No refresh fell inside the spike;
`~/.claude/.credentials.json` was writable in every run.

## Way (b): Claude Code's daemon in a delegated unit, each `bg-pty-host` in a child cgroup

**Not tried as a session.** The running daemon serves conductor and the controllers, so the only
admissible try was a second daemon under its own `CLAUDE_CONFIG_DIR`, and it could not be logged in:

| step | observed |
|---|---|
| the second config dir has its own daemon | `CLAUDE_CONFIG_DIR=<scratch>/b/cfg claude daemon status`: `not running`, `sock dir: /tmp/cc-daemon-1000/<hash>` with a hash other than the running daemon's. `TMPDIR` does not move it |
| first run in that dir | `<scratch>/b/cfg/.claude.json` seeded with `hasCompletedOnboarding`, `theme`, and the two trust marks above |
| log in with the API key present in the environment | `apiKeyHelper` in `<scratch>/b/cfg/settings.json` returning it; `claude -p` (`w03-u2-7`): `API error (attempt 7/11): 401 … "invalid x-api-key"`, killed by `timeout` after 60 s |
| log in through OAuth | not done: `claude auth login` and `claude setup-token` need a browser. A copy or link of `~/.claude/.credentials.json` would share a refresh token with the running daemon; inference: a refresh by either side rotates it for both |

So the second daemon was never started, and no `claude --bg` session was started by this spike.

**What was tried: the cgroup mechanism, with `sleep` standing in for Claude Code.**
`<scratch>/bin/b-cg.sh` ran in a delegated unit:

```
systemd-run --user --unit=w03-u2-b1 -p Delegate=yes -p MemoryMax=2G -p TasksMax=256 -p CPUWeight=50 \
  -p StandardOutput=null <scratch>/bin/b-cg.sh <scratch>/log/b1-inside.txt
```

```
unit cgroup: /user.slice/user-1000.slice/user@1000.service/app.slice/w03-u2-b1.service
unit cgroup.controllers: cpu memory pids
supervisor moved: 0::/…/w03-u2-b1.service/supervisor
subtree_control: cpu memory pids
child before move: 0::/…/w03-u2-b1.service/supervisor
child after move: 0::/…/w03-u2-b1.service/s1
s1 limits: memory.max=536870912 pids.max=64 cpu.weight=50
```

`systemctl --user show w03-u2-b1` gave `Delegate=yes` and
`DelegateControllers=cpu cpuset io memory pids`. In the unit's cgroup only `cpu memory pids` were
available.

Then, from a shell in the login scope (`session-3.scope`, where this agent runs):

| move | result |
|---|---|
| the child from `<unit>/s1` to `<unit>/s2` | `moved: 0::/…/w03-u2-b1.service/s2` |
| a `sleep` started in `session-3.scope` into `<unit>/s1` | `write error: permission denied`; it stayed in `session-3.scope` |

Read-only facts about the running daemon:
- `claude daemon run` is in `0::/user.slice/user-1000.slice/session-3.scope`.
- 14 `bg-pty-host` processes are in the same scope. 12 have the daemon as parent and 2 have parent
  pid 1, which means they outlived the daemon that spawned them.
- `user-1000.slice/cgroup.procs` and `session-3.scope/cgroup.procs` are `root root`;
  `user@1000.service/cgroup.procs` is owned by the user.

What follows for (b):
- A process can enter a delegated subtree only if it starts there, because a move from
  `session-3.scope` needs write access to a root-owned common ancestor.
- So (b) needs Claude Code's daemon itself started inside the delegated unit. The current daemon
  would have to stop and be started again in that unit, and that ends or orphans every running
  session.
- Inference, not tested:
  - A `bg-pty-host` would then start in the daemon's cgroup, and conductor would move it after the
    spawn: a race against the session's first work. The daemon also keeps pre-spawned spare hosts
    that are claimed later.
  - With `KillMode=control-group`, stopping the daemon's unit kills every session in it.
  - Listing, SendMessage and resume would be unchanged, since sessions stay `--bg` sessions.

## Go / no-go

| way | verdict | reason |
|---|---|---|
| (a) foreground `claude` under `script` in a `systemd-run --user` unit | **go** | the session was in the unit's cgroup with its limits, listed, messaged, outlived its starter, stopped and resumed, all observed. Under `bwrap` it also needs `TMPDIR` and `/run/user/<uid>/cc-socks` writable, and `~/.claude.json` persistence is open |
| (b) daemon in a delegated unit, `bg-pty-host` moved to child cgroups | **no-go for now** | not tried as a session: a second daemon cannot log in without sharing the running daemon's credentials. It also requires restarting the shared daemon, plus a move of every host after it spawns |

Stories `session-envelope-seam` and `session-limits-systemd` build on **(a)**: it is the only way
observed working end to end, and conductor owns the whole envelope with it. Conductor picks the
unit name, the limits, the `bwrap` set and the session id, and it never touches the shared daemon.

What (a) changes for conductor:
- A boxed session is `kind: "interactive"` in `claude agents --json`, with no short `id`, so
  `claude stop`, `claude rm`, `attach` and `logs` do not apply to it.
- Conductor stops it with `systemctl --user stop <unit>` and restarts it with `systemd-run …
  claude --resume <id>` in place of `claude --bg --resume`.
- Its first prompt goes through the PTY (the FIFO) or SendMessage.
- The operator watches it through the typescript.

## Units and sessions this spike started

All are stopped, and the checks are in `<scratch>/log/z-cleanup-check.txt`:
- `systemctl --user list-units --all 'w03-u2*'` lists `0 loaded units`.
- `claude agents --json` lists no `w03-u2*` name.
- No process has its cwd under `<scratch>`.

| unit | session | how it ended |
|---|---|---|
| `w03-u2-a1` (first try, fish, exit 127; `reset-failed`), `w03-u2-a1` | `w03-u2-1` (`<id>`) | `systemctl --user stop` |
| `w03-u2-a2` | `w03-u2-1` resumed | `systemctl --user stop` |
| `w03-u2-a3` | `w03-u2-3` (`<id>`), `bwrap` | `systemctl --user stop` |
| `w03-u2-a4` | `w03-u2-3` resumed, `bwrap` | `/exit` |
| `w03-u2-b1` | none (`sleep` stand-ins) | `systemctl --user stop`; cgroup gone |
| none | `w03-u2-2`, `w03-u2-4`, `w03-u2-5` (`claude -p` senders), `w03-u2-6` (`claude -p` under `bwrap`, debug log), `w03-u2-7` (`claude -p`, second config dir, 401) | exited |

No `claude --bg` session was started, so there was nothing for `claude stop` or `claude rm` to
remove.

`claude purge -y <scratch>/checkouts/box` removed three things:
- the checkout's transcripts directory;
- its entry in `~/.claude.json`, which held the trust mark;
- its 5 prompts in `~/.claude/history.jsonl`.

`<scratch>/records` was not purged. `claude purge --dry-run <scratch>/records` also listed
`projects["~/.cache"]`, an entry this spike did not create. So the records trust mark stays in
`~/.claude.json`.
