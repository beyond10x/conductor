---
format: aep.planning-md/3
id: review-result:w51-u2-adversary-2
kind: review-result
status: active
title: Wave 51 U2 handover-restart-by-controller-id, adversary pass 2
relations:
- reviews: story:handover-restart-by-controller-id
revision: 1
---
```
unit: U2 wave 51, story:handover-restart-by-controller-id, head 4fd9fe03d plus 169 uncommitted test lines in the worktree
verdict: INFEASIBLE (one note); nothing found that a case turns red
cases: executed 1845→1848, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths
needs-coordinator: none
```

The correction holds against everything I tried. None of my three new cases fails. The one thing I'm returning is a note on how the loop dates its own resume, and I could not show it happening.

## 1. Diff stat of the worktree

```
 crates/conductor/tests/run_handover.rs | 169 +++++++++++++++++++++++++++++++++
 1 file changed, 169 insertions(+)
```

Only the test file changed. No implementation file was touched.

## 2. Cases added

All three are in `crates/conductor/tests/run_handover.rs`, in a new "Adversary pass 2" section after `:1119`. All three pass now. Run alone first: `cargo nextest run -p conductor-cli --test run_handover --no-fail-fast adversary2_`, 3 run, 3 passed, exit 0.

- **`adversary2_a_start_by_hand_the_loop_resumes_before_a_tick_keeps_the_request`**
  - This is the case the implementor left untested: someone resumes the session by hand under its id after the delivery, then `conductor leave` makes the loop end and resume it before any tick sees the hand's start.
  - The request stands and carries the `resumed` stamp. At the hand-over the loop stops only `s0000001-0000`, the conversation that holds the request.
  - A session with another id cannot be saved by the stamp: `superseded` reaches the start check only for a live session carrying the asked id (`handlers.rs:448`). `spawn::resume` resumes the id the envelope records, and that matches the asked id at delivery (`measured_is_enveloped`). So this is not a finding.
- **`adversary2_a_replaced_controller_resumed_by_the_loop_stops_nothing`**
  - Acceptance 1 with the stamp in play: an entry for controller A, controller B under the same name, and B's session resumed by the loop.
  - The entry is forgotten, the journal names A, and the next tick stops nothing even with an idle session and a hand-over file. B runs on.
- **`adversary2_a_limited_session_whose_request_cannot_be_told_stays_marked`**
  - This pins the claim "list unreadable: the session stays marked, nothing stopped".
  - With headroom and a failing `claude agents`, nothing starts. `limited.json` and the request are kept, and one `hand-over of ex-alpha` journal line is written.
  - The first tick that can read the list again spawns afresh.
  - It also catches a mutant the suite missed before: in `limited()` (`:654`), resuming or forgetting on an `Err` from `stands` instead of `continue`.

## 3. Suite run, after the cases existed

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --no-fail-fast`, exit 100:

```
    Starting 1848 tests across 152 binaries (12 tests skipped)
        FAIL [  31.454s] ( 688/1848) conductor-cli::build_run a_second_signal_ends_a_stopped_run_that_waits_for_the_ledger
     Summary [ 277.643s] 1848 tests run: 1847 passed (4 slow), 1 failed, 12 skipped
```

The one failure is the known `build_run` failure under load. The `<before>` of 1845 is the implementor's reported count.

## 4. Findings (tree: 4fd9fe03d plus the cases above)

| # | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| N1 | `crates/conductor/src/run/handlers.rs:707` | INFEASIBLE | introduced | Read only. The stamp is the instant `spawn::resume` returned. Every case dates the resumed process at `now()` read before the tick, so before the resume even ran (`:861`, `:922`). No case dates the start after the stamp. | Not found. If the harness registers `startedAt` for `claude --bg --resume` after the command returns, the session the loop resumed counts as younger than the stamp. The request is dropped, and the watch never asks that session id again. Nothing in the tree shows what `startedAt` is after a resume. |

- **N1 fix, named, not applied:** take the stamp as a bound with a margin, or have the first tick after the stamp record the listed `startedAt` as the loop's own.

## 5. Attacked and not broken

- **Stamp written only on success:** it is written only when `spawn::resume` returns `Some`. A failed resume, a strict-box refusal or `None` writes nothing.
- **Stamp in `limited()`:** `forget` removes it at once there, but in that path an entry that stands is restarted, not resumed, so no request is lost.
- **Two resumes in a row:** the stamp moves to the later one.
- **A new delivery:** `context()` rewrites the whole entry, so an old stamp is cleared.
- **Clock stepping back:** the later-of falls back to the delivery, and the start lies before it. Every compared instant is in nanoseconds, against `startedAt` in milliseconds times 10^6.
- **Stamp from a resume of another session of the name:** it only matters for a live session carrying the asked id.
- **`on_limit` keeps the entry:** it cannot outlive its session for ever. The `silent_session` fallback, or the "running controller is another session" forget, ends it. That is the same per-tick work as any kept entry on the base, with no extra journal line.
- **List never readable:** one stderr line per tick per marked session, as `handovers()` already writes. On the base, the restart failed on the same list read and the session was forgotten.
- **`listed`:** `live_session_within` behaves exactly as on the base. The removed `live_entry` had one caller, `superseded`. Attach, deliver, watch and leave do not change.
- **1.2.0 lines without `(session <id>)`:** v1.2.0's watch always wrote `(session <8 chars of sessionId>)` (`watch.rs:1648`), skipped sessions with no id, and `context()` was the only writer of `handover.json`. No such line exists.
- **Line parse:** `rsplit_once` takes the last group. A repository name cannot contain one, and the `line` is the watch line, not the hand-over text.
- **Mutants, by reading against the cases:**
  - ignoring the stamp: caught by `:852` and `:908`;
  - `stands` always yes: caught by `:1016` and `:949`;
  - earlier-of instead of later-of: caught by the baseline `:584`;
  - `listed` returning only the first entry: caught by `:1050`'s listed-before variant;
  - the untied check without `of.is_none()`: caught by `:852`.
- **Acceptance 3:** `deliver::each_delivery_types_through_a_buffer_of_its_own_name` and `session_attach::two_calls_typing_into_one_server_at_once_do_not_consume_each_others_buffer` pass on this head.
- **Fine-clock helper:** every caller writes the file after the delivery and expects it newer. No case expects an older file, so the helper hides no defect.
- **`docs/design/conductor-run.md:64`:** each sentence matches `superseded`, `stands` and `resume`. "Since the delivery" for the file and `silent_session` still uses `delivered`, as the code does.

## 6. Paths written outside the worktree

- `<scratch>/w51/u2/adv2/cases-alone.log`
- `<scratch>/w51/u2/adv2/suite.log`

Builds went to the shared target through `conductor build run`. No commit and no git write.

## 7. Findings block

```findings
- file: crates/conductor/src/run/handlers.rs
  line: 707
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'the resume stamp is the instant the resume command returned, so a harness that dates the resumed session''s startedAt after that instant would drop the request the loop''s own resume was meant to keep; no case dates the start after the stamp and the tree does not show the harness''s behaviour'
```
