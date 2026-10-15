# Wave 38 (every metric, one vocabulary; the review's P2s)

Coordinator: conductor-dev. Approved up front (rules item 99). Target: v0.16.0, after v0.15.0.

Sources:
- the operator, 2026-10-14: a good UI with every metric, and "metric vs KPI" named right;
- the external review of 0.14.0: its eight P2 defects. Its three P1s ship in v0.15.0.

Started early during wave 37: kpi-records, which is paused, and conductor-context-budget, which
merged into wave 37.

## Units, in order

| unit | story | note |
|---|---|---|
| U1 | review-p2-0-14 | the review's eight P2 defects, each with its probe as the failing test |
| U2 | kpi-records | resume from `7daa31db`: clippy, docs-check, then K6 and K12 |
| U3 | reserve-from-peak | the disk ledger reserves a command's recorded peak, not a flat 8G |
| U4 | build-run-session-bound | a stopped session's queued build requests are withdrawn |
| U5 | snapshot-unlisted-repository | a repository missing from the workspace is a row, not a failed snapshot |
| U6 | kpi-transcripts, kpi-per-task | the metrics read from transcripts and per task |
| U7 | kpi-board, health-kpis, instance-usage-metrics | the metrics in the board and health |
| U8 | metrics-naming | one Metrics section in the dashboard: KPIs first, then every other metric |
| U9 | kpi-verify | every metric of the design's table is measured or names its story |
| U10 | need-id-unique, slice-remove | small fixes from conductor's NEEDs |

Then confinement: session-writes-bwrap, session-forge-credentials, confined-tutorial.

## Notes

## Outcome
