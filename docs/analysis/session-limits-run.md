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

## Second run, Thu Oct 15 01:26:47 AM UTC 2026

The checks that did not pass in the first run, re-run on commit `82508c0c`, which fixes faults 1
to 4. Same host and the same scratch instance. The only change to the config is that the
controller's `settings` and `profile` now name the second run's tree, `conductor-w37-liverun2`.
The first run's state directory was moved to `~/.cache/conductor-liverun/state-run1`, so the
store starts with no controller. The binary was built in the tree with
`conductor build run --reserve 4G -- cargo build -p conductor-cli`. Every call passed
`CONDUCTOR_CONFIG=~/.cache/conductor-liverun/conductor.yaml CONDUCTOR_INSTANCE=lrun`.
The run ended at `01:27:39 AM UTC`.

| # | check | result |
|---|---|---|
| 1 | spawn prints the session id; unit `conductor_lrun-lrunrepo.service` in `conductor_lrun.slice`, a sibling slice and not under `conductor.slice`; limits as before | **pass** |
| 2 | `controller pause-controller` stops the unit, records `Paused`, and the session leaves `claude agents` | **pass** |
| 3 | `controller resume-controller` starts a new unit running `claude --resume <id>` and records `Running`. The same session id is listed again, and the session answers a SendMessage | **pass** |
| 4 | `controller stop-controller` stops the unit and records `Stopped` | **pass** |
| 5 | cleanup: no `conductor_lrun*`/`conductor-lrun*` unit, no `conductor.slice` or `conductor_lrun.slice` | **pass** |

### 1. Spawn, unit, slice

```
$ conductor spawn --repository lrunrepo; echo "spawn exit $?"
ba51cb9a-cd71-4073-aa88-b3b393db7017
spawn exit 0
$ systemctl --user list-units 'conductor_lrun*' 'conductor-lrun*' --all --no-pager
  conductor_lrun-lrunrepo.service loaded active running [systemd-run] /bin/sh -c "…" conductor-launch ~/.cache/conductor-liverun/state/sessions/lrun-lrunrepo.fifo … "stty cols 160 rows 48; exec 'claude' '--session-id' 'ba51cb9a-cd71-4073-aa88-b3b393db7017' '-n' 'lrun-lrunrepo' … '--model' 'sonnet' …"
  conductor_lrun.slice            loaded active active  Slice /conductor_lrun
$ systemctl --user show conductor_lrun-lrunrepo.service -p Slice -p ControlGroup -p MemoryMax -p CPUWeight -p TasksMax
Slice=conductor_lrun.slice
ControlGroup=/user.slice/user-1000.slice/user@1000.service/conductor_lrun.slice/conductor_lrun-lrunrepo.service
CPUWeight=50
MemoryMax=3221225472
TasksMax=1024
$ systemctl --user show conductor_lrun.slice -p Slice -p ControlGroup -p MemoryMax -p CPUWeight -p TasksMax
Slice=-.slice
ControlGroup=/user.slice/user-1000.slice/user@1000.service/conductor_lrun.slice
CPUWeight=[not set]
MemoryMax=6442450944
TasksMax=infinity
$ systemctl --user list-units conductor.slice --all --no-pager --no-legend
(nothing)
$ cat /proc/761219/cgroup      # the claude process
0::/user.slice/user-1000.slice/user@1000.service/conductor_lrun.slice/conductor_lrun-lrunrepo.service
```

The cgroup files read `memory.max` 3221225472, `cpu.weight` 50 and `pids.max` 1024 for the
unit, and `memory.max` 6442450944 for the slice. The slice's parent is `-.slice`, the user
manager's root. No `conductor.slice` was created (fault 1 fixed). `conductor controller
controllers` shows `01a13d2b-41bd-75a7-8fdc-3a239f354e44  Running  lrunrepo  Claude
lrun-lrunrepo  1`. The dispatch log's `sent` line has the same `session_id`, and `claude
agents --json` lists `{"name":"lrun-lrunrepo","pid":761219,"status":"idle","sessionId":"ba51cb9a-…"}`.

### 2. Pause

```
$ conductor controller pause-controller --controller-id 01a13d2b-41bd-75a7-8fdc-3a239f354e44; echo "pause exit $?"
stopped lrun-lrunrepo (ba51cb9a-cd71-4073-aa88-b3b393db7017)
pause exit 0
$ conductor controller controllers
01a13d2b-41bd-75a7-8fdc-3a239f354e44  Paused  lrunrepo    Claude   lrun-lrunrepo  1
$ systemctl --user show conductor_lrun-lrunrepo.service -p LoadState -p ActiveState -p Result
LoadState=not-found
ActiveState=inactive
Result=success
$ kill -0 761219
kill 761219 failed: no such process
$ claude agents --json | jq -c '[.[] | select(.name | startswith("lrun"))]'
[]
```

The command took 3 s (01:27:02 to 01:27:05 UTC).

### 3. Resume and SendMessage

```
$ conductor controller resume-controller --controller-id 01a13d2b-41bd-75a7-8fdc-3a239f354e44; echo "resume exit $?"
resumed lrun-lrunrepo (ba51cb9a-cd71-4073-aa88-b3b393db7017) in conductor_lrun-lrunrepo.service
resume exit 0
$ conductor controller controllers
01a13d2b-41bd-75a7-8fdc-3a239f354e44  Running  lrunrepo    Claude   lrun-lrunrepo  1
$ systemctl --user list-units 'conductor_lrun*' --all --no-pager --no-legend
  conductor_lrun-lrunrepo.service loaded active running [systemd-run] … "stty cols 160 rows 48; exec 'claude' '--resume' 'ba51cb9a-cd71-4073-aa88-b3b393db7017' '-n' 'lrun-lrunrepo' … '--model' 'sonnet' …"
  conductor_lrun.slice            loaded active active  Slice /conductor_lrun
$ systemctl --user show conductor_lrun-lrunrepo.service -p ControlGroup -p MemoryMax -p CPUWeight -p TasksMax
ControlGroup=/user.slice/user-1000.slice/user@1000.service/conductor_lrun.slice/conductor_lrun-lrunrepo.service
CPUWeight=50
MemoryMax=3221225472
TasksMax=1024
$ claude agents --json | jq -c '.[] | select(.name | startswith("lrun")) | {name,pid,status,sessionId,cwd}'
{"name":"lrun-lrunrepo","pid":767613,"status":"idle","sessionId":"ba51cb9a-cd71-4073-aa88-b3b393db7017","cwd":"~/.cache/conductor-liverun/checkouts/lrunrepo"}
```

The resume finished within the same second, 01:27:10 UTC. The envelope record was written
again at `2026-10-15T01:27:10.708Z`, with the same cgroup, the same limits, and `unit`
`conductor_lrun-lrunrepo.service`.

A guard rule lets a controller message only its conductor. So the SendMessage to
`lrun-lrunrepo` (between 01:27:10 and 01:27:20 UTC) asked the session to write the word `pong` to `pong.txt` in its
repository, with no reply by message. The sender's result was `{"success":true, … → lrun-lrunrepo …}`.

```
$ date -u; ls -l ~/.cache/conductor-liverun/checkouts/lrunrepo/; cat …/pong.txt
Thu Oct 15 01:27:21 AM UTC 2026
-rw-r--r-- 1 timo timo  5 03:27:20 pong.txt          # local time, 01:27:20 UTC
-rw-r--r-- 1 timo timo 40 02:05:35 README.md
pong
```

The transcript of the resumed session,
`~/.claude/projects/-home-timo--cache-conductor-liverun-checkouts-lrunrepo/ba51cb9a-cd71-4073-aa88-b3b393db7017.jsonl`,
is the same file as before the pause. It holds the `cross-session-message` and the tool call
`"name":"Write","input":{"file_path":"~/.cache/conductor-liverun/checkouts/lrunrepo/pong.txt","content":"pong\n"}`.

### 4. Stop

```
$ conductor controller stop-controller --controller-id 01a13d2b-41bd-75a7-8fdc-3a239f354e44 --reason "live run done"; echo "stop exit $?"
stopped lrun-lrunrepo (ba51cb9a-cd71-4073-aa88-b3b393db7017)
stop exit 0
$ conductor controller controllers
01a13d2b-41bd-75a7-8fdc-3a239f354e44  Stopped  lrunrepo    Claude   lrun-lrunrepo  1
$ systemctl --user show conductor_lrun-lrunrepo.service -p LoadState -p ActiveState
LoadState=not-found
ActiveState=inactive
$ kill -0 767613
kill 767613 failed: no such process
$ claude agents --json | jq -c '[.[] | select(.name | startswith("lrun"))]'
[]
```

### 5. Cleanup

No conductor command stops an instance's slice, so the empty `conductor_lrun.slice` was stopped
with systemctl. No other unit was touched.

```
$ systemctl --user stop conductor_lrun.slice; echo "slice stop exit $?"
slice stop exit 0
$ systemctl --user list-units 'conductor_lrun*' 'conductor-lrun*' --all --no-pager
  UNIT LOAD ACTIVE SUB DESCRIPTION

0 loaded units listed.
$ systemctl --user list-units conductor.slice conductor_lrun.slice --all --no-pager
  UNIT LOAD ACTIVE SUB DESCRIPTION

0 loaded units listed.
$ claude agents --json | jq -c '[.[] | select(.name | test("lrun"))]'
[]
```

### Observations

- Faults 1, 2 and 3 of the first run are fixed live. Fault 4 (`wait_gone`) was exercised only in
  its easy case: after the pause, the old unit was already `not-found`.
- An instance's slice stays after its last session stops, and no conductor command removes it.
  This is not a fault for the run, but a scratch instance needs a `systemctl --user stop
  conductor_<prefix>.slice` by hand.
- Conductor's own role (`session start --role conductor`) is still not run under the provider.
