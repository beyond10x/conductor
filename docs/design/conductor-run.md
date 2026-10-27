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
| resume after a usage limit | the session lists and respawns by prompt | the handler marks the sessions and respawns when headroom returns | no |
| a stalled or silent controller | the session reads transcripts and nudges | one check-in delivered; a second stall respawns | no |
| a controller's report | `SendMessage` to the conductor session, recorded by it | `conductor message send` writes the mailbox; the daemon moves the dispatch (`report-to-state`) | no |
| a `[NEED]` | routed by the session | recorded; delivered to the other repository's live controller, else queued for its next start | no |
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
`records check`, dispatch v1); at `cadence.daily` `metric compute` for the previous UTC day, the
brief file `REPORT.md` in the records (deviations only), and then a commit of the records directory
through the bot route, pushed when the records repository has an `origin`; a refusal is one
`records-commit-refused` line, not retried (`story:records-committed-by-daemon`; `conductor run
--once --commit-records` runs the commit alone).

| watch line | handler | records |
|---|---|---|
| `context: <repo> <n>k` over `context_handover` | deliver `[HANDOVER <repo>] context <n>k` once per session; over 1.25× the threshold, or when `<checkout>/.git/handoff/` has a file newer than the delivery, stop the unit (`spawn::end`), append the hand-over path as the resume point to the charter, spawn again | `handover-sent`, `handover`, `controller-restarted` |
| `usage limit: …` | mark each named session `limit`; on the next tick where `ais status --json` (or the transcript's next successful call) shows headroom, resume (`spawn::resume`) each marked session, or respawn one over the threshold | `usage-limit`, `resumed` |
| `PR checks done: <repo>#<n> …`, `CI red on main: <repo> …`, `CI green again …` | deliver the line as `[REPORT <repo> -] watch: <line>` to the live controller; red `main` with no controller: dispatch v1 spawns one when the repository has a charter, else a deviation line | `relayed`, `red-ci` |
| `session stalled`, `session silent` | first: deliver `[REPORT <repo> -] check-in: report your state or blocked`; second for the same session: stop and respawn from the charter | `stalled`, `controller-restarted` |
| `session gone`, `session exited (blocked)` with an open dispatch | respawn from the charter | `controller-restarted` |
| `disk low` | `gc` (the reclaim the watch already runs under disk admit stays) | `cleanup` |
| `escalated`, `unanswered` | a desktop notification; the Decisions page already lists it | none |
| `answered <id> <choice>` | deliver `[DECISION <id>] <choice> …` to the controller that raised it | `decision-sent` |

Mailbox: a controller reports with `conductor message send --to conductor --from <session>
--text <file|->`, which parses the first line with the existing grammar (unparsed: exit 1 and
`FORMAT`), records the message (`receive-message`, transport `Mailbox`, `body_path`) and leaves the
body under `<state>/mailbox/conductor/<ulid>.md`. The daemon reads new files each tick:
`[REPORT]` moves the dispatch (`report-to-state`); `[NEED]` is routed; `[DECISION-REQUEST]` goes to
decide; `[RESOURCE]` is answered "use conductor build run"; `[HANDOVER]` from a controller
(`blocked handover <path>`) is a report. Delivery to a session is `conductor session deliver --name
<session> --text <file|->`, which types the text into the session through the tmux launcher
(`type_text`) and records `delivered`; a session the launcher cannot reach is reported, never
retried in a loop. `SendMessage` is no longer used between roles; the guard allows `message send`
and denies `session deliver` to a controller.

Dispatch v1: each cycle, for each Active repository with a charter and no Running controller,
spawn one, until `controllers.max_working` run; a repository marked `merge: human` with
`controllers.max_open_reviews` open pull requests waits. The charter stays hand-written until
`story:charter-generated`.

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
retired. The maintainer profile is at most 60 lines.

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
