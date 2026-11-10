---
format: aep.planning-md/3
id: review-result:w49-u10-adversary-1
kind: review-result
status: active
title: Wave 49 U10 dashboard-ui-loop, adversary pass 1
relations:
- reviews: story:dashboard-ui-loop
revision: 1
---
```
unit: U10 story:dashboard-ui-loop Round 1, worktree w49-u10 at edda5d6d0 (base 3a27c6469)
verdict: NEEDS-CHANGE
cases: executed 249→257, red 8
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths, all under the assigned scratch (part 6)
needs-coordinator: whether finding 2 holds the unit (read from check.yml, not run on a runner); I took no worktree session lease
```

## 1. Diff

`git --no-pager diff --stat` is empty, because the only change is one untracked test file:

```
?? crates/conductor/dashboard/tests/ux-adversary.spec.ts
```

No implementation file was touched; mutants ran on scratch copies only.

## 2. Cases added

`crates/conductor/dashboard/tests/ux-adversary.spec.ts` holds 8 cases, all red now. They were written first and run alone before anything else (`npx vitest run tests/ux-adversary.spec.ts`, exit 1):

```
FAIL … the ratchet, a value already held under a decision > does not rise in an update that names no decision
AssertionError: expected [] to deeply equal [ Array(1) ]
FAIL … a closed S1 without a regression asset > fails when its status reads "Closed"
FAIL … fails when its status reads "closed in R-02"
FAIL … fails when its status reads "closed (R-02)"
FAIL … fails when its status reads "fixed"
FAIL … fails when its status reads "done"
AssertionError: expected [] to not deeply equal []
FAIL … fails when the asset it names is a directory, not an assertion
AssertionError: .: expected [] to not deeply equal []
FAIL … the CI task check job > gives the gate the baseline of main to compare with
AssertionError: expected 'name: Check…' to match /fetch-depth: 0|git fetch [^\n]*origin…/
 Test Files  1 failed (1)
      Tests  8 failed (8)
```

- **Ratchet case:** `writeBaseline` takes 6 → 60 with no decision and keeps `R-01`; `worseThanMain` then reports nothing.
- **Five status cases:** a closed S1 with no asset passes the checker when its status is anything but the exact string `closed`.
- **Directory case:** a closed S1 whose `regression_asset` is `.`, `ux` or `ux/findings` passes.
- **CI case:** `check.yml` never fetches `origin/main` before `task check`.

## 3. Suite

`npm test` in `crates/conductor/dashboard` on Node 22.23.2, exit 1:

```
 Test Files  1 failed | 34 passed (35)
      Tests  8 failed | 249 passed (257)
```

With my file excluded (`npx vitest run --exclude tests/ux-adversary.spec.ts`): 34 files, 249 passed.

## 4. Findings (tree edda5d6d0, all `introduced`)

| # | file:line | verdict | severity | finding |
|---|---|---|---|---|
| 1 | `ux/ratchet.ts:115`, `ux/gate.ts:38` | NEEDS-CHANGE | blocker | `--update` without `--decision` lets a hard-gate value rise once the key carries any decision |
| 2 | `.github/workflows/check.yml:33`, `ux/gate.ts:53` | NEEDS-CHANGE | warning | the worse-than-main check is skipped on pull requests |
| 3 | `ux/findings.ts:58` | NEEDS-CHANGE | warning | only the exact status `closed` is checked |
| 4 | `ux/findings.ts:62` | INFEASIBLE | note | a directory counts as a regression asset |
| 5 | `ux/measure.ts:356` | CONFIRMED | warning | the gate passes with the Health and KPIs pages blank |
| 6 | `ux/contract.ts:50`, `ux/measure.ts:372` | CONFIRMED | warning | the gate passes with the dark theme dead |

1. **Measured:** the red ratchet case, plus an end-to-end run on a scratch copy. A CSS mutant took repository `axe_serious_critical_nodes` from 4 to 152 and `axe_rule_cases` from 2 to 6.
   - `node ux/gate.ts` failed with "worse", as it should.
   - `node ux/gate.ts --update` with no `--decision` then exited 0 and wrote 152 and 6 under the old `"decision": "R-01"`; the next gate run was green.
   - **Reaches it:** `task ui:ux-gate -- --update`, the documented command, on any of the ten keys held under R-01. Both guards skip a key whose baseline entry has a decision.
   - **Fix:** do not carry the old decision onto a higher value, or require `--decision` for any rise.

2. **Measured:** the red CI case. `gate.ts` skips the check with a note when `origin/main` is absent, and the checkout step sets no `fetch-depth` and fetches no main.
   - **Reaches it:** every pull-request run; on a push to main the two baselines are equal.
   - This is the only guard for the non-hard keys (`truncated_states`, `reduced_motion_violations`, `keyboard.tabs`, `screen_only.actions`), which `--update` raises without a decision.
   - Read from the workflow, not observed on a runner.

3. **Measured:** the five red status cases. Severity is held to a vocabulary; status is not.
   - **Reaches it:** the committed records already write prose in header fields (`regression_asset: none yet; proposed: …`), so `status: closed in R-02` is in the house style.

4. **Measured:** the red directory case.
   - **Reaches it:** nothing found; I constructed it.

5. **Measured:** on scratch copies, `.health>*{display:none!important}` appended to the built CSS gave `ux gate ok: 112 values at the baseline`, exit 0. The same held with `.kpis>*` added.
   - Every value is a count of defects, and these scenarios sit at 0 with no completion state, so a page that shows nothing in any state measures as clean.
   - **Reaches it:** any CSS or render regression that empties a zero-baseline page. Only Decisions is protected, by `completion_miss`.

6. **Measured:** on a scratch copy with the dark selectors renamed so they never match, the gate printed `ux gate ok`, exit 0.
   - `theme_not_applied` compares only the `data-theme` attribute, and its break-test only removes that attribute.
   - **Reaches it:** a CSS regression in the dark palette block.

## 5. Attacked and not broken

- **Stability:** three clean gate runs each gave exit 0 and 112 values.
- **Network:** every non-loopback request is aborted and counted, and the served CSP is `default-src 'none'`.
- **Break-tests:** each one fails on its own break, and the clean control page stops an always-true check.
- **Worse values:** the plain gate does fail on a worse value (first M3 run, exit 1).
- **CI install:** `npm ci` and `playwright install --with-deps chromium` come after setup-node and before `task check`.
- **Shot bytes:** nothing compares them; shots are referenced by file name only, so the re-encoded PNG is inert.
- **R-01 documents:** `round.json` matches `baseline.json` on all 112 keys with 0 differences, all 14 shot files exist, and every finding's evidence screenshot exists.

## 6. Paths written outside the worktree

- `<scratch>/probe.sh`
- `<scratch>/probe.log`
- `<scratch>/red-alone.log` and `suite.log` in the same directory
- `<scratch>/copy/` (19 MB: five app copies `clean`, `m1`–`m4`, each with a `node_modules` symlink into the worktree)

```findings
- file: crates/conductor/dashboard/ux/ratchet.ts
  line: 115
  category: mutant
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a key that already carries a decision id rises through --update without --decision (4 to 152 axe nodes written under the old R-01, gate green after), because gate.ts:38 and worseThanMain both skip keys that hold any decision
- file: .github/workflows/check.yml
  line: 33
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the checkout fetches no origin/main, so gate.ts:53 skips the worse-than-main check with a note on every pull request, and a committed baseline with a raised non-hard value merges without a decision
- file: crates/conductor/dashboard/ux/findings.ts
  line: 58
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the findings checker matches only the exact status "closed", so a closed S1 or S2 written as Closed, "closed in R-02", fixed or done passes with no regression asset
- file: crates/conductor/dashboard/ux/findings.ts
  line: 62
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a directory such as "." or "ux" satisfies the regression asset check for a closed S1; constructed, no writer of such a record found
- file: crates/conductor/dashboard/ux/measure.ts
  line: 356
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: with the Health and KPIs pages rendered blank in every state the gate reports ok at 112 values, because every gated value is a defect count and zero-baseline scenarios have no presence check
- file: crates/conductor/dashboard/ux/contract.ts
  line: 50
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: theme_not_applied and its break-test read only the data-theme attribute, so the gate reports ok with the dark palette rules dead in the built CSS
```
