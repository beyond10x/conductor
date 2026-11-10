---
format: aep.planning-md/3
id: review-result:w49-auto-resume-adversary-1
kind: review-result
status: active
title: Wave 49 U1 adversary, pass 1
relations:
- reviews: story:auto-resume
revision: 1
---
unit: story:auto-resume, commit 54b1c29a9 on impl/auto-resume plus three uncommitted cases in w49-u1
verdict: NEEDS-CHANGE
cases: executed 1610→1613, red 2
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 logs in the unit's scratch adv/ directory
needs-coordinator: none

Cases added in crates/conductor/tests/watch.rs:
- adversary_a_working_pane_that_quotes_the_banner_text_is_not_resumed (red): a working pane
  whose tool output quotes `Usage limit reached` got "usage limit resumed: 1 panes: ex-dev".
- adversary_a_pane_unread_for_one_pass_is_not_resumed_twice_for_one_stop (red): resumed, then a
  failed capture-pane, then the banner again gave a second resume instead of `not lifted`.
- adversary_escape_and_enter_are_pasted_as_keys_and_the_message_as_a_paste (green): guards the
  paste-buffer -p flags [false, true, false]; the unit's typed() helper ignores -p.

Suite with the cases: 1613 run, 1610 passed, 3 failed (the two red cases and build_run, which
fails on the base too). Without them: 1610 run, 1609 passed, 1 failed (build_run).

F1 watch.rs:501: screen.contains(BANNER) over the whole pane; a controller working on this
repository shows the phrase; Escape cancels its turn and a rotation runs.
F2 watch.rs:1437-1441: a failed capture counts as off the banner and resets once-per-stop.
F3 tests/watch.rs:4007: typed() ignores -p; case 3 covers it.
F4 roles/supervisor.md:173: says the watch resumes only under usage.rotate; the code resumes
whenever usage.probe is set, and does nothing for rotate without probe; spec/domains/config.yaml:517
omits that it still resumes.

Could not break: sessions_dir, the run loop's `usage limit: ` prefix, failing-source names,
headroom at 100 % and per account, config round trip and the empty-rotate refusal, wire.rs.

```findings
- file: crates/conductor/src/watch.rs
  line: 501
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the banner probe matches the phrase anywhere on the visible screen, so a working pane that only quotes it gets Escape and a resume, and a spurious rotation runs
- file: crates/conductor/src/watch.rs
  line: 1441
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a failed capture-pane is treated as off the banner and forgets the stop, so one unanswered pass leads to a second rotation and resume for the same stop
- file: crates/conductor/tests/watch.rs
  line: 4007
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the unit's typed() helper ignores the paste-buffer -p flag, so a bracketed Escape or Enter (no key press, no submit) would stay green; guard case added
- file: crates/conductor/roles/supervisor.md
  line: 173
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the role doc says the watch resumes only under usage.rotate, but it resumes whenever usage.probe is set, and does nothing for rotate without probe, so supervisor and watch both type into the same pane
```
