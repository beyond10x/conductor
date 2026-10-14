# Wave 37 (the operator's surface first)

Coordinator: conductor-dev. Approved up front (rules item 99). Integration branch `wave/37`. Target:
v0.15.0. Source: the operator, 2026-10-14 ~23:0xZ: the dashboard is slow and he cannot decide on
one page; measured on the live dashboard: `/api/board` 18.2 s, `/api/health` 15.7 s,
`/api/repositories` 26.0 s. Re-ordered that night: the dashboard and decisions stories come first,
the rest of session confinement moves to the next wave.

## Units

| unit | story | note |
|---|---|---|
| U1 | sandbox-spec | done |
| U2 | session-envelope-seam | done |
| U3 | session-limits-systemd | in work; live run on this host by the coordinator |
| U7 | decisions-notify | in work, beside U3 |
| U8 | dashboard-fast | in work, own tree `wave/37-fast` |
| U9 | decisions-page | after U7 (both change `spec/domains/decision.yaml`) |
| U10 | decisions-batch-answer | after U9; adversarial review of its write path |

Moved to wave 38: session-writes-bwrap, session-forge-credentials, confined-tutorial.

## Notes

## Outcome
