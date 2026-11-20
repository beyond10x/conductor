---
format: aep.planning-md/3
id: story:supervisor-watch-command
kind: story
status: active
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
  path: crates/conductor/Cargo.toml
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
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/store.rs
- confidence: cited
  path: crates/conductor/src/supervisor_watch/local.rs
- confidence: cited
  path: crates/conductor/src/supervisor_watch/mod.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/supervisor_watch.rs
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/host.yaml
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/reference/config.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T13:56:15Z", actor: "human:unknown", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-20T13:56:15Z", actor: "human:unknown", revision: 10}
---
## Why

Issues #254 and #247. The host supervisor learns of a supervisor trigger only through a watch it
writes by hand in a background shell each time it re-arms. Each rebuild drops triggers: a restart
skipped a report already in the mailbox, a progress report that named a wait on a person was
filtered out, an idle trigger the hand-off listed was never run, and a controller waited 27 min on
`ControllerLimit` unseen. Six triggers were added by hand in one day (unpushed, `ControllerLimit`,
green-unmerged, idle, CI end, disk baseline). Rule 3 of `AGENTS.md`: running code is Rust.

This story is the command, its specification and its eight host-local triggers; the two triggers
that read the forge (`GREEN`, `CIEND`) are `story:supervisor-watch-forge-triggers`, which depends
on this one (critic panel, wave 69: one unit was too large and mixed file reads with forge reads).

On main (release 1.20.1):

- The watch is prose: `crates/conductor/roles/supervisor.md:113-122` asks for "one background Bash"
  over `<state>/mailbox/conductor/`, six run-journal strings and a 20 min timeout (`inotifywait`
  or a `sleep 30` loop); unpushed commits are a manual `git log --branches --not --remotes` of the
  product checkout each pass (`supervisor.md:203-207`). No config key names the product checkout.
- The profile is at its cap: `crates/conductor/tests/profiles.rs:209-214` holds `supervisor.md` to
  463 lines, and it has 463.
- No command waits on a host-wide trigger. `conductor watch run` (`crates/conductor/src/watch.rs:860`)
  runs one instance's pass (`Watch::pass`, `watch.rs:1198`). Its seam is `Sources`
  (`watch.rs:372`): the clock (`watch.rs:416`), the free-bytes reader (`watch.rs:412`), and the
  forge programs as command templates; it reads silent transcripts (`silent`, `watch.rs:1793`;
  `newest_write`, `watch.rs:3688`) and free disk (`disk`, `watch.rs:2670`).
- The sources exist: the mailbox directory (`MAILBOX`, `crates/conductor/src/message.rs:562`),
  whose file names are ULIDs (`message.rs:619`), so they sort by arrival; the runtime log
  (`RUNTIME_LOG`, `crates/conductor/src/placement.rs:826`, under the sessions directory,
  `crates/conductor/src/daemon/reconcile.rs:136`); the daemon's `Reconciliation` records with
  reasons `LostRuntime` and `OperatorCommand` (`spec/domains/instance.yaml:32`, record `:234`,
  view `:322`) and action `Upgrade` (`instance.yaml:22`).
- `conductor.instance.UnknownInstance` (`spec/domains/instance.yaml:384`) is the error for an
  instance the config does not name; this story reuses it.
- `spec/` has no `supervisor` command and no trigger record; the story `host-scripts-into-conductor`
  names a narrower `conductor session wait --until mailbox,journal` that was never built.

## Outcome

`conductor supervisor watch [--instance <name>]... [--checkout <path>]... [--timeout <duration>]
[--hold <ref>]... [--format text|json]` blocks until the first trigger fires on any instance of
the config file (or the instances named), then reads every trigger once more and exits printing
one line per fired trigger, sorted by trigger then instance:

```text
<TRIGGER> <instance> <subject> <detail>
```

`subject` holds no whitespace (a file name, a path with `:` and the branch, a ref, a session
name; `-` when there is none); `detail` is the rest of the line, with newlines replaced by spaces.
`--format json` prints one `conductor.host.SupervisorTrigger` object per line instead. The
supervisor runs it as its one background command and re-arms it in the turn it exits.

- `--timeout` takes `<n>s`, `<n>m` or `<n>h`, default `20m`.
- `--checkout <path>` adds a checkout outside every instance (the product checkout) to `UNPUSHED`;
  its instance field is `-`.
- `--hold <ref>` is accepted and modelled here; it filters `GREEN`, which the forge story adds.
- Exit 0 with at least one line (a `TIMEOUT` line counts). Exit 1 with the specification's error on
  one line of standard error: an unparsable `--timeout` (`conductor.host.InvalidWatchTimeout`), an
  `--instance` the config does not name (`conductor.instance.UnknownInstance`), a host directory
  that cannot be written (`conductor.host.WatchStateUnwritable`), another run holding the lock
  (`conductor.host.SupervisorWatchRunning`, naming its pid). A source that cannot be read (log
  missing, an instance with no store yet, a git error) is one note on standard error and does not
  end the run; the watch reads the others.
- State: cursors and baselines live in the host directory (`crate::host::dir`,
  `crates/conductor/src/host.rs:376`), `supervisor-watch/<instance>/`, one small file per cursor,
  and persist across runs. One run per host directory: an exclusive lock on
  `supervisor-watch/lock`, released by the kernel when the process dies, so a killed run never
  blocks the next.
- Delivery is at least once: a cursor advances only after the lines it covers are written and
  flushed to standard output, and is written to a temporary file and renamed into place. A run
  killed before it prints loses nothing; one killed between printing and the rename prints those
  lines again on the next run. A restart skips nothing.
- A first run with no cursor for an instance sets its mail and log cursors to the newest file and
  the log's end, and prints one note on standard error naming how many mail files it passed over
  (class C: a new host does not get a flood of old mail; the note keeps them visible).
- File sources are read on a change (inotify) or every 30 s, inside the process; the read
  interval is a field of the library's seam so tests run it in milliseconds.
- What it opens: only stores that exist, read only (rule 7); it writes nothing but
  `supervisor-watch/` under the host directory; it creates no instance store or state directory.
  Nothing in `conductor guard` changes and it never writes `conductor.lease` (rule 6).

The triggers this story implements, each with the exact source it reads:

1. `MAIL`: every file in `<state>/mailbox/conductor/` whose ULID name sorts after the instance's
   mail cursor, which advances to the newest name handled. All new files are listed, not only the
   first. A file whose first line is `[REPORT <...>] progress`, `started`, `unblocked` or
   `blocked ci` is skipped unless its text holds `waits on` or `waiting for` (case-insensitive);
   the skipped files still advance the cursor. Subject: the file name; detail: its first line.
2. `LOG`: lines of `<state>/sessions/runtime.log` past the instance's log cursor (byte offset,
   reset to 0 when the file is shorter than the cursor) that carry `]: context: `, `LaunchFailed`,
   `no open dispatch`, `lists no lettered option`, `DispatchNotFound`, `snapshot exited`,
   `usage limit`, `start .claude.` or `waits: ControllerLimit`. Detail: the line.
3. `DAEMON`: `Reconciliation` records of the instance newer than the daemon cursor (the newest
   record time handled) with reason `LostRuntime`, or action `Upgrade` with reason
   `OperatorCommand`. Detail: action, reason, result.
4. `INBOX`: the file `globals.supervisor.inbox` holds a request id that `globals.supervisor.outbox`
   does not; off unless both keys are set. Instance `-`; subject: the first such id; detail: the
   count.
5. `DISK`: free bytes on `thresholds.disk_path` under `thresholds.disk_low`, or 5 GiB below the
   baseline the previous pass recorded (not the value at the run's start). The baseline is
   rewritten every pass.
6. `UNPUSHED`: a commit on a local branch of a checkout under `checkouts.root`, of a managed tree
   of the instance, or of a `--checkout`, that no remote branch holds
   (`git rev-list --branches --not --remotes`) and whose committer time is over 30 min old.
   Subject: `<path>:<branch>`; detail: the short sha and age.
8. `IDLE`: a controller the store holds `Running` with an open dispatch whose transcript (and its
   subagents') has not been written for 15 min (the reader `newest_write`), unless its newest report
   holds `waits on` or `waiting for`; a `blocked ci` report never suppresses it. At most once per
   30 min per controller (the time kept in the cursor directory).
10. `TIMEOUT`: `--timeout` passed with no other trigger. Subject and detail `-`.

Triggers 7 (`GREEN`) and 9 (`CIEND`) are `story:supervisor-watch-forge-triggers`; the numbering is
kept so the profile and the issues read the same.

Spec first (AGENTS.md rule 1), before code:

- `spec/domains/host.yaml`: `conductor.host.WatchSupervisor` (input: `instance: List<String>`,
  `checkout: List<String>`, `timeout: Optional<String>`, `hold: List<String>`,
  `format: Optional<String>`); `conductor.host.SupervisorTriggerKind` (enum, all ten: `Mail`, `Log`,
  `Daemon`, `Inbox`, `Disk`, `Unpushed`, `Green`, `Idle`, `CiEnd`, `Timeout`, so the forge story
  adds no spec); `conductor.host.SupervisorTrigger` (`trigger`, `instance`, `subject`, `detail`,
  `at`), the result `List<SupervisorTrigger>`; errors `conductor.host.InvalidWatchTimeout`,
  `conductor.host.WatchStateUnwritable`, `conductor.host.SupervisorWatchRunning`, and
  `conductor.instance.UnknownInstance` reused;
- `spec/domains/config.yaml`: `conductor.config.SupervisorWatch` (`inbox: Optional<String>`,
  `outbox: Optional<String>`, each a path, `~/` expanded) as `globals.supervisor`, optional;
- the local command `supervisor-watch` in `crates/conductor/cli.yaml`; `task regen` (re-pins
  `source_digest`, `crates/conductor/http.yaml:2`); `docs/config.md` names the two keys;
  `task docs-generate` (`website/docs/reference/cli.md`, `website/docs/reference/config.md`).

## Acceptance (tests that fail on main)

In `crates/conductor/tests/supervisor_watch.rs`. Seams, all deterministic: the trigger cases call
the library entry over its seam (a `Sources`-like struct as `tests/watch.rs` uses: a fixed clock,
a free-bytes reader, a read interval of a few ms) and fixture state directories under a temporary
`HOME`, with `CONDUCTOR_CONFIG` and `CONDUCTOR_INSTANCE` removed and the instance fixed with
`active::isolate` (rule 5); the clock stands past each threshold instead of waiting; git repositories
are made locally by the case with `GIT_COMMITTER_DATE`, with a bare local remote; no network. The
exit-code cases start the binary from `common::conductor` with a config file under the case's
directory and `--timeout 1s`.

- `mail_lists_every_new_file_and_resumes_from_its_cursor_across_runs`
- `mail_skips_a_progress_report_unless_it_waits_on_a_person`
- `log_names_each_trigger_string_past_the_cursor_and_rereads_a_truncated_log`
- `daemon_names_a_lost_runtime_and_an_operator_upgrade`
- `inbox_fires_when_requests_outnumber_answers_and_is_off_without_both_keys`
- `disk_fires_under_disk_low_and_five_gigabytes_under_the_last_baseline`
- `unpushed_names_a_local_only_commit_older_than_thirty_minutes_including_a_named_checkout`
- `idle_names_a_silent_running_controller_unless_it_waits_on_a_person_once_per_half_hour`
- `timeout_prints_one_timeout_line_and_exits_zero` (binary, `--timeout 1s`)
- `a_bad_timeout_or_unknown_instance_exits_one_naming_the_error` (binary)
- `a_second_run_exits_one_while_the_first_holds_the_lock` (binary)
- `a_run_stopped_before_it_prints_advances_no_cursor` (library: the lines are produced but the
  commit step is not called; the next run lists them again)
- `the_watch_creates_no_store_and_writes_only_under_the_host_directory`

And `crates/conductor/tests/profiles.rs`: `supervisor.md` names `conductor supervisor watch` and no
longer holds `inotifywait`, `sleep 30` or `log --branches --not --remotes`; the cap stays 463.

## Boundaries

- The guard stays fast: nothing in `conductor guard` changes, and this command opens no store a
  guard call reads.
- No polling in a model turn: the command blocks in its own process; the supervisor never loops.
- The profile change replaces the hand-written watch in `supervisor.md:113-122` and the manual
  unpushed check in `:203-207` (keeping its deviation rule) with the command, `--checkout <product
  checkout>` and the re-arm rule; `supervisor.md` stays within 463 lines (`tests/profiles.rs:214`);
  the change may not raise the cap, and is reported with its diff (rule 9).
- #247's other parts (the cycle line `green-unmerged:`, the dashboard and `conductor status`
  listing, the delivery to a `merge: controller` repository's controller) stay with #247.
- The per-instance `conductor watch` and its lines are unchanged; this command reuses its readers.
- Thresholds 15/30 min and 5 GiB are constants of this story; making them config keys is later
  work.
- No organization, host or private id in code, tests or fixtures (rule 4): fixtures use
  `example-org`.

## Ambiguities

- `INBOX`: the product has no operator inbox. Class C, undoable: two optional keys,
  `globals.supervisor.inbox` and `globals.supervisor.outbox`, each a file of one request id per
  line; with either unset the trigger is off.
- The product checkout has no config key. Class C: `--checkout`, repeatable, rather than a new key.

## Scope

`spec/domains/host.yaml`, `spec/domains/config.yaml`, `crates/conductor-model` (regenerated),
`crates/conductor/cli.yaml`, `crates/conductor/http.yaml` (digest), `crates/conductor/src/cli.rs`,
`crates/conductor/src/supervisor_watch/mod.rs` (new), `crates/conductor/src/supervisor_watch/local.rs`
(new), `crates/conductor/src/lib.rs` (module), `crates/conductor/src/watch.rs` (readers
`newest_write` and `disk` made `pub(crate)`, no behaviour change), `crates/conductor/src/config.rs`,
`crates/conductor/roles/supervisor.md`, `crates/conductor/tests/supervisor_watch.rs`,
`crates/conductor/tests/profiles.rs`, `docs/config.md`, `website/docs/reference/cli.md`,
`website/docs/reference/config.md`.
