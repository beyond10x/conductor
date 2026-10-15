# Session limits: live run of the systemd provider

`story:session-limits-systemd`, last acceptance item: one recorded live run on this host of the
`systemd` provider (`crates/conductor/src/confine/systemd.rs`, commit `68a38d82`).

Date: `date -u` → `Thu Oct 15 12:58:49 AM UTC 2026` at the start, `01:00:42 AM UTC` at the end.

## Host

| fact | command | output |
|---|---|---|
| systemd | `systemctl --version` | `systemd 261 (261.2-1-manjaro)` |
| kernel | `uname -r` | `6.18.49-1-MANJARO` |
| harness | `claude --version` | `2.1.300 (Claude Code)` |
| delegated controllers | `cat /sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/cgroup.controllers` | `cpu memory pids` |
| the same, from the system manager | `systemctl show -p DelegateControllers user@1000.service` | `DelegateControllers=cpu memory pids` |
| provider availability | `conductor doctor` (scratch config) | `provider systemd: available`, `provider none: available` |

## Setup

Everything ran against a scratch instance; the live instance, its config, records, state and
sessions were not used. Every `conductor` call ran as
`CONDUCTOR_CONFIG=~/.cache/conductor-liverun/conductor.yaml CONDUCTOR_INSTANCE=lrun
<tree>/target/debug/conductor …`, the binary built in the tree with
`conductor build run --reserve 4G -- cargo build -p conductor-cli`.

The checkouts root holds one fresh repository, `lrunrepo` (`git init`, one commit). Its charter,
`~/.cache/conductor-liverun/records/charters/lrunrepo.md`, has five lines. It asks the session
to reply `READY` and change nothing. `conductor trust` marked the checkout and the records as
trusted.

```yaml
version: conductor.config/1
default: lrun
instances:
  - name: lrun
    session_prefix: lrun
    sources:
      - local: ~/.cache/conductor-liverun/checkouts
    checkouts:
      root: ~/.cache/conductor-liverun/checkouts
      trees: ~/.cache/conductor-liverun/trees
    records: ~/.cache/conductor-liverun/records
    state: ~/.cache/conductor-liverun/state
    cache: ~/.cache/conductor-liverun/cache
    confinement:
      provider: systemd
      mode: report
      limits:
        memory_max: 6G
    roles:
      - {role: conductor, harness: claude, model: sonnet}
      - {role: conductor-dev, harness: claude, model: sonnet}
      - role: controller
        harness: claude
        model: sonnet
        settings: <tree>/.claude/controller-settings.json
        profile: <tree>/.agents/repo-controller.md
        limits:
          memory_max: 3G
          cpu_weight: 50
          tasks_max: 1024
```

`conductor config validate` → `~/.cache/conductor-liverun/conductor.yaml: valid, 1 instance(s): lrun`.

## Checks

| # | check | result |
|---|---|---|
| 1 | controller in `conductor-lrun-lrunrepo.service` inside `conductor-lrun.slice`, limits equal the role's, slice equal the instance's | **pass** |
| 2 | listed in `claude agents --json` | **pass** |
| 3 | answers a SendMessage with `pong` | **partial**: received and acted on in about 20 s; the `pong` was denied by the guard |
| 4 | a worker sub-agent's process sits in the controller's cgroup | **pass** |
| 5 | stop and resume through the provider | **partial**: the stop passed; the resume was not run |
| 6 | the multi-line first prompt arrived in full as a bracketed paste | **pass** |
| 7 | no `conductor-lrun*` unit left | **pass** |
| — | conductor's own role, `session start --role conductor` | **not done**: held for a restart window agreed with conductor |

### 1. Unit, slice and limits

`conductor spawn --repository lrunrepo` exited 0 at 00:58:57 UTC. It printed nothing (0 bytes; fault 2).

`systemctl --user list-units 'conductor-lrun*' --all --no-pager`:

```
  conductor-lrun-lrunrepo.service loaded active running [systemd-run] /bin/sh -c "exec 3<>\"\$1\" && exec script -qfec \"\$3\" \"\$2\" <&3" conductor-launch ~/.cache/conductor-liverun/state/sessions/lrun-lrunrepo.fifo ~/.cache/conductor-liverun/state/sessions/lrun-lrunrepo.typescript "stty cols 160 rows 48; exec 'claude' '--session-id' 'e2c62570-0c29-40df-8413-99b06f678e1a' '-n' 'lrun-lrunrepo' '--append-system-prompt-file' '<tree>/.agents/repo-controller.md' '--model' 'sonnet' '--permission-mode' 'bypassPermissions' '--setting-sources' 'project,local' '--settings' '<tree>/.claude/controller-settings.json'"
  conductor-lrun.slice            loaded active active  Slice /conductor/lrun
```

The cgroup files, under
`/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/conductor.slice/conductor-lrun.slice`:

```
unit memory.max: 3221225472
unit cpu.weight: 50
unit pids.max: 1024
slice memory.max: 6442450944
slice cpu.weight: 100
slice pids.max: max
```

These are 3G, 50 and 1024 for the role, and 6G for the instance. The slice's `cpu.weight` 100
and `pids.max` `max` are the defaults, because the instance sets neither. The output of
`systemctl --user show conductor-lrun-lrunrepo.service -p Slice -p MemoryMax -p CPUWeight -p TasksMax`
agrees: `Slice=conductor-lrun.slice`, `CPUWeight=50`, `MemoryMax=3221225472`, `TasksMax=1024`.
The slice shows `MemoryMax=6442450944`, `CPUWeight=[not set]`, `TasksMax=infinity`.
`cat /proc/<claude pid>/cgroup` →
`0::/user.slice/user-1000.slice/user@1000.service/conductor.slice/conductor-lrun.slice/conductor-lrun-lrunrepo.service`.

The provider's record, `~/.cache/conductor-liverun/state/sessions/lrun-lrunrepo.envelope.json`,
reads the same values back:

- `"provider": "systemd"`, `"memory_max": 3221225472`, `"cpu_weight": 50`, `"tasks_max": 1024`;
- `"cgroup": "/user.slice/user-1000.slice/user@1000.service/conductor.slice/conductor-lrun.slice/conductor-lrun-lrunrepo.service"`;
- `"writable": null`, `"fence_missing": "the systemd provider sets up no write fence yet"`;
- `session.unit` `conductor-lrun-lrunrepo.service` and `session_id` `e2c62570-…`.

The dispatch log line carries the same session id.

The slice path also shows fault 1: `conductor-lrun.slice` sits inside `conductor.slice`.

### 2. Session list

`claude agents --json | jq -c '.[] | select(.name | startswith("lrun")) | {name,pid,kind,status,sessionId,cwd}'`:

```
{"name":"lrun-lrunrepo","pid":3606192,"kind":"interactive","status":"idle","sessionId":"e2c62570-0c29-40df-8413-99b06f678e1a","cwd":"~/.cache/conductor-liverun/checkouts/lrunrepo"}
```

### 3. SendMessage

A SendMessage to `lrun-lrunrepo` was sent, asking for a `pong` by SendMessage. The sender's
result was `{"success":true, … → lrun-lrunrepo (another Claude session on this machine …)}`.
The controller's transcript, read at 00:59:27 UTC, holds the message and these steps:

1. It received `<cross-session-message from="uds:/run/user/1000/cc-socks/<pid>.sock" from-name="conductor-dev" …>`.
2. It loaded SendMessage and sent `{"to":"uds:/run/user/1000/cc-socks/<pid>.sock","message":"pong",…}`.
3. The guard denied that send: `PreToolUse:SendMessage hook error: … conductor guard: denied:
   SendMessage to "uds:…/<pid>.sock": no live session named lrun-conductor has pid <pid> in the
   session list …`. The scratch state's `conductor guard guard-decisions` holds one row for it,
   `SendMessage … Deny`.
4. It forwarded the ping to `lrun-conductor`. The harness answered `No agent named
   'lrun-conductor' is reachable.`, because the scratch instance runs no conductor.

No `pong` came back. This is the guard's rule (`crates/conductor/src/guard/rules.rs:1050`): a
controller messages only its own conductor. It is not a fault of the provider. To get a `pong`
end to end, the run needs a sender that is the instance's own conductor session.

### 4. A worker sub-agent's processes

The controller was asked to start one background `general-purpose` sub-agent that runs
`sleep 60`. Its transcript shows `Agent {"description":"Run sleep 60","subagent_type":"general-purpose",…,"run_in_background":"true"}`.

`pgrep -x -f "sleep 60"` and `/proc/<pid>/cgroup`:

```
pid 3614293 parent 3614292 cgroup 0::/user.slice/user-1000.slice/user@1000.service/conductor.slice/conductor-lrun.slice/conductor-lrun-lrunrepo.service
```

`pstree -ps 3614293` → `systemd(1)---systemd(1025)---script(3606184)---claude(3606192)---bash(3614292)---sleep(3614293)`.
The sub-agent runs inside the controller's process, so its tools' children are in the unit's
cgroup, under the role's limits.

### 5. Stop and resume

The stop used the provider's command (`systemd.rs:430`). No conductor command reaches
`Provider::stop` or `Provider::resume` for a controller (fault 3).

```
$ date -u; systemctl --user stop conductor-lrun-lrunrepo.service; echo "stop exit $?"; date -u
Thu Oct 15 01:00:04 AM UTC 2026
stop exit 0
Thu Oct 15 01:00:06 AM UTC 2026
$ systemctl --user show conductor-lrun-lrunrepo.service -p ActiveState -p Result -p ExecMainStatus
ActiveState=inactive
Result=success
ExecMainStatus=0
$ kill -0 3606192
kill 3606192 failed: no such process
$ claude agents --json | jq -c '[.[] | select(.name | startswith("lrun"))]'
[]
```

The sub-agent's `sleep 60` was gone with the unit: `pgrep` found none.

The resume was **not run**. The provider's preparation did run, as `run_unit` does it
(`systemd.rs:223-237`):

- `systemctl --user set-property --runtime conductor-lrun.slice MemoryMax=6G CPUWeight= TasksMax=infinity` (exit 0);
- the FIFO removed and made again (exit 0).

The next step was the `systemd-run --user --unit conductor-lrun-lrunrepo.service … 'claude'
'--resume' 'e2c62570-…' …` line that `run_unit` builds. The harness of the session that did this
run refused that command, so it was not retried. With no conductor path to `Provider::resume`,
nothing else could start it. The session's reappearance in `claude agents --json` after a resume
is therefore not observed in this run. The spike observed it with the same launcher
(`docs/analysis/session-box-spike.md`, "resume").

### 6. The first prompt

The charter's five lines arrived as one user turn of the transcript
(`~/.claude/projects/-…-checkouts-lrunrepo/e2c62570-….jsonl`), whole:

```
<pasted_content id="e1e2">
Charter for lrunrepo (a scratch repository of a conductor live run).

This is a test session. Do no planning and change no file.
Line three of a multi-line charter: if you can read this line, the paste arrived in full.
Final line: reply in your terminal with the single word READY and then wait for messages.
</pasted_content id="e1e2">
```

The session's first answer was `READY`. The bracketed paste and the Enter that follows it
(`systemd.rs:422-424`) delivered every line, including the blank one, and submitted it once.

### 7. Cleanup

The checks below ran after `claude stop`/`claude rm` of the session id. Both answered
`No job matching …`: the boxed session is `interactive`, not a background job.

```
$ systemctl --user stop conductor-lrun.slice; echo "slice stop exit $?"
slice stop exit 0
$ systemctl --user stop conductor.slice; echo "exit $?"     # created by this run (fault 1), empty
exit 0
$ systemctl --user list-units 'conductor-lrun*' --all --no-pager
  UNIT LOAD ACTIVE SUB DESCRIPTION

0 loaded units listed.
$ systemctl --user list-units 'conductor.slice' --all --no-pager
0 loaded units listed.
$ claude agents --json | jq -c '[.[] | select(.name | test("lrun"))]'
[]
```

`conductor.slice` was first created by this run's spawn (`Created slice Slice /conductor.` at the
spawn's second). Before it was stopped, it held only `conductor-lrun.slice` and 0 tasks.

## Faults

1. **A prefixed instance's slice nests in the unprefixed instance's slice.**
   `slice_name` (`crates/conductor/src/confine/systemd.rs:95-100`) names a prefixed instance's
   slice `conductor-<prefix>.slice`. In systemd, `-` in a slice name means nesting, so
   `conductor-lrun.slice` is a child of `conductor.slice` ("Slice /conductor/lrun"). That parent is
   the slice of an instance without a prefix. Starting the first prefixed session creates it
   implicitly, and `set_slice` (`systemd.rs:184-211`) sets the unprefixed instance's limits on it.
   Those limits then also cap every prefixed instance, and that instance's own sessions run in a
   slice that holds the other instances. A name without a `-` hierarchy would avoid this, such as
   `conductor_<prefix>.slice`, or `conductor-<prefix>.slice` under a parent no instance owns. It
   also needs a cleanup that stops the implicit parent.
2. **`conductor spawn` prints no session id under the systemd provider.** `Provider::prompt`
   answers `Said::default()` (`systemd.rs:425`), and `spawn` prints only what the harness said
   (`crates/conductor/src/spawn.rs`, `confine::pass_through`). The spawn wrote 0 bytes. The
   module documentation says it prints the background session's id. The id is chosen at
   `systemd.rs:389` and appears only in the envelope record and the dispatch log.
3. **No conductor command stops or resumes a controller through the provider.**
   `Provider::stop` and `Provider::resume` (`systemd.rs:428-449`) have no caller for a controller:
   - `grep -rn '\.resume(' crates/conductor/src` → 0 matches;
   - `spawn::end` is `not_implemented("spawn end")` (`spawn.rs:355-356`);
   - `session stop` accepts only `conductor` and `conductor-dev` (`session.rs:446`);
   - `controller stop-controller`/`resume-controller` only write the record.

   So the acceptance item "resumes after a stop" cannot be met through conductor. After the unit
   was stopped, the scratch store still shows the controller `Running`
   (`conductor controller controllers`).
4. **The resume unit reuses the started unit's name** (`systemd.rs:442`, through `run_unit`).
   That works here only because `--collect` (`systemd.rs:247`) lets systemd drop the stopped
   unit. A unit that failed rather than stopped is also collected. But a resume issued while the
   old unit is still stopping fails with `Unit … already exists`. Not observed in this run,
   because the resume did not run; it follows from the code.

Not a fault of the provider: the `pong` of check 3 was denied by the guard's controller message
rule. The story's "answers a SendMessage from another session" needs that other session to be
the instance's conductor.
