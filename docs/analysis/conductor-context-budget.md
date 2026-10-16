# What fills conductor's context at start, and what the start drops

Read 2026-10-15, for `story:conductor-context-budget`. "Measured" means read from the transcripts of
three conductor sessions of one instance on 2026-10-14 and 2026-10-15; "expected" means not yet
measured on a live session.

## Before (measured)

Three sessions handed over at 208.6k, 213.9k and 225.7k tokens. Each started at 51.6k tokens on its
first turn and reached 81–95k before it did any work. Per session:

| source | tokens | note |
|---|---|---|
| first-turn prompt | 51.6k | about 25k of it are definitions of tools conductor never called; Artifact alone is 57.8 KB |
| its own output | 57.9k | |
| tool results after the start reading | ~47k | |
| start reading | ~36k | the rules sections alone are 44.6 KB |
| messages in and out | ~30k | 29 KB of received messages re-typed into `receive-message --first-line` |

`conductor status` and the board are not significant consumers.

## The change

1. **Unused tools.** `.claude/conductor-settings.json` denies Artifact, Workflow, ScheduleWakeup,
   EnterWorktree, AskUserQuestion, ReportFindings and ShareOnboardingGuide in `permissions.deny`.
   `tests/guard_hook.rs` holds the list, and that the controllers' settings deny none of them. A
   role whose config names its own `settings` file starts without this list.
2. **Start reading** (`.agents/conductor.md`):
   - the rules for conductor's role are read titles-only, `conductor rules show --section <s>
     --brief`, one line per item (`<n>. <first sentence>`); an item is read whole with `--item`
     when a cycle needs it;
   - the newest hand-over is read first, and design §§ 4–8, `NORTHSTAR.md` and `STATUS.md` only by
     the section it points to, in full only when there is no hand-over;
   - `decisions/` and `dispatches/` are read from the hand-over's commit time, not their last 50
     lines;
   - no file is read twice: the first prompt acts on the hand-over the start reading opened instead
     of having it read again.

Not part of this change: the per-source figures in the health report, and logging received
messages without conductor re-typing them.

## Whether a denied tool leaves the prompt

Unproven. The probe — `claude -p --output-format json --disallowedTools Artifact "reply ok"` against
the same without the flag, comparing `usage` input and cache tokens — could not run on the
measuring host: both calls ended `api_error` 401 before any request was answered, so neither printed
a usage figure. The deny is in the settings file either way; the first-turn figure of the next
conductor session decides it.

## After (expected)

| figure | before | expected after | how |
|---|---|---|---|
| first-turn context | 51.6k | at most 30k (about 26.6k) | 51.6k less about 25k of tool definitions, if a denied tool's definition leaves the prompt |
| start reading | ~36k | at most 18k | the rules sections, 44.6 KB or about 11k tokens in full, become one line per item; design, `NORTHSTAR.md` and `STATUS.md` by section; the logs since the hand-over |
| context at the first `CronCreate` | 81–95k | at most 65k | the two rows above |

Each "after" figure is to be read from the transcript of the first conductor session that starts on
this change: the first `usage` for the first-turn context, and the `usage` of the turn that calls
`CronCreate` first.
