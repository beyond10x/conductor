---
format: aep.planning-md/3
id: story:conductor-context-budget
kind: story
status: active
title: Conductor runs 4 hours before its hand-over threshold
summary: Measure what fills conductor's context and cut the largest source
relations:
- decomposes: epic:measured-self-improvement
scope:
- confidence: inferred
  path: crates/conductor-model/src/observation.rs
- confidence: cited
  path: crates/conductor/kinds/claude-code/conductor-settings.json
- confidence: inferred
  path: crates/conductor/roles/maintainer.md
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/controller.rs
- confidence: cited
  path: crates/conductor/src/rules.rs
- confidence: cited
  path: crates/conductor/src/session.rs
- confidence: cited
  path: crates/conductor/src/status.rs
- confidence: cited
  path: crates/conductor/tests/controller.rs
- confidence: cited
  path: crates/conductor/tests/guard_hook.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: crates/conductor/tests/records_log.rs
- confidence: cited
  path: crates/conductor/tests/rules.rs
- confidence: cited
  path: crates/conductor/tests/status.rs
- confidence: cited
  path: docs/analysis/conductor-context-budget.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T00:06:56Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-08T00:06:56Z", actor: "human:timo", revision: 6}
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

Conductor filled 200k three times between 05:50Z and 08:06Z, about every 75 min (three conductor NEEDs, 2026-10-17). Session 7b51080e (06:50Z–08:06Z), in characters:
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

## Measured 2026-11-08

Read from the maintainer sessions' transcript usage lines, reported by the host teacher (2026-11-08 01:05 CET):

- one maintainer session stopped at 00:05 CET;
- its successor started at 45,190 tokens, was at 139,251 after 22 minutes, and handed over at 161k at about 01:0x CET;
- the next successor started at 47,105 tokens and was at 98,947 after 44 turns, at 01:04 CET.

The host supervisor counts this as a stalled instance by hand-over rate (two hand-overs within 2 hours; a first turn over a quarter of `thresholds.context_handover`) and moved this story to `active` as the maintainer's next unit. The acceptance above, written for the conductor session, applies to the maintainer session too.

## Maintainer start reading (2026-11-08, conductor-dev session e8d630ac)

Characters of each start-reading source on 2026-11-08 (`wc -c`, `conductor rules show`):

| source | in full | as read after this change |
|---|---|---|
| `rules.md` conductor-dev sections (`--section conductor-dev`) | 29,178 | 2,414 (`--brief`) |
| `rules.md` § All sessions | 45,426 | 4,488 (`--brief`) |
| design §§ 2–6 and § 12 | 16,487 | 0 at start |
| `AGENTS.md` | 19,741 | 19,741 |
| newest hand-over | 3,941 | 3,941 |
| total | 114,773 | 30,584 |

The first turn, before any reading, was 45,550 tokens in this session (transcript's first `usage`); its first `CronCreate` came at 69,833, after reading the conductor-dev sections in full, design § 12 and the hand-over, and before design §§ 2–6, `AGENTS.md` and § All sessions.

Tool deny, measured with two background `claude --bg` sessions on Haiku in an empty directory, prompt "reply ok": 28,269 first-turn tokens without a deny, 26,500 with `--disallowedTools Artifact,ArtifactComments,ArtifactData,Workflow,ScheduleWakeup,ShareOnboardingGuide,ReportFindings,AskUserQuestion`. A deny removes the definitions from the prompt (the open question above), worth 1,769 tokens there.

Change: `.agents/conductor-dev.md` reads the hand-over first, `rules.md` by item titles (`--brief`) with an item in full by `--item <n>` when a step touches it, and design §§ 2–6 and § 12 only when a change touches them; held by `conductor_dev_starts_on_the_hand_over_and_the_rules_titles` in `crates/conductor/tests/profiles.rs`. The after reading is the first-`CronCreate` context of the next maintainer session started on it.
