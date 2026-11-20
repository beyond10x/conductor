# Conductor run: the loop as a program

Status: takeover design, 2026-10-17. Once implemented it replaces design § 4 ("the session
schedules both"), § 8's "conductor runs it with `--follow` under the harness `Monitor` tool" and
§ 12's restart handshake, and retires the conductor session of § 1. The numbers behind it are
`docs/analysis/takeover-baseline-2026-10-17.md`.

## 1. Why

The loop's routine work was done by a model session: in ten hours the conductor session sent 268
messages, of which 236 were relays of watch lines and build words ("go" 112 times, "hold" 50) and
16 were decisions; it ran at a median context of 432k tokens against a hand-over threshold of 200k,
wrote its hand-over and kept running for three hours because the restart was a message nobody
could act on. The maintainer session spent 971M input tokens in three days, 29 % of the instance,
on 15 releases of conductor itself. The rules that were to govern both were 98 KB of prose. The
self-improvement loop approved on 2026-10-13 produced zero readings.

The organization's thesis says the opposite of this arrangement: "What is permitted, what is
required, which evidence is admissible and whether completion was earned are decided
deterministically" and "Final authority never rests on a model's judgment alone" (governed
autonomy, the organization's north star); a deterministic step "asks no model", a judgement step
"asks the model only among the actions it names" (Loom). The operator's goal is "the least amount
of tokens and the least amount of actual LLM inference calls".

## 2. What runs where

| step | today | with `conductor run` | model call |
|---|---|---|---|
| sense: snapshot, board, records check | the session runs the commands on a `CronCreate` | the daemon's cycle tick | no |
| watch: sessions, usage, context, CI, PR checks, disk | the session reads lines under `Monitor` and acts by prompt | the daemon runs the pass and the handler for each line | no |
| relay of a watch line to a controller | a session turn and a `SendMessage` | `conductor session deliver` from the handler | no |
| hand-over at the context threshold | a message the session sends, acted on by the receiver | the handler delivers `[HANDOVER]`; at 1.25× the threshold, or once the hand-over file exists, it stops the unit and respawns from charter and hand-over | no |
| resume after a usage limit | the session lists and respawns by prompt | the handler marks the sessions and resumes each at the reset its limit line names | no |
| a stalled or silent controller | the session reads transcripts and nudges | one check-in delivered; a second stall respawns | no |
| a controller's report | `SendMessage` to the conductor session, recorded by it | `conductor message send` writes the mailbox; the daemon moves the dispatch (`report-to-state`) | no |
| a `[NEED]` | routed by the session | recorded; delivered to the other repository's live controller, else queued for its next start. One whose `<other-repo>` is no repository of the instance is not relayed and not queued: it stays in the mailbox for the supervisor, and its sender is told so in one message, `[REPORT <repo> -] not relayed: <reason>`, but for a need for `conductor` itself, which is answered by no one. One whose `<other-repo>` names its sender's own repository stays in the mailbox for the supervisor as well, nothing queued for that repository and no alert raised; its sender's live controller is told so in one message | no |
| dispatch | the session writes a charter and spawns | v1: a repository that is Active, has a charter and no live controller is spawned, up to `controllers.max_working`; v2: the charter is generated | no |
| a decision request | the session classifies, decides or escalates | class by rules in code; class C answered by one bounded model call through the `Decider` adapter and recorded; class O and H go to the Decisions page; the answer is delivered by the daemon | one, bounded, class C only |
| the operator's answer on the Decisions page | the session relays it | the handler delivers it | no |
| metrics | never run | the daily tick runs `metric compute` | no |
| restart of conductor | `[RESTART conductor]`, a hand-over, `task conductor:restart` | `systemctl --user restart conductor_<instance>-run.service`; the daemon keeps no context | no |

## 3. The daemon

`conductor run` is one process per instance, installed as the user unit
`conductor_<instance>-run.service` by `conductor run install` (systemd provider) and started by
it; `conductor run --once` runs one watch pass and one cycle for tests and for a host without
systemd. It keeps no state in memory that is not in the store: every tick reads the store and the
watch's cursor files.

Ticks: every `cadence.watch` a watch pass (the existing `Watch::pass`) followed by the handlers
below and the mailbox; every `cadence.cycle` the cycle (`snapshot start-snapshot`, `publish-board`,
`records check`, dispatch v1), and at the next tick once an operator asked for one through
`conductor instance cycle` (`POST /v1/instances/{name}/cycle`, the runtime's `POST /v1/cycle`,
`story:cycle-now`), the loop's rest between ticks cut short by the ask; at `cadence.daily` `metric compute` for the previous UTC day, the
brief file `REPORT.md` in the records (deviations only), and then a commit of the records directory
through the route `forge.route` names (the bot route `b10x-gates bot` under `gates`, plain git
under `direct`), pushed when the records repository has an `origin`; a refusal is one
`records-commit-refused` line, not retried (`story:records-committed-by-daemon`; `conductor run
--once --commit-records` runs the commit alone). The daily tick is marked done for the day once
it ended, so a loop stopped during it runs it again at its next start; one started after
`cadence.daily` with none ended that day runs it at once.

| watch line | handler | records |
|---|---|---|
| `context: <repo> <n>k` at or over `context_handover` | deliver `[HANDOVER <repo>] context <n>k` once per session; over 1.25× the threshold, when `<checkout>/.git/handoff/` has a file newer than the delivery and the session list shows the session idle, or once `silent_session` passed since the delivery (then from the newest hand-over file, of any age), stop the unit (`spawn::end`), append the hand-over path as the resume point to the charter, spawn again; the `controller-restarted` line carries the `context:` line (`story:loop-restarts-controller-at-handover`). The request is kept with the id of the controller and of the session it was asked of; a later tick drops it, with one journal line and nothing stopped, once the repository runs a controller of another id, the session asked is not among the live sessions of the name while another is, or that session started after the delivery; a second session of the same name beside the one asked drops nothing, and a start up to the instant the loop itself resumed the session is not one after the delivery. A request kept without a controller id, as 1.2.0 wrote it, stands for the session its line names while the session list names that session, and is dropped the same way otherwise. The restart of a session a usage limit marked makes the same checks (`story:handover-restart-by-controller-id`) | `handover-sent`, `handover`, `controller-restarted` |
| `usage limit: …` | mark each named session `limit` with the reset its line names (`limit.rs` reads the 5-hour and the weekly form; with none, the token's limit window), kept with its attempts in `limited.json`; the pool token the session holds is marked `Limited` until that reset (through conductord for a runtime in a box), and when another token of the host's pool has headroom the session is resumed on it in the same tick, else marked to the first reset of a token of the pool, ahead of its own when another resets first (`story:limit-aware-token-reassignment`); on the first tick after that reset, resume (`spawn::resume`, the controller role's launch and model) each marked session once, or respawn one over the threshold. The loop asks no headroom probe: the session's own line and the tokens' state are the answer. The `usage-limit` line names the token and its reset, the `resumed` line the token resumed on, never a value. A second limit after such a resume backs off 5 minutes, doubling, at most an hour, never before a newly named reset; a marked session whose process is gone is recorded stopped with reason `usage limit`, giving back its `max_working` place, and resumed from its transcript at the reset (`story:auto-resume`, `story:limit-resume-at-reset`) | `usage-limit`, `resumed` |
| `PR checks done: <repo>#<n> …`, `PR merged: <repo>#<n> …` (`story:merge-wakes-idle-controller`), `main CI after merge: <repo>#<n> …` (`story:default-branch-pipeline-wakes-controller`), `CI red on main: <repo> …`, `CI green again …` | deliver the line as `[REPORT <repo> -] watch: <line>` to the live controller; with none, a repository held for a person (see Person wait below) keeps it for its next controller, red `main` included, one per kind and subject, a newer line replacing the older; it does not end the wait; else red `main` with no controller: dispatch v1 spawns one when the repository has a charter, else a deviation line | `relayed`, `red-ci` |
| `session stalled`, `session silent` | of a `Running` controller only: first: deliver `[REPORT <repo> -] check-in: report your state or blocked`; second for the same session: stop and respawn from the charter | `stalled`, `controller-restarted` |
| `session gone` with an open dispatch | respawn from the charter; a repository whose every open dispatch is `Blocked` yields instead (see Yield below), and one held for a person starts none (see Person wait below): one journal line `<repo> waits: …` and the session forgotten; a repository with a `Paused` controller is left as it is (see Pause below) | `controller-restarted` |
| `session exited (blocked)` with an open dispatch | blocked with no `pid`, as the list keeps a session a usage limit stopped: marked as `usage limit` marks it, so the first tick after its reset resumes it (`story:auto-resume`) | `usage-limit` (reason `blocked with no pid`), `resumed` |
| `session gone` of the instance's conductor-dev | when the plan of the checkout it worked in (the watch keeps it) holds an `active` story, `conductor session start --role maintainer` there, whose first prompt sends it to its newest hand-over; else nothing (`story:loop-restarts-gone-conductor-dev`) | `controller-restarted` (the session, the checkout as `repo`) |
| `disk low` | `gc` on a thread beside the ticks, one at a time (a `disk low` while one runs starts none); the first watch tick that finds it ended writes its line; `--once` waits for it (the reclaim the watch already runs under disk admit stays) | `cleanup` |
| none: each watch tick, after the mailbox, reads free disk on `disk_path` | the first reading back at `disk_clear` after one under it ends the fall: each dispatch whose last `blocked` reason starts with `disk` is reported `unblocked` by conductor (actor `Daemon`, its line's `reason` naming the free disk), and its running controller gets `[REPORT <repo> <id>] unblocked: …; build again`; with none running, the next cycle starts one as for any open dispatch (`story:disk-recovery-unblocks-dispatch`) | `unblocked`, `relayed` |
| `escalated`, `unanswered` | a desktop notification; the Decisions page already lists it | none |
| `answered <id> <choice>` | deliver `[DECISION <id>] <choice> …` to the controller that raised it | `decision-sent` |

A delivery to a controller that `conductor session deliver` refuses `SessionNotReachable` records
that controller stopped (`StopController`), with one journal line, so no later tick delivers to it
again; a kept delivery waits for the repository's next controller start
(`story:deliver-to-stopped-controller`). A refusal is no proof the session ended: when the session
list (`claude agents --json`) names the session live, or cannot be read, the controller stays
Running and the one journal line says so (`story:deliver-reaches-bg-session`). A kept delivery is
then offered to that same controller again on the next tick, since nothing was typed and no next
controller starts while it runs; the kept item counts its offers, and after the fifth refusal it
is marked refused by that controller with the last refusal, with one journal line, and waits for
the repository's next controller. A refusal that is not `SessionNotReachable`
(`DeliveryNotSeenRead`: the text was typed and not seen to be read; a delivery past its bound) is
never offered again: the session may hold the text.

Mailbox: a controller reports with `conductor message send --to conductor --from <session>
--text <file|->`, which parses the first line with the existing grammar (unparsed: exit 1 and
`FORMAT`), records the message (`receive-message`, transport `Mailbox`, `body_path`) and leaves the
body under `<state>/mailbox/conductor/<ulid>.md`. The daemon reads new files each tick:
`[REPORT]` moves the dispatch (`report-to-state`), and a `blocked`, `done` or `failed` on a dispatch
still `Sent` reports it `started` first, both lines in the log, and a `started` on a `Blocked`
dispatch (a controller restarted after a hand-over) is recorded `unblocked`; a `started` on a
`Started` dispatch or a `blocked` on a `Blocked` one (a controller resumed after a runtime restart
repeats its report) moves nothing and is not refused; a `done` within ten minutes of the
controller session's resume from its transcript, or on a `Blocked` dispatch, is held as a claim
(`done-held`, the dispatch unmoved, the controller told its dispatch) for `dispatch report-done`
to confirm, and a controller's `withdrawn` on a `done` taken since the last cycle started reopens
the dispatch (`reopened`; later only `dispatch reopen-dispatch` does, issue #202); a resumed
session's first input is its open dispatch; `[NEED]` is routed; `[DECISION-REQUEST]` goes to
decide; `[RESOURCE]` is answered "use conductor build run"; `[HANDOVER]` from a controller
(`blocked handover <path>`) is a report. A message the daemon refuses is answered to the
controller of the repository it names, as one message: `[REPORT <repo> -] refused: <reason>`, the
message and the form its kind expects (`relayed`); a message whose own tag names no repository of
the instance is answered to no one, and the journal carries the reason. Delivery to a session is `conductor session deliver --name
<session> --text <file|->`, which types the text into the session through the tmux launcher
(`type_text`) and records `delivered`. A session started with `claude --bg` has no launcher: when
the session list names it live, the text is typed the same way into a pane made for that one
delivery, which runs `claude attach <id>` on a socket of the delivery's own
(`<state>/sessions/deliveries/<process id>-<count>.tmux`, so two deliveries at once share
nothing, no reader of launcher sockets lists the pane, and the socket's name does not grow with
the session's) and is ended once the attach client has read the text; a client that does not
read it within 10 s, or that ends with the text unread, is refused `DeliveryNotSeenRead`. The
pane outlives its client, so a client that ended is looked at like one that runs: only a look at
the pane's terminal after the typing that shows nothing unread answers "read". A delivery asked to
end (SIGTERM, SIGINT) ends its pane first (`story:deliver-reaches-bg-session`). A session
neither route reaches is reported, never
retried in a loop. `SendMessage` is no longer used between roles; the guard allows `message send`
and denies `session deliver` to a controller.

Reconcile: each cycle, before dispatch v1, every Running controller whose session the session
list names neither live nor blocked, and that no usage limit marked, is recorded stopped with the
reason; one whose repository has an open dispatch is started again, as after `session gone`
(`story:controllers-reconciled-with-live-sessions`). An unread list stops none.

Yield (issue #245, `story:blocked-repository-yields-place-to-waiting-unit`): every start through
`Tick::try_start` (the restart after `session gone`, a hand-over restart, reconcile's restart, dispatch
v1's start of an Active repository, a red `main`) of a repository whose every open dispatch is
`Blocked` is checked inside the `STARTS` lock and before `make_room`. A waiting unit of work is a
`Sent` dispatch with a reference whose repository has no `Running` or `Paused` controller and no
controller stopped at a usage limit, and whose `waiting` reason dispatch v1 last recorded as
`ControllerLimit`: a unit that starts once a place is free, not one refused for a lasting reason
(`NoCheckout`, say) or left to the operator. A start yields only while places are short: the free
places under `controllers.max_working`, counted after the restart's stop, do not exceed the
repositories the waiting units are in. Then it is not made while a waiting unit was opened by this
repository's need: its reference starts `need-` and its brief's first line is a `[NEED <repo> …]`
naming it. Nor, when it is a respawn (the caller held no `Running` or `Paused` controller as the
start began: `session gone` after a stop, dispatch v1, a red `main`), while any unit waits. A
hand-over or reconcile restart of a held controller yields only to the first rule, so a routine
hand-over keeps its place (decision class C). The refusal reads `max_working <n>: the place is kept
for <dispatch> of <repo>`, the unit the first rule found, else the first waiting unit in store
order; in the watch tick it is one journal line and no error, the controller stopped and its
dispatch left `Blocked` for the unit's controller to start first.

Person wait (issue #258, `story:person-wait-and-pause-hold-restarts`): a dispatch waits on a person
when it is `Blocked` and the newest `blocked` line of its id in the dispatches log has a reason
whose first word is `person` (case-insensitive, then the end or a character that is not
alphanumeric: `person: …`, not `personal …`), as a controller reports work left only to persons
(`blocked person: waits on <who> for <what>`). A `blocked` report on a dispatch already `Blocked`
writes no line, so a `blocked handover` after it leaves the person's reason the newest. A
repository is held for a person when it has an open dispatch, every one waits on a person, and
`<state>/run/pending.json` keeps no delivery for it that no controller refused for good and that
is no watch line; the log is read only when every open dispatch is `Blocked`, once per tick. `Tick::try_start` refuses its start first,
inside the `STARTS` lock and before the yield check and `make_room`, so it holds no place: the
watch tick's restarts (`session gone`, `session stalled` the second time, a hand-over) stop the
controller as before and start none, with one journal line `<repo> waits: <dispatch> waits on a
person: <rest>`; a red `main` writes `red-ci` with that reason and keeps the line; a resume of
its session (`session exited (blocked)`, a usage limit's reset, a withdrawn token) is a wait too,
its controller recorded stopped; reconcile's restart and dispatch v1 say it as the repository's
`waits:` part of the `cycle` line and its `waiting` reason. Each refusal keeps the repository in
`<state>/run/person.json` (its open dispatches, the reason, when, and the journal line last said),
written only when it changes.
Each watch tick reads that file after the hand-overs and again after the mailbox and free disk,
each time followed by the kept deliveries: an entry whose repository runs or pauses a controller,
or has no open dispatch (done, failed, cancelled), is dropped; one no longer held is started, its
`controller-restarted` line's reason `person wait ended: a delivery kept: <event>` or `person wait
ended: the dispatch moved`, and the kept deliveries are offered to it (after the mailbox, only to
the repositories that pass started, so a delivery is offered once per tick); a start refused for any
reason keeps the entry, its journal line written only when it differs from the one said last. What
ends the wait is the next input: a `[DECISION]` answer, a `[NEED]` or its answer, a resource answer,
a mailbox reply or any other delivery kept for the repository, `dispatch report-unblocked`, a newer
`blocked` line with another reason, or a new dispatch sent. A watch line ends none (decision class
C, wave 72): each CI event would start a controller only for it to wait again, the churn of issue
#258; it is kept for the controller the next input starts.
A controller started for an input that still waits only on persons reports `blocked person: …`
again and ends its turn; its idle session is restarted by `session silent` the second time, which
stops it into the hold. `conductor spawn --repository <repo>` by hand starts one at any time.

Pause: no watch-tick handler starts, resumes or stops a session of a repository with a `Paused`
controller. `Tick::restart_with` answers before it stops anything (`session gone`, a usage-limited
hand-over), and the resumes of `session exited (blocked)`, a usage limit's reset and a withdrawn
token resume nothing. One journal line `<repo> is paused: <session> not restarted` (or `not
resumed`), said once while the pause lasts (`paused.json`), the controller left `Paused`; the
session's hand-over request and usage-limit mark are kept for the controller `resume-controller`
resumes. `session stalled` and `session silent` and the hand-over read only a
`Running` controller, and the cycle starts none for a `Paused` repository. Only `controller
resume-controller`, or `stop-controller` and then a start, ends a pause. A pause that ends the
session before it is recorded can still be read as `session gone` in between (a follow-up).

Dispatch v1: each cycle, for each Active repository with a charter and no Running controller,
spawn one, until `controllers.max_working` run; a repository marked `merge: human` with
`controllers.max_open_reviews` open pull requests waits. Every controller the daemon starts (dispatch
v1, a red `main`, a restart) works on a dispatch: the repository's open one, else one sent in the
same tick for the goal the repository serves (a confirmed goal before a draft); the charter it is
started with ends in a `## Dispatch` section naming the id. A repository that serves no open goal
gets none, and its controller reports with `-`. The charter stays hand-written until
`story:charter-generated`.

Alerts: a condition the loop or the daemon cannot act on reaches the operator through one route,
`crate::alert` (`story:loop-alerts-when-it-cannot-act`): one `alert` line in the dispatch log
(`condition`, `repo` or the instance's name, `reason`, and `id` when the subject is a dispatch) and
a critical desktop notification titled `<instance>: alert`, through the same `notify-send` the
`escalated` and `unanswered` lines use; no config key chooses it. Each condition and subject alerts
once: the pair is kept in `alerts.seen` (`<state>/run/` for the loop, the daemon's own directory for
the daemon) and forgotten when the condition clears, so a second occurrence alerts again. A
notifier that is missing or fails is a journal line; the alert is recorded all the same, and
`conductor doctor`'s notifier row names the gap. The first condition, `unit-left-to-operator`: a
unit of work whose start failed at the launch three times, which dispatch v1 tries no more; it
clears when the dispatch is no longer `Sent`. `box-not-recreated` (the daemon,
`story:failing-start-recreates-box`): a box substrate answered as gone was forgotten and the new
box's start failed, subject the instance; it clears once a new box ran past the start bound. A
condition the loop acts on itself alerts nothing.

Decide: a request is classified in code: data loss, a history rewrite, a new, renamed or retired
repository, an ADR or a change to one, a credential, a login, a paid account → class O (or H for the
hands cases); everything else class C. Class C is answered by the `Decider` adapter with one bounded
prompt: the request and its options, the repository's accepted ADR titles, the role's rules file,
the operator's decision principles from the instance records, and the repository's decisions of the
last 7 days; the answer is one letter and one line of reason, recorded with
`record-conductor-decision` (evidence names the adapter and the prompt's hash) and delivered. The
first adapter is `claude -p` on the model `roles[conductor].model`; `llm` and Loom replace it behind
the same trait. Class O and H are escalated with the options parsed from the request, a default and
a deadline, and shown on the Decisions page; the handler delivers the operator's answer.

## 4. Sessions

Every session is started by `conductor spawn` or `conductor session start` through the systemd
provider with the tmux launcher, so `conductor session attach` reaches each one and `session
deliver` can type into it. No profile asks a session to arm a timer, to watch anything, to hand
itself over, to resume anyone, to free disk, or to relay a line: the daemon does these. The
controller profile keeps: read the charter and the repository's `AGENTS.md`, plan in the store,
deliver in waves in managed trees, report with `conductor message send`, build with `conductor
build run`, stop on `[HANDOVER]` after writing the hand-over file. The conductor profile is
retired. The maintainer profile is at most 60 lines. The director's profile is back
(`story:box-runtime-starts-director`), at most 60 lines, for the session that answers the
operator: the runtime of a boxed instance starts it in the box, which reads no role file, and
keeps it running; it lists what the session reads at start and arms, watches and relays nothing.

Isolation: `story:session-home-spike` starts one controller with `HOME`, `CLAUDE_CONFIG_DIR` and
`TMPDIR` under `<instance>/home/` and records what the harness then lacks (plugins, trust, the
sessions list, transcripts the watch reads) and the fix for each, before the setting becomes the
default. The write fence (bwrap) stays `epic:session-confinement`.

## 5. Acceptance

- The instance runs 24 h with no live session named `<prefix>-conductor` and no `CronCreate` in any
  transcript of the instance.
- 0 sessions of the instance over `context_handover` at their last turn (K16).
- 0 relay turns by a model: every `relayed`, `handover-sent`, `resumed` and `decision-sent` line in
  the dispatch log is written by the daemon.
- every decision request has a record, and its class was set by code.
- `metric compute` has one reading per metric per day.
- every message between roles is a `conductor message send` or `session deliver` record.

## 6. Stories

`epic:conductor-run`: `story:mailbox-send-and-deliver`, `story:run-daemon` (adopting
`story:cycle-command`, `story:report-to-state` and `story:auto-resume`), `story:decide-model-adapter`,
`story:session-home-spike`, `story:controller-profile-for-run`, `story:maintainer-guard`,
`story:charter-generated` (v2). `story:watch-waits` and `story:handover-draft` stay in their epics
and are picked up by the daemon when they land.

## 7. Not in this design

The bwrap write fence (`epic:session-confinement`), the TUI and the host view
(`epic:dashboard-for-engineers`), generated charters beyond v1, and any new rule text.
