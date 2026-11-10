---
format: aep.planning-md/3
id: review-result:w49-u24-adversary-1
kind: review-result
status: active
title: Wave 49 U24 dashboard-live-controllers-and-report-text, adversary pass 1
relations:
- reviews: story:dashboard-live-controllers-and-report-text
revision: 3
---
unit: U24 story:dashboard-live-controllers-and-report-text, commit 5d28b1e0a on base 3290f2211, worktree ~/.local/state/worktree/trees/b10x/conductor/w49-u24
verdict: NEEDS-CHANGE
cases: executed 63→67, red 2
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 5 paths
needs-coordinator: decide whether a report row's text is bounded (finding 2); build grant 01a1c420-06b2-7115-add1-add817fedf31 was left `Running` in the ledger by my second run

`executed <before>` = the implementor's 26 (`dashboard_sessions` + `dashboard_overview`, its green.log) plus the 37 cases of `dashboard`, which I added to the run and did not touch.

## 1. git --no-pager diff --stat

```
 crates/conductor/tests/dashboard_overview.rs | 111 +++++++++++++++++++++++++++
 crates/conductor/tests/dashboard_sessions.rs |  67 ++++++++++++++++
 2 files changed, 178 insertions(+)
```

Test files only, appended at the end of each file, uncommitted. No existing case changed.

## 2. Cases added

| case | file | asserts | now |
|---|---|---|---|
| `adversary_a_report_the_loop_recorded_with_its_verb_as_its_text_is_the_verb_once` | tests/dashboard_overview.rs | lines `done`/`evidence: "done"`, `blocked`/`reason: "blocked"`, `failed`/`reason: "failed"` read `done`, `failed`, `blocked` | red |
| `adversary_a_report_row_s_text_is_one_bounded_line_whatever_the_report_carried` | tests/dashboard_overview.rs | 50 `failed` lines with a 400-line reason: each row's text has no line break and at most 1200 characters | red |
| `adversary_a_report_row_takes_first_line_before_reason_before_evidence_and_trims_it` | tests/dashboard_overview.rs | precedence of the three keys, trimming, a blank or non-string `first_line` falls through | green (pins a mutant) |
| `adversary_a_restarted_controller_is_its_running_record_s_row_when_its_session_is_not_live` | tests/dashboard_sessions.rs | six controllers stopped and started again with no live session are `not running` / `exited` rows by their Running record, count 0; a stopped one is no row | green (pins a mutant) |

Red output of the cases alone, the first thing executed (`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test dashboard_sessions --test dashboard_overview --no-fail-fast -E 'test(adversary_)'`, exit 100):

```
        FAIL [   1.084s] (2/4) conductor-cli::dashboard_overview adversary_a_report_the_loop_recorded_with_its_verb_as_its_text_is_the_verb_once
    thread '...' panicked at crates/conductor/tests/dashboard_overview.rs:1488:5:
    assertion `left == right` failed: a report with no text of its own reads its verb once
      left: ["done: done", "failed: failed", "blocked: blocked"]
     right: ["done", "failed", "blocked"]
        FAIL [   1.513s] (3/4) conductor-cli::dashboard_overview adversary_a_report_row_s_text_is_one_bounded_line_whatever_the_report_carried
    thread '...' panicked at crates/conductor/tests/dashboard_overview.rs:1558:5:
    50 report rows carry 3314850 bytes of text (longest Some(66297) characters, 50 with a line break, 50 with an escape character), case set up and answered in 1.498787594s
     Summary [   2.344s] 4 tests run: 2 passed, 2 failed, 26 skipped
```

## 3. Suite run, after the cases existed

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test dashboard_sessions --test dashboard_overview --test dashboard --no-fail-fast`

```
    Starting 67 tests across 3 binaries
        FAIL [   1.814s] (52/67) conductor-cli::dashboard_overview adversary_a_report_the_loop_recorded_with_its_verb_as_its_text_is_the_verb_once
        FAIL [   2.650s] (54/67) conductor-cli::dashboard_overview adversary_a_report_row_s_text_is_one_bounded_line_whatever_the_report_carried
     Summary [  33.388s] 67 tests run: 65 passed, 2 failed, 0 skipped
error: test run failed
conductor: build run: the ledger was not free within 30s, or a signal arrived: grant 01a1c420-06b2-7115-add1-add817fedf31 stays Running, and the next command that reads the ledger releases it once its process group is gone
exit 100
```

The only red cases are the two I added. Two build requests in total; the first waited about 45 minutes for disk.

## 4. Findings (cover commit 5d28b1e0a)

| # | file:line | verdict | origin | severity | finding |
|---|---|---|---|---|---|
| 1 | crates/conductor/src/dashboard/overview.rs:410 | NEEDS-CHANGE | introduced | warning | A report that said nothing reads its verb twice: `done: done`, `blocked: blocked`, `failed: failed`. |
| 2 | crates/conductor/src/dashboard/overview.rs:411 | CONFIRMED | introduced | warning | A report row's text is the whole reason or evidence: unbounded, with its line breaks and control characters. |
| 3 | crates/conductor/tests/dashboard_sessions.rs:425 | CONFIRMED | introduced | note | The unit's two cases leave two lines of the change unpinned (mutants survive). |
| 4 | crates/conductor/src/dashboard/repo.rs:303 | CONFIRMED | pre-existing | note | The repository page still takes `Running` records only, so it and the Sessions page now disagree on a controller recorded stopped and live. |

1. **Verb twice.**
   - Measured: `left: ["done: done", "failed: failed", "blocked: blocked"]`, dashboard_overview.rs:1488, exit 100. Base printed the verb alone.
   - Reaches it: `crates/conductor/src/run/mailbox.rs:273`, `:296`, `:308`. The loop records a bare `[REPORT <repo> <dispatch>] done` with `evidence: Some("done")`, and a bare `blocked` / `failed` with the verb as reason. This is the loop's normal path for a report with no text.
   - Fix to name, not applied: treat text equal to the verb as none.

2. **Unbounded text.**
   - Measured: 50 rows carry 3 314 850 bytes; longest 66 297 characters; all 50 with line breaks and an escape character; case set up and answered in 1.5 s. dashboard_overview.rs:1558.
   - Reaches it: `dispatch report-done --evidence` and `report-failed --reason` take any text (`crates/conductor/src/dispatch.rs:370`, `:407`); the mailbox path gives one line only. So it needs a controller that passes a log as its evidence: possible, not shown to happen.
   - The bound in the case (one line, 1200 characters) is my reading, not the story's. The Sessions page excerpts the same `reason` to 120 characters (`sessions.rs:459`); the `dispatch` row of `/events` is also unbounded when a line has no goal (pre-existing, `overview.rs:397`).
   - No redaction applies to dispatch log text anywhere in the dashboard, so an evidence line holding a secret now reaches `/events` where base showed the verb. Only the home directory is rewritten (`tilde_all`, `dashboard.rs:778`).

3. **Unpinned lines.** Not run against a mutated build (one build lock, no copy built); read from the code.
   - `sessions.rs:83`: drop `!left.held` from the sort key and the unit's case stays green, because its restarted controller `epsilon` is live and either record shows it. My sessions case fails on that mutant whenever the store lists a stopped record first.
   - `overview.rs:407`: reorder the three keys, or drop `.trim()`, and the unit's case stays green: no line carries two keys or padded text. My precedence case pins both.

4. **Repository page.** `repo.rs:301-305` is unchanged by the unit and filters `ControllerState::Running`. A controller recorded stopped, live, and working outside its checkout is counted on the Sessions page and is not the repository's controller session for the `usage-limit` and `silent` blockers. No case written. The scoper named it; the acceptance does not ask for it.

## 5. Attacked and not broken

- Two records of one session name: the Running or Paused one wins regardless of store order (green case above).
- A stopped record with no session, or an exited one: no row, not counted.
- A Running record whose session is gone or exited: still a `not running` / `exited` row, not counted, as before.
- The same controller counted twice: one row per session name; `pick` joins one list row per record.
- Overview against Sessions page counts: the app derives both from `data.sessions` (`dashboard/src/api/overview.ts:358`); one source.
- A live row with no record: not listed, unchanged.
- A live session of the same name from another instance or repository: joined by name only, as for Running records before; a stopped record now joins it too. This is what the acceptance asks ("whatever the store recorded"); not a finding.
- HTML or script text: Vue interpolation only (`TodayTimeline.vue:27-28`, `TodayEvents.vue:20-21`), no `v-html`; the Rust side answers JSON.
- Blank or non-string `first_line`: falls through to `reason`, then `evidence`.
- Non-report rows: untouched by the diff. TypeScript fixtures and snapshots: static files, not read by the Rust change; Vitest not run.
- Log key names: `records.rs:320-327` writes `reason`, `evidence`, `first_line`, the keys `events()` reads.

## 6. Paths written outside the worktree

- ~/.cache/scratch/<session>/w49/u24/adv/sessions-cases.rs
- ~/.cache/scratch/<session>/w49/u24/adv/overview-cases.rs
- ~/.cache/scratch/<session>/w49/u24/adv/red.log (and `red.done`)
- ~/.cache/scratch/<session>/w49/u24/adv/suite.log (and `suite.done`)
- ~/.cache/scratch/<session>/w49/u24/adv-report-1.md

Also: the shared target directory through `conductor build run`, and a `worktree` session lease `adv-w49-u24-p1`, released.

## 7. Findings block

```findings
- file: crates/conductor/src/dashboard/overview.rs
  line: 410
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'a report the loop records with its verb as placeholder text (run/mailbox.rs:273, 296, 308) reads "done: done", "blocked: blocked", "failed: failed" in /events'
- file: crates/conductor/src/dashboard/overview.rs
  line: 411
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'a report row''s text is the whole reason or evidence, unbounded and unredacted, with line breaks and control characters (50 rows, 3.3 MB measured), where the Sessions page excerpts the same field to 120 characters'
- file: crates/conductor/tests/dashboard_sessions.rs
  line: 425
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the unit''s cases stay green when roster() drops the held-first sort key (sessions.rs:83) and when events() reorders first_line, reason, evidence or drops the trim (overview.rs:407); two added green cases pin them'
- file: crates/conductor/src/dashboard/repo.rs
  line: 303
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 'the repository page keeps Running controller records only, so a controller recorded stopped and live is counted on the Sessions page and is not that repository''s controller session for its blockers'
```
