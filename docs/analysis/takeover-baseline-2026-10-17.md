# Baseline before the loop moved into the binary, 2026-10-17

The "before" numbers for the `conductor run` work (`docs/design/conductor-run.md`). One instance,
window 2026-10-14T16:00Z to 2026-10-17T20:00Z unless stated. Instruments: `conductor resource
usage --since`, `conductor repository shipped --since`, the records' decision and dispatch logs,
`conductor metric kpi-readings`, and a transcript reader over the two conductor-role sessions
(distinct `requestId` per API call, context = input + cache read + cache write of each call).

| measure | value |
|---|---|
| input tokens, instance sessions (conductor, conductor-dev, controllers, workers) | 3,355M |
| of which the conductor-dev session | 971M (29 %) |
| of which the conductor session (10 h) | 351M |
| merged pull requests / releases in the window | 118 / 76; the conductor product alone 15 / 15 |
| K14 merged pull requests per 100M input tokens | 3.5 (2026-10-14 reading: 1.8; target ≥ 10) |
| operator decisions recorded (O / H / C) | 45 / 11 / 7 in 4 days; 94 requests raised, 29 withdrawn |
| K1 operator touches per merged pull request | 0.53 (first-week baseline 0.245; target ≤ 0.10) |
| operator messages typed into the conductor-dev session | 99 in 75 h |
| K16 sessions over `context_handover` (200k) at their last turn, 2026-10-17 | 59 of 118 |
| conductor-dev session context, p50 per day | 205k, 293k, 379k, 527k; max 924k; 3 compactions, all typed by the operator |
| conductor session context, 2026-10-17 | p50 432k, max 797k over 815 calls |
| what the conductor session sent in 10 h | 268 messages: 236 `[REPORT]` relays, 16 `[DECISION]`, 16 `[HANDOVER]` |
| words in those messages | "go" 112, a disk size 99, "release" 90, "queue" 55, "hold" 50, "reserve" 40 |
| session-only timers in the conductor-dev session | 9 `CronCreate`; the work tick's text never updated after 2026-10-14 |
| KPI readings produced by `metric compute` since the design was approved (2026-10-13) | 0 |
| instance rules file | 98,546 bytes, 151 items; the sessions that were to follow it ran at 400k–900k context |
| hand-over mechanism | a message a session sends to itself at 200k; nothing in code stops a session |
| restart of the conductor session | hand-over written 17:10Z, session still running at 20:17Z |
| guard denials recorded | 66 in 4 days |
| disk | 200G free fell to 15–19G three times on 2026-10-17; recovered only by the operator clearing 58G |

Reading. The sessions are cheap per turn (cache reads) and deliver little per token, because the
loop's routine work (disk and build words, relays, restarts, resumes, hand-overs) is done in model
turns at 2–4.6× the context where instructions hold, and the rules that were to govern it are prose.
The organization's thesis (governed autonomy: "what must be true" is decided by code, a model
chooses how) is inverted in the tool that runs it. `conductor-run.md` moves the routine into the
binary and leaves the model one job: answering a decision request within a bounded prompt.

Targets after the move (read daily by `metric compute`): K16 = 0; K14 ≥ 10; K1 ≤ 0.10; 0 relay
turns by a model; one binary and one state directory per instance, nothing outside it.
