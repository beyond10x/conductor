---
format: aep.planning-md/3
id: story:supervisor-watch-command
kind: story
status: draft
title: conductor supervisor watch blocks until any supervisor trigger fires on the host
summary: 'One command replaces the supervisor''s hand-written watch: ten triggers, persistent cursors, one line per fired trigger'
refs:
- provider: github
  reference: beyond10x/conductor#247
- provider: github
  reference: beyond10x/conductor#254
relations:
- decomposes: epic:loop-without-handwork
- informed_by: story:host-scripts-into-conductor
scope:
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/http.yaml
- confidence: cited
  path: crates/conductor/roles/supervisor.md
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/supervisor_watch.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: inferred
  path: crates/conductor/tests/supervisor_watch.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/host.yaml
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/reference/config.md
revision: 3
---
## Why

Issues #254 and #247. The host supervisor learns of a supervisor trigger only through a watch it
writes by hand in a background shell each time it re-arms. Each rebuild drops triggers: a restart
skipped a report already in the mailbox, a progress report that named a wait on a person was
filtered out, an idle trigger the hand-off listed was never run, and a controller waited 27 min on
`ControllerLimit` unseen. Six triggers were added by hand in one day (unpushed, `ControllerLimit`,
green-unmerged, idle, CI end, disk baseline). Rule 3 of `AGENTS.md`: running code is Rust.

On main (release 1.20.1):

- The watch is prose: `crates/conductor/roles/supervisor.md:113-122` asks for "one background Bash"
  over `<state>/mailbox/conductor/`, six run-journal strings and a 20 min timeout (`inotifywait`
  or a `sleep 30` loop); unpushed commits are a manual `git log --branches --not --remotes` each
  pass (`supervisor.md:203-205`).
- The profile is at its cap: `crates/conductor/tests/profiles.rs:209-214` holds `supervisor.md` to
  463 lines, and it has 463.
- No command waits on a host-wide trigger. `conductor watch run` (`crates/conductor/src/watch.rs:860`)
  runs one instance's pass (`Watch::pass`, `watch.rs:1198`) and prints lines into that instance's
  run; it already reads what several triggers need: silent transcripts (`silent`, `watch.rs:1793`;
  `newest_write`, `watch.rs:3688`), pull-request checks (`pull_checks`, `watch.rs:2133`), open pull
  and merge requests (`open_pulls`, `watch.rs:2939`; `open_merge_requests`, `watch.rs:3023`) and
  free disk (`disk`, `watch.rs:2670`).
- The sources exist: the mailbox directory (`MAILBOX`, `crates/conductor/src/message.rs:562`), the
  runtime log (`RUNTIME_LOG`, `crates/conductor/src/placement.rs:826`, under the sessions directory,
  `crates/conductor/src/daemon/reconcile.rs:136`), and the daemon's `Reconciliation` records with
  reasons `LostRuntime` and `OperatorCommand` (`spec/domains/instance.yaml:32`, `:234`, view `:322`).
- `spec/` has no `supervisor` command and no trigger record; the story `host-scripts-into-conductor`
  names a narrower `conductor session wait --until mailbox,journal` that was never built.

## Outcome

`conductor supervisor watch [--instance <name>]... [--timeout <duration>] [--hold <ref>]...
[--format text|json]` blocks until the first trigger fires on any instance of the config file (or
the instances named), then reads every trigger once more and exits printing one line per fired
trigger, sorted by trigger then instance:

```text
<TRIGGER> <instance> <subject> <detail>
```

`--format json` prints one `conductor.host.SupervisorTrigger` object per line instead. The
supervisor runs it as its one background command and re-arms it in the turn it exits.

- Exit 0 with at least one line (a `TIMEOUT` line counts). Exit 1 with the specification's error on
  one line of standard error: an unparsable `--timeout`, an `--instance` the config does not name,
  a state directory that cannot be written. A source that cannot be read (forge down, log missing)
  is one note on standard error and does not end the run; the watch reads the others.
- Its cursors and baselines live in the host directory (`crate::host::dir`,
  `crates/conductor/src/host.rs:376`), `supervisor-watch/<instance>/`, and persist across runs. A
  cursor advances only after the lines it covers are written to standard output, so a run killed
  before it prints loses nothing and a restart skips nothing.
- Forge reads (triggers 7 and 9) run at most every 2 min per instance; file sources are read on a
  change (inotify) or every 30 s, inside the process.

The ten triggers, each with the exact source it reads:

1. `MAIL`: every file in `<state>/mailbox/conductor/` newer than the instance's mail cursor, which
   advances to the newest file handled. All new files are listed, not only the first.
   `[REPORT …]` lines `progress`, `started`, `unblocked` and `blocked ci` are skipped unless the
   report names a person or a wait. Subject: the file name; detail: its first line.
2. `LOG`: lines of `<state>/sessions/runtime.log` past the instance's log cursor (byte offset,
   reset when the file shrinks) that carry `]: context: `, `LaunchFailed`, `no open dispatch`,
   `lists no lettered option`, `DispatchNotFound`, `snapshot exited`, `usage limit`,
   `start .claude.` or `waits: ControllerLimit`. Detail: the line.
3. `DAEMON`: `Reconciliation` records of the instance newer than the daemon cursor with reason
   `LostRuntime`, or action `Upgrade` with reason `OperatorCommand`. Detail: action, reason, result.
4. `INBOX`: the host operator inbox holds more request ids than its outbox answers, read only when
   the config names both files (see Ambiguities).
5. `DISK`: free bytes on `thresholds.disk_path` under `thresholds.disk_low`, or 5G below the
   baseline the previous pass recorded (not the value at the run's start). The baseline is
   rewritten every pass.
6. `UNPUSHED`: a commit on a local branch of a checkout under `checkouts.root`, or of a tree under
   it, that no remote branch holds (`git rev-list --branches --not --remotes`) and whose committer
   time is over 30 min old. Subject: the checkout and branch; detail: the short sha and age.
7. `GREEN` (folds in #247's trigger): an open, non-draft pull or merge request of a repository in
   the instance's sources whose head pipeline or check suite succeeded over 15 min ago, with no
   auto-merge or merge-when-pipeline-succeeds set; reported at most once per head sha (kept in the
   cursor directory); a ref given with `--hold` is left out. Subject: `<repo>!<iid>` (GitHub
   `<repo>#<n>`); detail: minutes green.
8. `IDLE`: a controller the store holds `Running` with an open dispatch whose transcript (and its
   subagents) has not been written for 15 min, unless its newest report waits on a person; a
   `blocked ci` report never suppresses it. At most once per 30 min per controller.
9. `CIEND`: a controller's newest report, in the records' `dispatches/` log, is `blocked ci <ref>`,
   and that ref's head pipeline ended after the report's time. Detail: ref, conclusion, end time.
10. `TIMEOUT`: `--timeout` (default 20 min) passed with no other trigger. Subject and detail `-`.

Spec first (AGENTS.md rule 1), in `spec/domains/host.yaml` before code:

- `conductor.host.WatchSupervisor` (input: `instance: List<String>`, `timeout: Optional<String>`,
  `hold: List<String>`, `format: Optional<String>`);
- `conductor.host.SupervisorTriggerKind` (enum: `Mail`, `Log`, `Daemon`, `Inbox`, `Disk`,
  `Unpushed`, `Green`, `Idle`, `CiEnd`, `Timeout`) and `conductor.host.SupervisorTrigger`
  (`trigger`, `instance`, `subject`, `detail`, `at`), the result `List<SupervisorTrigger>`;
- errors `conductor.host.InvalidWatchTimeout`, `conductor.host.InstanceNotConfigured`,
  `conductor.host.WatchStateUnwritable`;
- the local command `supervisor-watch` in `crates/conductor/cli.yaml`; `task regen` (re-pins
  `source_digest`, `crates/conductor/http.yaml:2`); `task docs-generate`
  (`website/docs/reference/cli.md`).

## Acceptance (tests that fail on main)

In `crates/conductor/tests/supervisor_watch.rs`, over fixture state directories under a temporary
`HOME` (rule 5), forge answers from fixtures:

- `mail_lists_every_new_file_and_resumes_from_its_cursor_across_runs`
- `log_names_each_trigger_string_past_the_cursor`
- `daemon_names_a_lost_runtime_and_an_operator_upgrade`
- `inbox_fires_when_requests_outnumber_answers`
- `disk_fires_under_disk_low_and_five_gigabytes_under_the_last_baseline`
- `unpushed_names_a_local_only_commit_older_than_thirty_minutes`
- `green_names_an_unmerged_green_request_once_per_head_sha_and_skips_a_hold`
- `idle_names_a_silent_running_controller_unless_it_waits_on_a_person`
- `ciend_fires_when_the_blocking_pipeline_ends_after_the_report`
- `timeout_prints_one_timeout_line_and_exits_zero`
- `a_bad_timeout_or_unknown_instance_exits_one_naming_the_error`
- `a_killed_run_advances_no_cursor`

And `crates/conductor/tests/profiles.rs`: `supervisor.md` names `conductor supervisor watch` and no
longer holds `inotifywait` or `sleep 30`.

## Boundaries

- The guard stays fast: nothing in `conductor guard` changes, and this command opens no store a
  guard call reads.
- No polling in a model turn: the command blocks in its own process; the supervisor never loops.
- The profile change replaces the hand-written watch in `supervisor.md:113-122` and the manual
  unpushed check in `:203-205` with the command and its re-arm rule; `supervisor.md` stays within
  463 lines (`tests/profiles.rs:214`), so the change may not raise the cap.
- #247's other parts (the cycle line `green-unmerged:`, the dashboard and `conductor status`
  listing, the delivery to a `merge: controller` repository's controller) stay with #247; this
  story takes only its trigger.
- The per-instance `conductor watch` and its lines are unchanged; this command reuses its readers.
- Thresholds 15/30 min and 5G are constants of this story; making them config keys is later work.

## Ambiguities

- `INBOX`: the product has no operator inbox. Class C, undoable: two optional keys,
  `globals.supervisor.inbox` and `globals.supervisor.outbox`, each a file of one request id per
  line; with neither set the trigger is off. The keys go into `spec/domains/config.yaml` with the
  command.

## Scope

`spec/domains/host.yaml`, `spec/domains/config.yaml` (inbox keys), `crates/conductor-model`
(regenerated), `crates/conductor/cli.yaml`, `crates/conductor/http.yaml` (digest),
`crates/conductor/src/cli.rs`, `crates/conductor/src/supervisor_watch.rs` (new),
`crates/conductor/src/watch.rs` (readers made shared), `crates/conductor/src/config.rs`,
`crates/conductor/roles/supervisor.md`, `crates/conductor/tests/supervisor_watch.rs`,
`crates/conductor/tests/profiles.rs`, `website/docs/reference/cli.md`, `website/docs/reference/config.md`.
