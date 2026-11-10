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

## What the binary holds now (2026-11-10)

The conductor session is retired: `conductor run` runs the loop with no model call, and the
sessions that fill a context are the director, started by hand, and the maintainer. The files
named above moved into the binary (`crates/conductor/kinds/claude-code/conductor-settings.json`,
`crates/conductor/roles/maintainer.md`).

| acceptance | held by | test |
|---|---|---|
| the report prints, per session, the first-turn context and what each source adds per hour | the fourth table of `conductor resource usage`, `context` | `tests/usage.rs`: `usage_prints_each_session_s_first_context_and_what_each_source_adds_per_hour`, `since_counts_the_context_added_from_that_instant` |
| the start denies the tools the role never calls | `--disallowedTools` in the start arguments of the director (the seven tools above) and of the maintainer (Artifact, ScheduleWakeup, AskUserQuestion, ReportFindings, ShareOnboardingGuide), with or without a settings file of the role's own | `session::start_arguments::the_director_and_the_maintainer_start_without_the_tools_they_never_call`, `tests/confine.rs`: `the_provider_is_given_the_role_s_request_and_first_prompt`, `tests/taskfile.rs`: `dev_start_appends_the_binary_s_maintainer_profile_with_settings_pinned_to_its_instance` |
| the start reading is halved | the maintainer profile: the hand-over first, the rules by title, the logs since the hand-over, no file twice | `tests/profiles.rs`: `conductor_dev_starts_on_the_hand_over_and_the_rules_titles`, `the_maintainer_reads_the_logs_since_its_hand_over_and_no_file_twice`, `the_maintainer_s_start_reading_from_this_repository_keeps_the_whole_under_half` |
| received messages are logged without a session re-typing them | `conductor message send` leaves the text in the mailbox, logs it and answers its id alone | `tests/mailbox.rs`: `a_sent_message_is_answered_with_its_id_alone` |
| a REPORT first line is at most 300 characters | `message::read`: a longer one does not parse and is answered with the format | `tests/mailbox.rs`: `a_report_whose_first_line_is_over_300_characters_is_refused` |

A denied tool leaves the prompt: measured 2026-11-08 with two background sessions in an empty
directory, 28,269 first-turn tokens without `--disallowedTools` and 26,500 with eight tools
denied. The maintainer keeps Workflow and EnterWorktree, which a wave may call.

### The context table

One row per session with a call counted: `first_context`, the input, cache-read and cache-write
tokens of its first call; `hours`, from its first line, or `--since` when later, to its last;
and per hour `own_output_per_hour`, the output tokens of its calls, `tool_output_per_hour` and
`messages_per_hour`, the characters of its tool results and of every other text a user line
carries, at four characters a token. The last two are estimates: a transcript holds no token
count for them.

### Not measured

No test can hold these; each is read on a live instance with `conductor resource usage`:

- first-turn context at most 30k tokens: `first_context` of the first director or maintainer
  session started on this change (before: 51.6k for conductor, 45.5k to 47.1k for the
  maintainer);
- context at the first `CronCreate` at most 65k tokens: the `usage` of the turn that calls it
  first, read from that transcript (before: 69.8k for the maintainer);
- at least 4 hours from start to `thresholds.context_handover` on a day with 8 working
  controllers: two reports of `conductor resource usage --since <start of day>`, one from a day
  before this change and one after, each with the session's `first_context`, its three per-hour
  figures and the time between its first call and its hand-over. With the threshold at T tokens
  the three per-hour figures must sum to at most (T - `first_context`) / 4.
