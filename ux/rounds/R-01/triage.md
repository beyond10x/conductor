# R-01 triage

## Caveats

- First round: no earlier change set, no earlier baseline. The baseline is written from this round's numbers (`ux/baseline.json`, hard-gate values above 0 carry decision id `R-01`).
- No people, no recordings, no questionnaire: sources that can agree are the metrics (L2, L3), the heuristic review and the report review. Confidence 2 where two of them name the same defect.
- Both review agents ran on the first measurement run. The report reviewer's tool defects were fixed afterwards and the round re-run: T1 (state double count), T3 (axe now counts nodes, report groups by rule), T5 (note when main has no baseline), T6 (theme applied is a metric), T7 (`maxActions` is read: `over_budget`), T8 (truth text must be visible), T9 (dirty tree listed), T10 (report text). Not fixed: T2 (the toggle check stays strict, the toggle in a closed drawer is UXF-002), T4 beyond the decision rule on `--update`, T11, T12. The numbers in `review-report.md` are therefore the old definitions; its findings are triaged below.
- "frequency" is pages affected of the 36 sitemap pages for page defects, scenarios affected of 3 for task defects.
- The baseline was measured with the harness files untracked (`tree_dirty` in `round.json`); the website sources were those of dad6b16b9 plus the added dependencies.

## Did the last change sets move their metrics?

| change set | target metric | moved? |
|---|---|---|
| none (first round) | not applicable | not applicable |

## Findings and priority

priority = severity weight x frequency x confidence / cost

| id | sev | frequency | conf | cost | priority | note |
|---|---|---|---|---|---|---|
| UXF-002 theme toggle hidden at 390 | S2 = 3 | 36/36 = 1.00 | 2 | S = 1 | 6.00 | metric + UXF-106 |
| UXF-003 links by colour alone | S2 = 3 | 29/36 = 0.81 | 2 | S = 1 | 4.86 | metric + UXF-102 |
| UXF-001 no search | S2 = 3 | 36/36 = 1.00 | 2 | M = 2 | 3.00 | needs a decision |
| UXF-005 landing CTA below the fold | S2 = 3 | 3/3 = 1.00 | 2 | M = 2 | 3.00 | bot + report review; anchor card; needs a decision |
| UXF-006 code text 13.1 px | S3 = 1 | 36/36 = 1.00 | 2 | S = 1 | 2.00 | metric + UXF-107 |
| UXF-104 source list pushes content down on a phone | S2 = 3 | 2/3 = 0.67 | 1 | M = 2 | 1.00 | heuristic only; layout change |
| UXF-004 unlabelled checkboxes | S2 = 3 | 1/36 = 0.03 | 2 | S = 1 | 0.17 | critical axe on one page |
| UXF-105 CLI table clipped at 390 | S3 = 1 | 1/36 = 0.03 | 1 | S = 1 | 0.03 | |

No S1 finding: every task completed at both gating viewports. `start-the-daemon@390x844` is over its 4-action budget (6 actions, `over_budget` = 1) and is held in the baseline under decision R-01.

## Change set (at most 5)

1. UXF-002, build: CSS keeps the theme toggle in the navbar below 997 px. Target: `always_visible_defects@390x844` 2 to 1, `always_visible_violations@390x844` 144 to 72.
2. UXF-003, build: underline links inside running text. Target: `axe_serious_critical_nodes@1280x800` 330 to about 44, `@390x844` 324 to about 44 (the 44 are UXF-004).
3. UXF-006, build: code block text at 14 px. Target: the new regression assertion; the measure-only 14 px page count stays until breadcrumbs and table of contents are decided.
4. UXF-001, decision only: `decision.md` (R-01). Nothing built. Waits for the yes or no; blocks nothing.
5. UXF-005, waits: needs a layout decision and the round's one decision slot is spent on UXF-001. First in line for R-02. Blocks nothing.

Left out by the cap of 5, in priority order: UXF-104, UXF-004, UXF-105. UXF-004 has its failing test written in `ux/regression/pending/UXF-004.test.mjs` and goes first into R-02 once the cap allows.

Findings closed by this round: UXF-002, UXF-003, UXF-006 (and their duplicates UXF-102, UXF-107), when the gate and their regression assets pass.

Standing story: the repository holds an AEP planning store (`.engineering/`); this round did not create the standing story or its task, so the change set above is the record until it does.
