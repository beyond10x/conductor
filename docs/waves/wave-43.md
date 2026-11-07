# Wave 43 (the loop keeps its daily tick)

Three stories that were draft and serve instances whose daily tick did not commit or did not run.
One tree, one release. Target: v0.20.2.

## Units

| unit | story | note |
|---|---|---|
| U1 | records-check-legacy-months | a month file before the first one a command wrote is not judged; `UnknownDecision` reads no id out of `line` |
| U2 | first-cycle-not-blocked-by-archiving | `gc` on a `disk low` line runs beside the loop, not inside the watch tick; archiving prints `archive n of N` |
| U3 | daily-tick-runs | `daily.last` is written once the daily tick ended, so a loop stopped during it runs it again at start |
| U4 | dashboard-instance-switch | a header selector lists the host registry's instances and opens the chosen one's own dashboard; each server still answers for its own instance |

## Metric

Daily ticks that end with the records committed, per instance and day (`conductor session
run-ticks`, kind `daily`, action `records-committed`).

| | before (2026-11-07) | after |
|---|---|---|
| instance A | 1 daily tick, action `records-commit-refused` | |
| instance B | 0 daily ticks: the loop waited in `worktree gc` from 06:49Z past 09:00Z | |
