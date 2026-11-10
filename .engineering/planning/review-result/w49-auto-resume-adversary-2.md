---
format: aep.planning-md/3
id: review-result:w49-auto-resume-adversary-2
kind: review-result
status: active
title: Wave 49 U1 adversary, pass 2
relations:
- reviews: story:auto-resume
revision: 1
---
unit: story:auto-resume, correction commit cf1ecd643 on impl/auto-resume plus four uncommitted adversary2 cases in w49-u1
verdict: NEEDS-CHANGE
cases: executed 1638→1642, red 2
origin: introduced 4 / pre-existing 0 / undecided 1
wrote-outside-worktree: 3 logs in the unit's scratch adv2/ directory
needs-coordinator: none

Cases (run alone: 4 run, 2 passed, 2 failed):
- watch.rs adversary2_a_banner_quoted_on_a_tool_outputs_second_line_is_not_resumed (red):
  an idle pane whose Bash output quotes the banner on its second line (no ⎿) got
  "usage limit resumed: 1 panes: ex-super".
- conductor-docs generate.rs adversary2_the_config_page_does_not_call_usage_probe_required (red):
  missing | `usage.probe` | string | none |.
- watch.rs adversary2_a_working_pane_with_a_banner_shaped_line_is_not_resumed (green guard).
- watch.rs adversary2_the_bullet_banner_and_the_eighth_last_line_are_read (green guard).

Suite: 1642 run, 1639 passed, 3 failed (the two red cases and build_run, which fails on the base
on this host). fmt exit 0.

F1 watch.rs:590: trim_start_matches(whitespace | '│') strips a tool output's continuation indent.
Reached by supervisor.md:172's `capture-pane -p | tail` and by a session debugging this feature.
F2 website/docs/reference/config.md:210: the generator (conductor-docs/src/config.rs:593) drops
probe from a sample that sets rotate, the new refusal fails it, and the key reads `required`.
F3 supervisor.md:181 and :189 still say to rotate and resume a usage-limit stop by hand.
F4 watch.rs:275, :268: the installed harness ~/.local/share/claude/versions/2.1.321 has
`esc to interrupt` only in a low-priority wait line and send-now notices, and holds
`Usage limit reached · wrapping up` (still working), `Usage limit reached` alone (stopped), and
the phrase with no ` · …` when no reset time is known; behind feature flags, no live pane seen.
F5 tests/watch.rs:4176-4199: three mutants survived by reading (no WORKING check, no ` ∙ `,
BANNER_LINES 9); guard cases 3 and 4 cover them.

Could not break: the unread capture state, the usage.rotate refusal (null as absent, empty
probe refused), rotated()'s headroom against the spec, the border trim, the 8-line window.

```findings
- file: crates/conductor/src/watch.rs
  line: 590
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: stopped() strips all leading whitespace, so the banner text on a tool output's continuation line (no ⎿) is read as the harness's banner, and the quoting pane gets Escape, the resume message and a host-wide rotation
- file: website/docs/reference/config.md
  line: 210
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the generated config reference now calls usage.probe required, because the generator drops probe from a sample that sets rotate and the new refusal fails it, while a config without a probe stays valid
- file: crates/conductor/roles/supervisor.md
  line: 189
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: lines 181 and 189 still tell the supervisor to rotate and resume a usage-limit stop by hand whether or not usage.probe is set, so it types into panes the watch already resumed
- file: crates/conductor/src/watch.rs
  line: 275
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: undecided
  message: the installed harness binary has esc to interrupt only in a low-priority wait line and send-now notices, and draws Usage limit reached alone when stopped and with a wrapping-up suffix while working, so the guard may not fire and the plain banner is never matched; not seen in a live pane
- file: crates/conductor/tests/watch.rs
  line: 4176
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the correction's tests let three mutants survive (dropping the esc to interrupt check, dropping the bullet separator, BANNER_LINES 8 to 9); guard cases adversary2_a_working_pane_with_a_banner_shaped_line_is_not_resumed and adversary2_the_bullet_banner_and_the_eighth_last_line_are_read added
```
