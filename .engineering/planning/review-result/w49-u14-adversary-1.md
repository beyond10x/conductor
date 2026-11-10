---
format: aep.planning-md/3
id: review-result:w49-u14-adversary-1
kind: review-result
status: active
title: Wave 49 U14 decision-options-spellings, adversary pass 1
relations:
- reviews: story:decision-options-spellings
revision: 3
---
unit: U14 story:decision-options-spellings, commit 8b561a64f on impl/decision-options-spellings (base 3a27c6469), plus my uncommitted test cases
verdict: NEEDS-CHANGE
cases: executed 62→67, red 4
origin: introduced 5, pre-existing 1, undecided 0
wrote-outside-worktree: 11 paths, all under the assigned scratch
needs-coordinator: whether F2 (label keeps `(cost: )`) and F5 (dash-cost false positives) hold the unit; whether F3 becomes a backlog draft

`executed 62` is the suite run below minus my 5 cases (decide_model + run binaries only, as the unit file directs). No suite run was made before my cases existed.

## 1. git --no-pager diff --stat (worktree)

```
 crates/conductor/tests/decide_model.rs | 93 ++++++++++++++++++++++++++++++++++
 crates/conductor/tests/run.rs          | 41 +++++++++++++++
 2 files changed, 134 insertions(+)
```

Test files only. No implementation file touched. Nothing committed.

## 2. Cases added

| Case | File:line | Asserts | Now |
|---|---|---|---|
| `adv_a_cost_that_ends_in_its_own_parenthesis_keeps_it` | tests/decide_model.rs:600 | `A. … Cost: about 2 h (one migration).` and `B) … — cost nothing (for now)` keep the cost's own `)` | red |
| `adv_an_option_with_an_empty_cost_keeps_its_label_without_the_marker` | tests/decide_model.rs:621 | `A: x (cost: )` and `B: y; cost:` give labels `x`, `y`, as on the base | red |
| `adv_the_spelling_a_refusal_teaches_is_read_with_its_cost` | tests/decide_model.rs:643 | `A) <label>: <cost>`, the form a refusal names, is read with its cost by `parse_request` | red |
| `adv_a_dash_before_a_longer_word_names_no_cost` | tests/decide_model.rs:664 | `— costs nothing`, `- cost-free` name no cost | green (mutant guard) |
| `adv_a_mailbox_option_keeps_a_label_ending_in_cost_and_a_cost_in_parentheses` | tests/run.rs:2065 | no-seam mailbox: `A) accept the higher cost: 200 a month` keeps its label; a cost's own `)` is kept | red |

Red output, the five cases alone (`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test decide_model --test run --no-fail-fast -E 'test(/^adv_/)'`, exit 100), assertion lines verbatim:

```
    Starting 5 tests across 2 binaries (62 tests skipped)
        PASS [   0.196s] (1/5) conductor-cli::decide_model adv_a_dash_before_a_longer_word_names_no_cost

    thread 'adv_the_spelling_a_refusal_teaches_is_read_with_its_cost' (1340154) panicked at crates/conductor/tests/decide_model.rs:652:5:
    assertion `left == right` failed
      left: [("A", "Tree layout: one migration", "not stated in the request"), ("B", "accept the higher", "200 a month")]
     right: [("A", "Tree layout", "one migration"), ("B", "accept the higher cost", "200 a month")]

    thread 'adv_an_option_with_an_empty_cost_keeps_its_label_without_the_marker' (1340157) panicked at crates/conductor/tests/decide_model.rs:630:5:
    assertion `left == right` failed
      left: [("A", "the first mirror (cost: )", "not stated in the request"), ("B", "the default mirror; cost:", "not stated in the request")]
     right: [("A", "the first mirror", "not stated in the request"), ("B", "the default mirror", "not stated in the request")]

    thread 'adv_a_cost_that_ends_in_its_own_parenthesis_keeps_it' (1340155) panicked at crates/conductor/tests/decide_model.rs:609:5:
    assertion `left == right` failed
      left: [("A", "The first mirror", "about 2 h (one migration"), ("B", "the default mirror", "nothing (for now")]
     right: [("A", "The first mirror", "about 2 h (one migration)"), ("B", "the default mirror", "nothing (for now)")]

    thread 'adv_a_mailbox_option_keeps_a_label_ending_in_cost_and_a_cost_in_parentheses' (1340158) panicked at crates/conductor/tests/run.rs:2093:5:
    assertion `left == right` failed
      left: [("A", "accept the higher", "200 a month"), ("B", "Tree layout", "about 2 h (one migration")]
     right: [("A", "accept the higher cost", "200 a month"), ("B", "Tree layout", "about 2 h (one migration)")]

     Summary [   2.062s] 5 tests run: 1 passed, 4 failed, 62 skipped
```

Full log: ~/.cache/scratch/<session>/w49/u14/adv/red.log

## 3. Suite run (after part 2)

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test decide_model --test run --no-fail-fast`, exit 100:

```
    Starting 67 tests across 2 binaries
     Summary [   4.412s] 67 tests run: 63 passed, 4 failed, 0 skipped
        FAIL [   0.052s] ( 5/67) conductor-cli::decide_model adv_an_option_with_an_empty_cost_keeps_its_label_without_the_marker
        FAIL [   0.037s] ( 6/67) conductor-cli::decide_model adv_the_spelling_a_refusal_teaches_is_read_with_its_cost
        FAIL [   0.156s] (13/67) conductor-cli::decide_model adv_a_cost_that_ends_in_its_own_parenthesis_keeps_it
        FAIL [   0.883s] (49/67) conductor-cli::run adv_a_mailbox_option_keeps_a_label_ending_in_cost_and_a_cost_in_parentheses
error: test run failed
```

Only my four cases are red; the unit's three cases and the 59 others pass. Full log: ~/.cache/scratch/<session>/w49/u14/adv/suite.log

## 4. Findings (tree: 8b561a64f)

Origins were decided by running a verbatim copy of the base and unit parser functions side by side on 59 lines (scratch `probe/`), not by building the base.

| # | file:line | Measured | What reaches it | Verdict | Origin |
|---|---|---|---|---|---|
| F1 | crates/conductor/src/decide/handle.rs:910 | A cost that ends in its own parenthesis loses the `)`: `Cost: about 2 h (one migration).` gives `about 2 h (one migration`; `— cost nothing (for now)` gives `nothing (for now`. `trim_end_matches([')', '.'])` strips a `)` in both new spellings, where no `(` opened the cost. | The real loop: `run.rs:270` sets the decider seam, `decide::handle_request` calls `parse_request`; the cost goes into the decider prompt and the escalated options. Base did not read either spelling, so the truncated cost is new. (`(cost: 2 h (one-off))` was already truncated on the base.) Fix: strip `)` only when the marker was preceded by `(`. | NEEDS-CHANGE | introduced |
| F2 | crates/conductor/src/decide/handle.rs:912 | An empty cost now leaves the marker in the label: `A: do it (cost: )` gave label `do it` on the base, now `do it (cost: )`; `A: do it; cost:` gave `do it`, now `do it; cost:`. `stated_cost` returns `None` and `label_and_cost` falls back to the whole text. | Same real-loop path. A lead that leaves a cost blank. | CONFIRMED | introduced |
| F3 | crates/conductor/src/run/mailbox.rs:138 | The form a refusal teaches the sender, `A) <label>: <cost>`, is read by `parse_request` with no cost (`Tree layout: one migration` / not stated), and a label ending in `cost` is cut (`accept the higher`). The two readers disagree on this line. | `answer_refused` (mailbox.rs:150) sends this form on every refused `[DECISION-REQUEST]` in the real loop. Identical on the base. | CONFIRMED | pre-existing |
| F4 | crates/conductor/src/run/mailbox.rs:366 | The no-seam reader changed for a line it read before: `A) accept the higher cost: 200 a month` was `accept the higher cost` / `200 a month`, now `accept the higher` / `200 a month`; costs with their own `)` or a final `.)` are now cut as in F1. | Nothing found outside tests: `handle_request: None` is built only by tests/run.rs:153; `run.rs:270` always sets the seam. | INFEASIBLE | introduced |
| F5 | crates/conductor/src/decide/handle.rs:897 | A dash before the word `cost` used as an ordinary word is read as a cost: `A) rename the report — cost centre stays` gives label `rename the report`, cost `centre stays`; `A) use the low - cost tier` gives `use the low` / `tier`; `A) accept — cost of delay: 2 days` gives `accept` / `of delay: 2 days`. Base kept the whole label. Probe only, no test: the acceptance asks for `— cost <cost>`, so the expected value is a design call. | Real loop. I did not show a request that writes these. | INFEASIBLE | introduced |
| F6 | crates/conductor/src/decide/handle.rs:897 | Mutant the unit's suite misses: dropping `&& text[end..].starts_with(char::is_whitespace)` leaves all three unit cases' expected values unchanged (checked on a scratch copy of the functions, not by building a mutated crate); `— costs nothing` then reads cost `s nothing`. `adv_a_dash_before_a_longer_word_names_no_cost` pins it and is green on the unit. | Real loop. | CONFIRMED | introduced |

Residue from the focus list, measured in the probe and not raised as rows:
- `A. Tree layout. **Cost:** one migration` reads label `Tree layout. **`, cost `** one migration` in both readers (base decide: no option; base mailbox: whole label, no cost).
- `I. e. …`, `A. Smith said …`, `N. B. …` are now options in `parse_request` (the mailbox reader already took them). A prose line `A. …` after an `A:` option now refuses the request with "lists the option A twice". Follows directly from the acceptance; I did not show a request that does it.
- `A. cost: 2 h` and `A) — cost 2 h` give the whole text as label plus the cost.

## 5. Attacked, not broken

- `low-cost path - cost 2 h`, `A. Cost centre move`, `A) the cost: none` (same as base in decide), `A. No cost named.`: read as intended.
- Non-ASCII before the dash (`déjà vu café — cost 2 h`, `naïve – cost 2 h`, `İİİ`): no panic, byte indices hold.
- Cost ending in a version (`Cost: move to 1.0.`, `Use v1.2. Cost: 1.`): label and cost correct.
- `—cost`, `label—cost`, double space and tab around the dash: read; `--` and U+2015 are not, consistently in both readers.
- Every `cost:` line the base decide reader parsed gives the same label and cost, except F2 and `Tree layout: cost: x` (label loses a trailing `:`, an improvement).
- Mailbox and decide readers agree on label and cost for every line that names a cost; they differ only on `: ` and ` - ` splits without the word (F3).

## 6. Paths written outside the worktree

All under ~/.cache/scratch/<session>/w49/u14/:
- adv/run.sh
- adv/red.log, adv/red.log.raw, adv/red.log.done
- adv/suite.log, adv/suite.log.raw, adv/suite.log.done
- adv/probe/probe.rs, adv/probe/probe_mut.rs, adv/probe/probe, adv/probe/probe_mut, adv/probe/corpus.txt, adv/probe/mut.txt
- adv-report-1.md

Worktree lease `adv-w49-u14-p1` released.

```findings
- file: crates/conductor/src/decide/handle.rs
  line: 910
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'a cost that ends in its own parenthesis loses the closing `)` in both spellings the story adds, because `)` is stripped whether or not `(cost:` opened it'
- file: crates/conductor/src/decide/handle.rs
  line: 912
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'an option with an empty cost (`(cost: )`, `; cost:`) now keeps the marker in its label where the base dropped it'
- file: crates/conductor/src/run/mailbox.rs
  line: 138
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: 'the form a refusal teaches, `A) <label>: <cost>`, is read by parse_request with no cost and cuts a label ending in `cost`'
- file: crates/conductor/src/run/mailbox.rs
  line: 366
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'the no-seam reader now cuts `accept the higher cost: 200 a month` to label `accept the higher` and truncates parenthesised costs, on a path only tests build'
- file: crates/conductor/src/decide/handle.rs
  line: 897
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'a dash before `cost` used as an ordinary word (`— cost centre stays`, `low - cost tier`) is read as the option''s cost and cut from the label'
- file: crates/conductor/src/decide/handle.rs
  line: 897
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the unit''s cases stay green when the whitespace check after the word `cost` is dropped; the added green case pins it'
```
