---
format: aep.planning-md/3
id: review-result:w49-u8-adversary-1
kind: review-result
status: active
title: Wave 49 U8 conductor-context-budget, adversary pass 1
relations:
- reviews: story:conductor-context-budget
revision: 3
---
unit: U8 story:conductor-context-budget, commit 0bc8408b2 on base 1e0be7a7a, worktree ~/.local/state/worktree/trees/b10x/conductor/w49-u8
verdict: NEEDS-CHANGE
cases: executed 52→56, red 3
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 6 paths
needs-coordinator: none

`executed 52` is the 56 of the suite run less my four cases (three binaries: usage, mailbox, profiles); no separate deselected run was made, to stay within two build requests.

## 1. git --no-pager diff --stat

```
 crates/conductor/tests/mailbox.rs | 70 +++++++++++++++++++++++++++++++++++++++
 crates/conductor/tests/usage.rs   | 33 ++++++++++++++++++
 2 files changed, 103 insertions(+)
?? crates/conductor/tests/fixtures/usage/context-short.jsonl
```

Test files only. Nothing committed.

## 2. Cases added (run alone first, request 1: `4 tests run: 1 passed, 3 failed, 21 skipped`, exit 100)

| case | file | asserts | now |
|---|---|---|---|
| `a_session_whose_printed_hours_are_0_has_0_in_each_per_hour_column` | tests/usage.rs, fixture `context-short.jsonl` | a one-turn session of 9 seconds prints `hours` 0.0, so each per-hour column is 0, as the module documentation of usage.rs says | red |
| `a_long_first_line_of_another_kind_that_breaks_its_form_is_not_told_the_report_bound` | tests/mailbox.rs | a malformed `[NEED cortex] …` of 350 characters is not refused with "over the 300 a first line holds" | red |
| `the_first_line_the_log_holds_of_a_report_is_at_most_300_characters` | tests/mailbox.rs | a report padded with 400 trailing spaces is not logged with a first line over 300 characters | red |
| `a_first_line_of_another_kind_is_not_bounded_and_characters_are_counted` | tests/mailbox.rs | 300 non-ASCII characters (over 300 bytes) parse, 301 do not; a 400-character NEED and DECISION-REQUEST parse and a long NEED is sent | green (mutant guard) |

Red output, verbatim, from the run of the cases alone:

```
thread 'a_session_whose_printed_hours_are_0_has_0_in_each_per_hour_column' panicked at crates/conductor/tests/usage.rs:806:9:
assertion `left == right` failed: hours are 0, so messages_per_hour is 0 (usage.rs module documentation): {
  "first_context": 30000,
  "hours": 0.0,
  "messages_per_hour": 4100,
  "own_output_per_hour": 40000,
  "repository": "other",
  "session": "aaaaaaaa",
  "tool_output_per_hour": 0
}
  left: Number(4100)
 right: 0
```

```
thread 'a_long_first_line_of_another_kind_that_breaks_its_form_is_not_told_the_report_bound' panicked at crates/conductor/tests/mailbox.rs:472:5:
a need has no 300-character bound; the line breaks its form: conductor: message send: refused: Unparsed: from "ex-cortex" to "conductor": the first line is 350 characters, over the 300 a first line holds: put the detail in the body; the format is on standard output, and nothing was written
```

```
thread 'the_first_line_the_log_holds_of_a_report_is_at_most_300_characters' panicked at crates/conductor/tests/mailbox.rs:521:5:
a report was accepted and logged with a first line of [440] characters: exit Some(0)
stdout:
01a1c3d7-f19c-77b6-9277-b0fb3485c7e8
```

## 3. Suite run (request 2, after the cases existed)

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test usage --test mailbox --test profiles --no-fail-fast`

```
    Starting 56 tests across 3 binaries
     Summary [   0.386s] 56 tests run: 53 passed, 3 failed, 0 skipped
        FAIL conductor-cli::mailbox a_long_first_line_of_another_kind_that_breaks_its_form_is_not_told_the_report_bound
        FAIL conductor-cli::usage a_session_whose_printed_hours_are_0_has_0_in_each_per_hour_column
        FAIL conductor-cli::mailbox the_first_line_the_log_holds_of_a_report_is_at_most_300_characters
error: test run failed
exit 100
```

The only failures are the three added cases.

## 4. Findings (all cover commit 0bc8408b2)

| # | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| 1 | crates/conductor/src/usage.rs:598 (`per_hour`, `hours > 0.0` on the unrounded value) | a 9-second session prints `hours: 0.0` with `own_output_per_hour: 40000`, `messages_per_hour: 4100`; the module documentation says a session whose hours are 0 has 0 in each per-hour column. Any session under 18 s rounds to 0.0 and scales seconds to an hour | every short session in the table: a one-shot `claude -p`, the `reply ok` sessions the story measured the deny with, a session that died at start | NEEDS-CHANGE | introduced |
| 2 | crates/conductor/src/message.rs:576 | the refusal detail tests only the length, not the kind: a malformed NEED of 350 characters is told "over the 300 a first line holds", while a well-formed NEED of 400 is accepted. The sender shortens, and is then refused again for the form | any controller sending a long, malformed non-REPORT first line through `conductor message send` | NEEDS-CHANGE | introduced |
| 3 | crates/conductor/src/message.rs:226 | the bound counts `line.trim()`, and `send` stores `first` untrimmed: a report with 400 trailing spaces is accepted and logged with a 440-character first line | nothing found: a sender has to pad with white space. I built it | INFEASIBLE | introduced |
| 4 | crates/conductor/src/session.rs:1040 (`denied_tools_argument`, `own_settings`) | a director or maintainer whose config names any settings file starts with no `--disallowedTools`, even when that file denies nothing (`{}`); the acceptance says the start denies the tools. Only the unit test of the helper holds this branch; no test holds `harness_args` with `role.settings` set | an instance config with `settings:` on the role; `tests/confine.rs` fixture gives conductor-dev one. The default is none | CONFIRMED | introduced |
| 5 | docs/analysis/conductor-context-budget.md:72 | names `session::tests` as the test of the deny; it is `session::start_arguments::the_director_and_the_maintainer_start_without_the_tools_they_never_call` | a reader following the table | CONFIRMED | introduced |
| 6 | crates/conductor/tests/profiles.rs:769 | `the_maintainer_s_start_reading_…_under_half` holds a real cap (maintainer.md + AGENTS.md at most 46,543 characters; 26,900 today), but `INSTANCE_FILES = 10_843` is a constant standing for the rules titles and hand-over, and the start list's `rules show --role maintainer` and the waves list are in neither side. `a_controller_keeps_a_report_s_first_line_to_300_characters` is a constant pin on prompt text | the instance's rules or hand-over growing leaves the test green | CONFIRMED | introduced |

Fixes, named and not applied: 1, test the rounded hours, or print more decimals; 2, add `reading.kind == Some(Report)` to the length branch; 3, count and store the same string; 4, append the deny regardless, or document the loss in the config reference.

## 5. Attacked and not broken

- Bytes against characters: 300 non-ASCII characters pass, 301 are refused.
- Other kinds: a 400-character NEED and DECISION-REQUEST parse; a long NEED is sent and recorded.
- CRLF: `first_line` uses `lines()`, which drops the `\r`; a bare `\r` does not parse, as before.
- Lower-case tag and leading white space: unchanged from the base (no kind; trimmed).
- Stored long reports: nothing re-reads a stored first line with `message::read` (callers: `receive_message`, `send`, `decide::handle::parse_request` for decision requests only), so no mailbox drain is blocked.
- `--disallowedTools` position: one comma-joined word, always followed by `--permission-mode`; `background_args` keeps order. Resume (`spawn::resume`) is controller-only and gets no deny.
- Roles: `lead`, `controller`, `supervisor`, `teacher` get none; both names of director and maintainer get theirs. maintainer.md names none of its denied tools.
- Usage: no calls gives no row; one call and no user line gives hours 0 and zeros; `--since` after the last instant cannot go negative (clamped); JSON and JSONL only gain the `context` table; no document in the repository fixes the table count.

Not run: `--lib` (`session::start_arguments`), `confine`, `taskfile`; I did not change them.

## 6. Paths written outside the worktree

- ~/.cache/scratch/<session>/w49/u8/adv/usage-append.rs
- ~/.cache/scratch/<session>/w49/u8/adv/mailbox-append.rs
- ~/.cache/scratch/<session>/w49/u8/adv/run.sh
- ~/.cache/scratch/<session>/w49/u8/adv/alone.log
- ~/.cache/scratch/<session>/w49/u8/adv/suite.log
- ~/.cache/scratch/<session>/w49/u8/adv/done
- this report, ~/.cache/scratch/<session>/w49/u8/adv-report-1.md (the seventh; the header counts the six scratch files)

The shared build target directory was written by `conductor build run`.

```findings
- file: crates/conductor/src/usage.rs
  line: 598
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a session under 18 seconds prints hours 0.0 with non-zero per-hour columns scaled from seconds, against the module documentation's "hours 0 has 0 in each per-hour column"
- file: crates/conductor/src/message.rs
  line: 576
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: message send tells the sender of any unparsed first line over 300 characters that a first line holds 300, though only a REPORT is bounded and the line was refused for its form
- file: crates/conductor/src/message.rs
  line: 226
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the bound counts the trimmed line and the untrimmed one is logged, so a white-space-padded report is logged over 300 characters; no sender was shown to pad
- file: crates/conductor/src/session.rs
  line: 1040
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a director or maintainer whose config names any settings file starts with no tool deny at all, and no test holds harness_args on that branch
- file: docs/analysis/conductor-context-budget.md
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the acceptance table names session::tests for the deny test, which is in session::start_arguments
- file: crates/conductor/tests/profiles.rs
  line: 769
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the under-half test caps two repository files against a constant for the instance's files and omits the role file and waves list the start list also reads; the controller 300-character test pins prompt text only
```
