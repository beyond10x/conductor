---
format: aep.planning-md/3
id: story:conductor-context-budget
kind: story
status: draft
title: Conductor runs 4 hours before its hand-over threshold
summary: Measure what fills conductor's context and cut the largest source
relations:
- decomposes: epic:measured-self-improvement
revision: 2
---
## Why

On 2026-10-14 conductor reached its hand-over threshold four times: restarts at about 16:35Z (273k), 17:27Z (200k, about 40 minutes after the previous start), 18:52Z (202k) and 20:34Z (204k, about 100 minutes after the previous start). Each restart re-reads the start of session with a cold cache. The controller-health run of 16:20Z read 34.8M input tokens for conductor over 3 hours (199 turns), 99 % of it the context re-read. Neither the share of the start reading nor the share of message traffic per turn is measured.

## Acceptance

- `conductor health` (or the health report) prints, per conductor session, the context after its first turn and the context added per hour by each source: tool output, messages received, its own output; read from the session transcript.
- One measured change brings conductor's time from start to `thresholds.context_handover` to at least 4 hours on a day with 8 working controllers, recorded as evidence with the before and after figures from two health reports.

## Scope

`crates/conductor/src/health.rs` or the controller-health skill's report, `.agents/conductor.md`, tests.

## Measured (2026-10-15, conductor-dev)

Sessions 8a98b1a4, 4e3194ba and d1bd2be9 handed over at 208.6k, 213.9k and 225.7k tokens. Each started at 51.6k on its first turn and reached 81–95k before doing any work. Per session:
- first-turn prompt: 51.6k, of which about 25k are tool definitions conductor never called (Artifact alone is 57.8KB);
- its own output: 57.9k;
- tool results after the start reading: ~47k;
- start reading: ~36k, with rules sections at 44.6KB;
- messages: ~30k in and out, including 29KB of received messages re-typed into `receive-message --first-line`.

`conductor status` and the board are not significant consumers.

## Acceptance (added from the measurement, ranked by tokens saved)

- Conductor's start denies the tools it never calls: Artifact, Workflow, ScheduleWakeup, EnterWorktree, AskUserQuestion, ReportFindings and ShareOnboardingGuide. The first-turn context is at most 30k tokens, down from 51.6k, read from the transcript's first `usage`. A test holds the harness arguments.
- The start reading is halved:
  - the rules sections are read titles-only;
  - the newest hand-over is read instead of design §§ 4–8, NORTHSTAR and STATUS in full;
  - decisions and dispatches are read since the hand-over;
  - no file is read twice;
  - the context at the first `CronCreate` is at most 65k tokens.
- Received messages are logged without conductor re-typing them, and a controller's REPORT first line is at most 300 characters.

## Scope (scoper, medium confidence)

`.agents/conductor.md` start list, `crates/conductor/tests/profiles.rs`, `crates/conductor/src/session.rs` `harness_args` (inferred), `.claude/conductor-settings.json` (inferred), `crates/conductor/src/rules.rs` and `cli.rs` `RulesShowArgs` for a titles-only mode (inferred), `crates/conductor/src/health.rs` (via story:health-command), `docs/analysis/conductor-context-budget.md` (new). Open: whether a tool deny removes the definition from the prompt (unproven).

## Measured again (2026-10-17, conductor-dev)

Conductor filled 200k three times between 05:50Z and 08:06Z, about every 75 min (NEEDs N-20261017-06, -09, -11). Session 7b51080e (06:50Z–08:06Z), in characters:
- tool results 217k over 216; Bash inputs 120k over 193 calls; SendMessage 45k over 66.
- Largest results:
  - `message receive-message`: 22.7k (it echoes the message back).
  - `rules show --section conductor --brief`: 18.8k, read twice.
  - `controller controllers`: 15.9k, read twice.
  - `status | head -120`: 8.3k.
  - The hand-over: 7k.
- 91 of 193 Bash calls begin `cd <records> &&`.

Added acceptance, ranked by today's characters:
- `receive-message` answers one line, not the body.
- `rules show --brief` answers item numbers and titles only, plus `--changed-since <commit>`.
- `controller controllers` defaults to one line per Running or Paused controller (`--all` for the table).
- `status` defaults to the attention lines.
- Commands find the records from anywhere, so the `cd` prefix goes.
