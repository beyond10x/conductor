---
format: aep.planning-md/3
id: review-result:w49-u10-adversary-2
kind: review-result
status: active
title: Wave 49 U10 dashboard-ui-loop, adversary pass 2
relations:
- reviews: story:dashboard-ui-loop
revision: 1
---
```
unit: U10 story:dashboard-ui-loop Round 1, pass 2, worktree w49-u10 at 8dfdb59f7 (base 3a27c6469)
verdict: CONFIRMED
cases: executed 269→279, red 9
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 13 paths, all under the assigned scratch (part 6)
needs-coordinator: whether finding 1 holds the unit (gate green with three pages invisible); the CI fetch was read, not run on a runner; I took no worktree session lease
```

## 1. Diff

`git --no-pager diff --stat` is empty; the only change is one untracked test file:

```
?? crates/conductor/dashboard/tests/ux-adversary2.spec.ts
```

No implementation file was touched; mutants ran on scratch copies only.

## 2. Cases added

`crates/conductor/dashboard/tests/ux-adversary2.spec.ts` holds 10 cases: 9 red, 1 green control. They were written first and run alone (`npx vitest run tests/ux-adversary2.spec.ts`, exit 1):

```
FAIL … the decision rule, second pass > takes no empty decision id for a hard-gate value that rose
AssertionError: expected [] to not deeply equal []
FAIL … never writes a risen value under the id the lower value was held under
AssertionError: expected 'R-01' not to be 'R-01'
FAIL … does not let a tolerance raised in the baseline hold a worse value than main accepted
AssertionError: expected [] to not deeply equal []
FAIL … the findings checker, second pass > reads a closed S1 record whatever its file is called in ux/findings/
AssertionError: UXF-005-header.md: expected false to be true
FAIL … takes no file that asserts nothing for a regression asset
AssertionError: ux/findings/UXF-001.md: expected [] to not deeply equal []
FAIL … the contract in a browser, second pass > misses a page whose main is transparent
FAIL … misses rows whose every cell is hidden
FAIL … misses rows that hold no text
FAIL … misses the dark palette dead for the page that follows the system
AssertionError: expected [] to not deeply equal []
 Test Files  1 failed (1)
      Tests  10 failed | 1 passed (11)
```

That first run had an eleventh case (a dark palette with only `--bg` left). I removed it after the probe showed axe-core catches that mutant in the app; the file re-run alone gives 9 failed, 1 passed.

## 3. Suite

`npx vitest run` in `crates/conductor/dashboard` on Node 22.23.2, exit 1:

```
 Test Files  1 failed | 35 passed (36)
      Tests  9 failed | 270 passed (279)
```

With my file excluded (`--exclude tests/ux-adversary2.spec.ts`): 35 files, 269 passed, exit 0. The 8 pass 1 cases in `tests/ux-adversary.spec.ts` are green now.

## 4. Findings (tree 8dfdb59f7, all `introduced`)

| # | file:line | verdict | severity | finding |
|---|---|---|---|---|
| 1 | `ux/contract.ts:64` | CONFIRMED | warning | the gate passes with the Health, KPIs and Decisions pages invisible |
| 2 | `ux/measure.ts:76` | CONFIRMED | warning | the gate passes with the dark palette dead for the default (system) theme |
| 3 | `ux/gate.ts:37` | INFEASIBLE | warning | `--update --decision ''` writes a risen hard-gate value under an empty id |
| 4 | `ux/ratchet.ts:174` | CONFIRMED | note | `--update --decision R-01` writes a risen value under the old id |
| 5 | `ux/ratchet.ts:48` | INFEASIBLE | note | a tolerance raised by hand in `baseline.json` holds any worse value |
| 6 | `ux/findings.ts:95` | INFEASIBLE | note | a record not named `UXF-nnn.md` is never checked |
| 7 | `ux/findings.ts:82` | INFEASIBLE | note | any file counts as a regression asset |

1. **Measured:** three red browser cases, plus gate runs on scratch copies.
   - `.health,.kpis,.decisions{opacity:0!important}` appended to the built CSS gave `ux gate ok: 122 values at the baseline`, exit 0. A separate read confirmed the three page roots compute opacity 0.
   - Every table cell and list item of Health and KPIs hidden: also `ux gate ok`, exit 0.
   - `main{opacity:0}` on every page failed only with "improvement not taken" on overview (its held axe nodes vanished). The `--update` that message names then exited 0 with no decision, and the next gate run was green.
   - **Cause:** `shown()` reads only the named element's own box, display, visibility and opacity; nothing reads an ancestor's opacity, a child's visibility or the text.
   - **Reaches it:** a wrapper left at opacity 0, or a render that draws rows with empty cells.
   - **Fix:** `checkVisibility` with the opacity and visibility options, plus a non-empty text test per `present` entry and a break-test for each.

2. **Measured:** the red theme case, plus the gate on a copy with `prefers-color-scheme:dark` renamed: `ux gate ok`, exit 0.
   - **Cause:** the gate opens only pinned pages, each under the other system scheme, so the `@media` palette block is never on screen.
   - **Reaches it:** `system` is the app's default theme (`src/App.vue:105`), so every operator on a dark system who has not pinned one.
   - **Fix:** measure one unpinned page per system scheme.

3. **Measured:** the red case, plus an end-to-end run. A contrast mutant took repository `axe_serious_critical_nodes` from 4 to 152; `--update --decision ''` exited 0 and wrote `"decision": ""`; the next gate was green.
   - **Cause:** `gate.ts:37`, `undecided` and `worseThanMain` all test only `=== undefined`.
   - **Reaches it:** nothing found in the repository; an unset shell variable would.

4. **Measured:** the red case; `writeBaseline`'s own comment says "never the old id".
   - `worseThanMain` catches it once main holds a baseline; today `origin/main` has none, so nothing does.
   - **Reaches it:** the documented command with the id the operator already has.

5. **Measured:** the red case, plus the gate on a copy with every `"tolerance": 0` set to 1000 and the same contrast mutant: `ux gate ok`, exit 0.
   - **Reaches it:** nothing found; a hand edit of a committed file.

6. **Measured:** the red case: closed S1 records named `UXF-005-header.md`, `UXF-1000.md` and `uxf-006.md` produce no problem.
   - **Reaches it:** nothing found; the four committed records follow the pattern.

7. **Measured:** the red case: the record itself, a screenshot and `package.json` each pass for a closed S1.
   - **Reaches it:** nothing found; I constructed it.

### The six pass 1 findings

| pass 1 | fix holds? |
|---|---|
| 1 `--update` without `--decision` raises a held value | yes; residue is findings 3 and 4 |
| 2 no `origin/main` in CI | yes as read: the fetch step precedes `task check`, and the CI-set failure is unit-tested. Not run on a runner; with `persist-credentials: false` the fetch is anonymous, which works only while the repository is public |
| 3 status vocabulary | yes |
| 4 directory as asset | yes; residue is finding 7 |
| 5 blank Health and KPIs pages | only for `display:none` and a missing element; finding 1 |
| 6 dark theme dead | yes for the pinned palette, by its break-tests (not re-run on the app); finding 2 is the system palette |

## 5. Attacked and not broken

- **Stability:** three clean gate runs each gave exit 0 and 122 values.
- **Background-only dark palette:** the pinned dark rule cut down to `--bg` fails the gate through axe-core (overview 2 to 76 nodes, repository 4 to 57).
- **Plain worse value:** the gate fails on it (repository 4 to 152, exit 1).
- **Shot bytes:** nothing in `ux/*.ts` or `tests/ux-*.spec.ts` hashes or reads a PNG for comparison; the re-encoded shot is inert.
- **R-01 documents:** `round.json` matches `baseline.json` on all 122 keys, and every finding's evidence screenshot exists.
- **Other workflows:** only `check.yml` runs `task check`, so the CI-set failure hits no job without the fetch.

## 6. Paths written outside the worktree

All under `<scratch>/`:

- `red.sh`, `probe.sh`, `probe2.sh`, `t2.mjs`, `roundcheck.mjs`, `bites.ts`
- `red-alone.log`, `red-alone-2.log`, `suite.log`, `suite-excl.log`, `probe.log`, `probe2.log`
- `copy/` (39 MB: ten app copies `clean`, `p1`, `p1b`, `p1c`, `p2`, `p2b`, `t1`, `t2`, `d1`, `d2`, each with a `node_modules` symlink into the worktree; `copy/p1b/bites.ts` is a copy of the script)

```findings
- file: crates/conductor/dashboard/ux/contract.ts
  line: 64
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the presence check reads only each named element's own box and style, so the gate reports ok at 122 values with the Health, KPIs and Decisions page roots at opacity 0, and with every table cell and list item of Health and KPIs hidden
- file: crates/conductor/dashboard/ux/measure.ts
  line: 76
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the gate opens only pinned themes under the opposite system scheme, so it reports ok with the prefers-color-scheme dark palette block dead, which is the palette of the app's default system theme
- file: crates/conductor/dashboard/ux/gate.ts
  line: 37
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: --update --decision '' writes a risen hard-gate value (4 to 152 axe nodes) under an empty decision id that gate.ts, undecided and worseThanMain all accept; constructed, no caller passing an empty id found
- file: crates/conductor/dashboard/ux/ratchet.ts
  line: 174
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: writeBaseline writes a risen value under the old id when --decision repeats it, against its own comment and AGENTS.md, and only worseThanMain catches it once main holds a baseline
- file: crates/conductor/dashboard/ux/ratchet.ts
  line: 48
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: compare trusts the tolerance stored in baseline.json and worseThanMain compares values only, so a tolerance raised by hand holds any worse measurement with no decision; constructed, a hand edit of a committed file
- file: crates/conductor/dashboard/ux/findings.ts
  line: 95
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: readFindings keeps only names matching UXF-nnn.md, so a closed S1 record in UXF-005-header.md, UXF-1000.md or uxf-006.md is never checked; constructed, the committed records follow the pattern
- file: crates/conductor/dashboard/ux/findings.ts
  line: 82
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: any existing file satisfies the regression asset of a closed S1, including the record itself, a screenshot and package.json; constructed, no writer of such a record found
```
